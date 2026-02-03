use crate::machine_identification::{MachineIdentification, MachineIdentificationUnique};
use crate::bbm_automatik_v2::api::{
    LiveValuesEvent, BbmAutomatikV2Events, StateEvent,
};
use crate::{AsyncThreadMessage, Machine, MachineMessage, BBM_AUTOMATIK_V2, VENDOR_QITECH};
use control_core::socketio::namespace::NamespaceCacheingLogic;
use ethercat_hal::io::digital_input::DigitalInput;
use ethercat_hal::io::digital_output::DigitalOutput;
use ethercat_hal::io::pulse_train_output::PulseTrainOutput;
use smol::channel::{Receiver, Sender};
use std::time::Instant;

pub mod act;
pub mod api;
pub mod new;

use crate::bbm_automatik_v2::api::BbmAutomatikV2Namespace;

/// Device Roles for BbmAutomatikV2
/// Hardware: 1x EL2522 (2 Achsen), EL1008, EL2008
/// TODO: Später 2x EL2522 für 4 Achsen
pub mod roles {
    pub const DIGITAL_INPUT: u16 = 1;  // EL1008 - 8x DI (3x Referenzschalter, 2x Türsensoren)
    pub const DIGITAL_OUTPUT: u16 = 2; // EL2008 - 8x DO (1x Rüttelmotor, 3x Ampel)
    pub const PTO_1: u16 = 3;          // EL2522 #1 - Kanal 1: MT, Kanal 2: Schieber
    // pub const PTO_2: u16 = 4;       // EL2522 #2 - Kanal 1: Drücker, Kanal 2: Bürste (TODO: später)
}

/// Axis indices
pub mod axes {
    pub const MT: usize = 0;         // Magazin Transporter (Linear)
    pub const SCHIEBER: usize = 1;   // Schieber (Linear)
    pub const DRUECKER: usize = 2;   // Drücker (Linear)
    pub const BUERSTE: usize = 3;    // Bürste (Rotation)
}

/// Digital input indices
pub mod inputs {
    pub const REF_MT: usize = 0;        // Referenzschalter MT
    pub const REF_SCHIEBER: usize = 1;  // Referenzschalter Schieber
    pub const REF_DRUECKER: usize = 2;  // Referenzschalter Drücker
    pub const TUER_1: usize = 3;        // Türsensor 1
    pub const TUER_2: usize = 4;        // Türsensor 2
}

/// Digital output indices
pub mod outputs {
    pub const RUETTELMOTOR: usize = 0;  // Rüttelmotor
    pub const AMPEL_ROT: usize = 1;     // Ampel Rot
    pub const AMPEL_GELB: usize = 2;    // Ampel Gelb
    pub const AMPEL_GRUEN: usize = 3;   // Ampel Grün
}

/// Mechanical constants for the linear axes
pub mod mechanics {
    /// Motor pulses per revolution (default stepper setting)
    pub const PULSES_PER_REV: u32 = 200;
    /// Ball screw lead in mm per revolution
    pub const LEAD_MM: f32 = 10.0;
    /// Calculated pulses per mm
    pub const PULSES_PER_MM: f32 = PULSES_PER_REV as f32 / LEAD_MM; // = 20.0

    /// Convert mm/s to frequency (Hz)
    pub fn mm_per_s_to_hz(mm_per_s: f32) -> i32 {
        (mm_per_s * PULSES_PER_MM) as i32
    }

    /// Convert frequency (Hz) to mm/s
    pub fn hz_to_mm_per_s(hz: i32) -> f32 {
        hz as f32 / PULSES_PER_MM
    }

    /// Convert position (pulses) to mm
    pub fn pulses_to_mm(pulses: u32) -> f32 {
        pulses as f32 / PULSES_PER_MM
    }
}

#[derive(Debug)]
pub struct BbmAutomatikV2 {
    pub api_receiver: Receiver<MachineMessage>,
    pub api_sender: Sender<MachineMessage>,
    pub machine_identification_unique: MachineIdentificationUnique,
    pub namespace: BbmAutomatikV2Namespace,
    pub last_state_emit: Instant,
    pub main_sender: Option<Sender<AsyncThreadMessage>>,

    // Digital Inputs (1x EL1008 = 8 inputs)
    pub digital_inputs: [DigitalInput; 8],

    // Digital Outputs (1x EL2008 = 8 outputs)
    pub digital_outputs: [DigitalOutput; 8],
    pub output_states: [bool; 8],

    // Pulse Train Outputs (2x EL2522 = 4 channels)
    // Axis 0: MT (EL2522 #1, Ch1)
    // Axis 1: Schieber (EL2522 #1, Ch2)
    // Axis 2: Drücker (EL2522 #2, Ch1)
    // Axis 3: Bürste (EL2522 #2, Ch2)
    pub axes: [PulseTrainOutput; 4],
    pub axis_speeds: [i32; 4],           // Current speed (Hz) - used by software ramp
    pub axis_target_speeds: [i32; 4],    // Target speed (Hz) - what we want to reach
    pub axis_accelerations: [f32; 4],    // Acceleration in mm/s² per axis
    pub axis_target_positions: [u32; 4], // Target position in pulses for position mode
    pub axis_position_mode: [bool; 4],   // True if axis is in position mode (auto-stop at target)
    pub last_ramp_update: Instant,       // For software ramp timing
}

impl Machine for BbmAutomatikV2 {
    fn get_machine_identification_unique(&self) -> MachineIdentificationUnique {
        self.machine_identification_unique.clone()
    }

    fn get_main_sender(&self) -> Option<Sender<AsyncThreadMessage>> {
        self.main_sender.clone()
    }
}

impl BbmAutomatikV2 {
    pub const MACHINE_IDENTIFICATION: MachineIdentification = MachineIdentification {
        vendor: VENDOR_QITECH,
        machine: BBM_AUTOMATIK_V2,
    };

    /// Get current state for UI
    pub fn get_state(&self) -> StateEvent {
        StateEvent {
            output_states: self.output_states,
            axis_speeds: self.axis_speeds,
            axis_target_speeds: self.axis_target_speeds,
            axis_accelerations: self.axis_accelerations,
            axis_target_positions: self.axis_target_positions,
            axis_position_mode: self.axis_position_mode,
        }
    }

    /// Get live values (sensor readings, positions)
    pub fn get_live_values(&self) -> LiveValuesEvent {
        // Read digital inputs
        let mut input_states = [false; 8];
        for (i, di) in self.digital_inputs.iter().enumerate() {
            input_states[i] = di.get_value().unwrap_or(false);
        }

        // Read axis positions from PTO feedback
        let mut positions = [0u32; 4];
        for (i, axis) in self.axes.iter().enumerate() {
            positions[i] = axis.get_position();
        }

        LiveValuesEvent {
            input_states,
            axis_positions: positions,
        }
    }

    /// Emit state event to UI
    pub fn emit_state(&mut self) {
        let event = self.get_state().build();
        self.namespace.emit(BbmAutomatikV2Events::State(event));
    }

    /// Emit live values to UI
    pub fn emit_live_values(&mut self) {
        let event = self.get_live_values().build();
        self.namespace
            .emit(BbmAutomatikV2Events::LiveValues(event));
    }

    /// Set a digital output
    pub fn set_output(&mut self, index: usize, on: bool) {
        if index < self.output_states.len() {
            self.output_states[index] = on;
            self.digital_outputs[index].set(on);
            self.emit_state();
        }
    }

    /// Set all digital outputs
    pub fn set_all_outputs(&mut self, on: bool) {
        for i in 0..self.output_states.len() {
            self.output_states[i] = on;
            self.digital_outputs[i].set(on);
        }
        self.emit_state();
    }

    /// Set axis speed (frequency value for PTO)
    pub fn set_axis_speed(&mut self, index: usize, speed: i32) {
        if index < self.axis_speeds.len() {
            self.axis_speeds[index] = speed;
            self.axes[index].set_frequency(speed);
            self.emit_state();
        }
    }

    /// Stop all axes (set speed to 0)
    pub fn stop_all_axes(&mut self) {
        for i in 0..self.axis_speeds.len() {
            self.axis_speeds[i] = 0;
            self.axis_target_speeds[i] = 0;
            self.axes[i].set_frequency(0);
        }
        self.emit_state();
    }

    /// Stop single axis
    pub fn stop_axis(&mut self, index: usize) {
        if index < self.axis_speeds.len() {
            self.axis_speeds[index] = 0;
            self.axis_target_speeds[index] = 0;
            self.axis_position_mode[index] = false;
            self.axes[index].set_frequency(0);
            self.emit_state();
        }
    }

    // ============ Speed/Acceleration Functions ============

    /// Set target axis speed in mm/s (software ramp will handle transition)
    /// Positive = forward, Negative = backward
    pub fn set_axis_speed_mm_s(&mut self, index: usize, mm_per_s: f32) {
        if index < self.axis_target_speeds.len() {
            let hz = mechanics::mm_per_s_to_hz(mm_per_s);
            self.axis_target_speeds[index] = hz;
            self.emit_state();
            tracing::info!(
                "[BbmAutomatikV2] Axis {} target speed set: {:.1} mm/s = {} Hz (accel: {:.1} mm/s²)",
                index,
                mm_per_s,
                hz,
                self.axis_accelerations[index]
            );
        }
    }

    /// Set axis acceleration in mm/s²
    pub fn set_axis_acceleration(&mut self, index: usize, accel_mm_s2: f32) {
        if index < self.axis_accelerations.len() {
            // Clamp acceleration to reasonable range (1-500 mm/s²)
            let clamped = accel_mm_s2.clamp(1.0, 500.0);
            self.axis_accelerations[index] = clamped;
            self.emit_state();
            tracing::info!(
                "[BbmAutomatikV2] Axis {} acceleration set: {:.1} mm/s²",
                index,
                clamped
            );
        }
    }

    /// Move to a target position in mm
    /// This starts the motor and auto-stops when position is reached
    pub fn move_to_position_mm(&mut self, index: usize, position_mm: f32, speed_mm_s: f32) {
        if index < self.axes.len() {
            let target_pulses = (position_mm * mechanics::PULSES_PER_MM) as u32;
            let current_pulses = self.axes[index].get_position();

            // Determine direction based on current vs target position
            let speed_hz = if target_pulses > current_pulses {
                mechanics::mm_per_s_to_hz(speed_mm_s.abs())
            } else {
                mechanics::mm_per_s_to_hz(-speed_mm_s.abs())
            };

            // Set position mode
            self.axis_target_positions[index] = target_pulses;
            self.axis_position_mode[index] = true;

            // Set target speed (software ramp will handle acceleration)
            self.axis_target_speeds[index] = speed_hz;

            // Set target counter value in hardware for auto-stop
            let mut output = self.axes[index].get_output();
            output.target_counter_value = target_pulses;
            self.axes[index].set_output(output);

            self.emit_state();
            tracing::info!(
                "[BbmAutomatikV2] Axis {} moving to {:.1} mm ({} pulses) at {:.1} mm/s",
                index,
                position_mm,
                target_pulses,
                speed_mm_s
            );
        }
    }

    /// Software ramp: update axis_speeds towards target_speeds based on acceleration
    /// Called from act() loop at ~30Hz
    /// Returns true if any speed changed (for state emission)
    pub fn update_software_ramp(&mut self, dt_secs: f32) -> bool {
        let mut changed = false;
        for i in 0..self.axis_speeds.len() {
            // Check if we're in position mode and reached target
            if self.axis_position_mode[i] {
                let current_pos = self.axes[i].get_position();
                let target_pos = self.axis_target_positions[i];
                let moving_forward = self.axis_target_speeds[i] > 0;

                // Check if we've reached or passed the target
                let reached = if moving_forward {
                    current_pos >= target_pos
                } else {
                    current_pos <= target_pos
                };

                if reached {
                    // Stop the motor
                    self.axis_target_speeds[i] = 0;
                    self.axis_position_mode[i] = false;
                    tracing::info!(
                        "[BbmAutomatikV2] Axis {} reached target position {} pulses",
                        i,
                        target_pos
                    );
                }
            }

            let current = self.axis_speeds[i];
            let target = self.axis_target_speeds[i];

            if current != target {
                // Convert acceleration from mm/s² to Hz/s
                let accel_hz_per_s = self.axis_accelerations[i] * mechanics::PULSES_PER_MM;
                let delta_hz = ((accel_hz_per_s * dt_secs) as i32).max(1); // At least 1 Hz step

                // Move towards target
                let new_speed = if current < target {
                    // Accelerating
                    (current + delta_hz).min(target)
                } else {
                    // Decelerating
                    (current - delta_hz).max(target)
                };

                // Apply new speed to hardware
                self.axis_speeds[i] = new_speed;
                self.axes[i].set_frequency(new_speed);
                changed = true;
            }
        }
        changed
    }

    // ============ Convenience Functions ============

    /// Set Rüttelmotor on/off
    pub fn set_ruettelmotor(&mut self, on: bool) {
        self.set_output(outputs::RUETTELMOTOR, on);
    }

    /// Set Ampel state
    pub fn set_ampel(&mut self, rot: bool, gelb: bool, gruen: bool) {
        self.output_states[outputs::AMPEL_ROT] = rot;
        self.output_states[outputs::AMPEL_GELB] = gelb;
        self.output_states[outputs::AMPEL_GRUEN] = gruen;
        self.digital_outputs[outputs::AMPEL_ROT].set(rot);
        self.digital_outputs[outputs::AMPEL_GELB].set(gelb);
        self.digital_outputs[outputs::AMPEL_GRUEN].set(gruen);
        self.emit_state();
    }

    /// Check if door sensors indicate safe (both closed)
    pub fn are_doors_closed(&self) -> bool {
        let input1 = self.digital_inputs[inputs::TUER_1].get_value().unwrap_or(false);
        let input2 = self.digital_inputs[inputs::TUER_2].get_value().unwrap_or(false);
        // Assuming normally closed sensors (true = door closed)
        input1 && input2
    }

    /// Check if axis is at home position (reference switch active)
    pub fn is_axis_homed(&self, axis: usize) -> bool {
        match axis {
            axes::MT => self.digital_inputs[inputs::REF_MT].get_value().unwrap_or(false),
            axes::SCHIEBER => self.digital_inputs[inputs::REF_SCHIEBER].get_value().unwrap_or(false),
            axes::DRUECKER => self.digital_inputs[inputs::REF_DRUECKER].get_value().unwrap_or(false),
            _ => false, // Bürste has no home switch
        }
    }
}
