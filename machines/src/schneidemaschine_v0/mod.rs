use crate::machine_identification::{MachineIdentification, MachineIdentificationUnique};
use crate::schneidemaschine_v0::api::{
    DebugPtoEvent, LiveValuesEvent, SchneidemaschineV0Events, StateEvent,
};
use crate::{AsyncThreadMessage, Machine, MachineMessage, SCHNEIDEMASCHINE_V0, VENDOR_QITECH};
use control_core::socketio::namespace::NamespaceCacheingLogic;
use ethercat_hal::io::digital_input::DigitalInput;
use ethercat_hal::io::digital_output::DigitalOutput;
use ethercat_hal::io::pulse_train_output::PulseTrainOutput;
use smol::channel::{Receiver, Sender};
use std::time::Instant;

pub mod act;
pub mod api;
pub mod new;

use crate::schneidemaschine_v0::api::SchneidemaschineV0Namespace;

/// Device Roles for SchneidemaschineV0
pub mod roles {
    pub const DIGITAL_INPUT: u16 = 1;  // EL1008
    pub const DIGITAL_OUTPUT: u16 = 2; // EL2008
    pub const PTO: u16 = 3;            // EL2522
}

/// Mechanical constants for the linear axis
pub mod mechanics {
    /// Motor pulses per revolution (CL57T setting)
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
pub struct SchneidemaschineV0 {
    pub api_receiver: Receiver<MachineMessage>,
    pub api_sender: Sender<MachineMessage>,
    pub machine_identification_unique: MachineIdentificationUnique,
    pub namespace: SchneidemaschineV0Namespace,
    pub last_state_emit: Instant,
    pub main_sender: Option<Sender<AsyncThreadMessage>>,

    // Digital Inputs (1x EL1008 = 8 inputs)
    pub digital_inputs: [DigitalInput; 8],

    // Digital Outputs (1x EL2008 = 8 outputs)
    pub digital_outputs: [DigitalOutput; 8],
    pub output_states: [bool; 8],

    // Pulse Train Outputs (1x EL2522 = 2 channels)
    pub axes: [PulseTrainOutput; 2],
    pub axis_speeds: [i32; 2],           // Current speed (Hz) - used by software ramp
    pub axis_target_speeds: [i32; 2],    // Target speed (Hz) - what we want to reach
    pub axis_accelerations: [f32; 2],    // Acceleration in mm/s² per axis
    pub last_ramp_update: Instant,       // For software ramp timing
}

impl Machine for SchneidemaschineV0 {
    fn get_machine_identification_unique(&self) -> MachineIdentificationUnique {
        self.machine_identification_unique.clone()
    }

    fn get_main_sender(&self) -> Option<Sender<AsyncThreadMessage>> {
        self.main_sender.clone()
    }
}

impl SchneidemaschineV0 {
    pub const MACHINE_IDENTIFICATION: MachineIdentification = MachineIdentification {
        vendor: VENDOR_QITECH,
        machine: SCHNEIDEMASCHINE_V0,
    };

    /// Get current state for UI
    pub fn get_state(&self) -> StateEvent {
        StateEvent {
            output_states: self.output_states,
            axis_speeds: self.axis_speeds,
            axis_target_speeds: self.axis_target_speeds,
            axis_accelerations: self.axis_accelerations,
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
        let mut positions = [0u32; 2];
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
        self.namespace.emit(SchneidemaschineV0Events::State(event));
    }

    /// Emit live values to UI
    pub fn emit_live_values(&mut self) {
        let event = self.get_live_values().build();
        self.namespace
            .emit(SchneidemaschineV0Events::LiveValues(event));
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
            self.axes[i].set_frequency(0);
        }
        self.emit_state();
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
                "[SchneidemaschineV0] Axis {} target speed set: {:.1} mm/s = {} Hz (accel: {:.1} mm/s²)",
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
                "[SchneidemaschineV0] Axis {} acceleration set: {:.1} mm/s²",
                index,
                clamped
            );
        }
    }

    /// Software ramp: update axis_speeds towards target_speeds based on acceleration
    /// Called from act() loop at ~30Hz
    pub fn update_software_ramp(&mut self, dt_secs: f32) {
        for i in 0..self.axis_speeds.len() {
            let current = self.axis_speeds[i];
            let target = self.axis_target_speeds[i];

            if current != target {
                // Convert acceleration from mm/s² to Hz/s
                let accel_hz_per_s = self.axis_accelerations[i] * mechanics::PULSES_PER_MM;
                let delta_hz = (accel_hz_per_s * dt_secs) as i32;

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
            }
        }
    }

    // ============ Debug Functions ============

    /// Get comprehensive debug info for PTO channel
    pub fn get_debug_pto(&self, index: usize) -> DebugPtoEvent {
        if index >= self.axes.len() {
            return DebugPtoEvent::default();
        }

        let axis = &self.axes[index];
        let input = axis.get_input();
        let output = axis.get_output();

        DebugPtoEvent {
            channel: index as u8,
            // Output (what we're sending)
            frequency_setpoint_hz: output.frequency_value,
            frequency_setpoint_mm_s: mechanics::hz_to_mm_per_s(output.frequency_value),
            target_position_pulses: output.target_counter_value,
            target_position_mm: mechanics::pulses_to_mm(output.target_counter_value),
            disable_ramp: output.disble_ramp,
            set_counter_request: output.set_counter,
            set_counter_value: output.set_counter_value,
            // Input (feedback from device)
            actual_position_pulses: input.counter_value,
            actual_position_mm: mechanics::pulses_to_mm(input.counter_value),
            ramp_active: input.ramp_active,
            error: input.error,
            sync_error: input.sync_error,
            counter_overflow: input.counter_overflow,
            counter_underflow: input.counter_underflow,
            set_counter_done: input.set_counter_done,
            input_t: input.input_t,
            input_z: input.input_z,
            select_end_counter: input.select_end_counter,
        }
    }

    /// Emit debug event for PTO channel
    pub fn emit_debug_pto(&mut self, index: usize) {
        let debug = self.get_debug_pto(index);
        let event = debug.build();
        self.namespace.emit(SchneidemaschineV0Events::DebugPto(event));
    }

    /// Log all debug info to console
    pub fn log_debug_all(&self) {
        tracing::info!("========== SchneidemaschineV0 Debug ==========");

        // Digital Inputs
        let mut input_str = String::from("DI: ");
        for (i, di) in self.digital_inputs.iter().enumerate() {
            let val = di.get_value().unwrap_or(false);
            input_str.push_str(&format!("{}={} ", i + 1, if val { "1" } else { "0" }));
        }
        tracing::info!("{}", input_str);

        // Digital Outputs
        let mut output_str = String::from("DO: ");
        for (i, state) in self.output_states.iter().enumerate() {
            output_str.push_str(&format!("{}={} ", i + 1, if *state { "1" } else { "0" }));
        }
        tracing::info!("{}", output_str);

        // PTO Channels
        for i in 0..2 {
            let pto_info = self.get_debug_pto(i);
            tracing::info!(
                "PTO{}: freq={}Hz ({:.1}mm/s) pos={}p ({:.2}mm) ramp={} err={}",
                i + 1,
                pto_info.frequency_setpoint_hz,
                pto_info.frequency_setpoint_mm_s,
                pto_info.actual_position_pulses,
                pto_info.actual_position_mm,
                pto_info.ramp_active,
                pto_info.error
            );
        }
        tracing::info!("===============================================");
    }
}
