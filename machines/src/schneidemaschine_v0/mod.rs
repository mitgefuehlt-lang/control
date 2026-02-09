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
    pub const DIGITAL_INPUT: u16 = 1; // EL1008
    pub const DIGITAL_OUTPUT: u16 = 2; // EL2008
    pub const PTO: u16 = 3; // EL2522
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
    pub axis_speeds: [i32; 2],
    pub axis_target_speeds: [i32; 2],
    pub axis_accelerations: [f32; 2],
    pub axis_target_positions: [i32; 2],
    pub axis_position_mode: [bool; 2],

    // Hardware ramp control
    pub sdo_write_u16: Option<crate::SdoWriteU16Fn>,
    pub pto_subdevice_index: usize,
}

impl std::fmt::Debug for SchneidemaschineV0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SchneidemaschineV0")
    }
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

    /// Stop all axes - hardware immediate stop
    pub fn stop_all_axes(&mut self) {
        for i in 0..self.axis_speeds.len() {
            self.axis_speeds[i] = 0;
            self.axis_target_speeds[i] = 0;
            self.axis_position_mode[i] = false;

            let mut output = self.axes[i].get_output();
            output.disble_ramp = true;
            output.go_counter = false;
            output.frequency_value = 0;
            self.axes[i].set_output(output);
        }
        self.emit_state();
    }

    /// Stop single axis - hardware immediate stop
    pub fn stop_axis(&mut self, index: usize) {
        if index < self.axis_speeds.len() {
            self.axis_speeds[index] = 0;
            self.axis_target_speeds[index] = 0;
            self.axis_position_mode[index] = false;

            let mut output = self.axes[index].get_output();
            output.disble_ramp = true;
            output.go_counter = false;
            output.frequency_value = 0;
            self.axes[index].set_output(output);

            self.emit_state();
        }
    }

    // ============ Speed/Acceleration Functions ============

    /// Set target axis speed in mm/s (hardware ramp handles transition)
    /// Positive = forward, Negative = backward
    pub fn set_axis_speed_mm_s(&mut self, index: usize, mm_per_s: f32) {
        if index < self.axis_target_speeds.len() {
            let hz = mechanics::mm_per_s_to_hz(mm_per_s);
            self.axis_target_speeds[index] = hz;
            // Hardware ramp accelerates/brakes automatically to target
            self.emit_state();
        }
    }

    /// Set axis acceleration in mm/s² - writes ramp time constants via SDO
    pub fn set_axis_acceleration(&mut self, index: usize, accel_mm_s2: f32) {
        if index < self.axis_accelerations.len() {
            let clamped = accel_mm_s2.clamp(4.0, 500.0);
            self.axis_accelerations[index] = clamped;

            // Calculate ramp time constants for hardware
            let accel_hz_s = clamped * mechanics::PULSES_PER_MM;
            let base_freq = 5000.0_f32;
            let rising_ms = ((base_freq / accel_hz_s) * 1000.0) as u16;
            let falling_ms = rising_ms; // Same as rising to avoid step loss from aggressive braking

            // SDO-Write to EL2522
            if let Some(sdo_write) = &self.sdo_write_u16 {
                let subdevice_index = self.pto_subdevice_index;
                // PTO Base Index: Channel 1 = 0x8000, Channel 2 = 0x8010
                let pto_base = if index == 0 { 0x8000u16 } else { 0x8010u16 };

                sdo_write(subdevice_index, pto_base, 0x14, rising_ms);
                sdo_write(subdevice_index, pto_base, 0x15, falling_ms);
            }
            self.emit_state();
        }
    }

    /// Move to a target position in mm using hardware Travel Distance Control
    pub fn move_to_position_mm(&mut self, index: usize, position_mm: f32, speed_mm_s: f32) {
        if index < self.axes.len() {
            let target_pulses = (position_mm.round() * mechanics::PULSES_PER_MM) as i32;
            let speed_hz = mechanics::mm_per_s_to_hz(speed_mm_s.abs());

            let current_pulses = self.axes[index].get_position() as i32;
            let direction = if target_pulses > current_pulses {
                1
            } else {
                -1
            };

            self.axis_target_positions[index] = target_pulses;
            self.axis_position_mode[index] = true;

            let mut output = self.axes[index].get_output();
            output.go_counter = true;
            output.disble_ramp = false;
            output.frequency_value = speed_hz * direction;
            output.target_counter_value = target_pulses as u32;
            self.axes[index].set_output(output);

            self.axis_target_speeds[index] = speed_hz * direction;
            self.axis_speeds[index] = speed_hz * direction;

            self.emit_state();
            tracing::info!(
                "[SchneidemaschineV0] Axis {} moving to {:.0} mm ({} pulses) at {:.1} mm/s",
                index,
                position_mm.round(),
                target_pulses,
                speed_mm_s
            );
        }
    }

    /// Hardware ramp monitor: watches hardware status, does not set speeds
    pub fn update_hardware_monitor(&mut self) -> bool {
        let mut changed = false;
        for i in 0..self.axis_speeds.len() {
            let input = self.axes[i].get_input();

            // Position mode: target detection
            if self.axis_position_mode[i] {
                if input.select_end_counter {
                    self.axis_speeds[i] = 0;
                    self.axis_target_speeds[i] = 0;
                    self.axis_position_mode[i] = false;

                    let mut output = self.axes[i].get_output();
                    output.go_counter = false;
                    output.frequency_value = 0;
                    self.axes[i].set_output(output);

                    changed = true;
                    let actual_pos = self.axes[i].get_position() as i32;
                    let target_pos = self.axis_target_positions[i];
                    let deviation = (actual_pos - target_pos).abs();
                    if deviation > 2 {
                        tracing::warn!(
                            "[Axis {}] STEP LOSS DETECTED: target={} actual={} deviation={} pulses ({:.2} mm)",
                            i, target_pos, actual_pos, deviation,
                            deviation as f32 / mechanics::PULSES_PER_MM
                        );
                    } else {
                        tracing::info!(
                            "[Axis {}] Target reached: {} pulses (actual: {}, deviation: {})",
                            i, target_pos, actual_pos, deviation
                        );
                    }
                }
            }

            // JOG mode: send speed directly to hardware
            if !self.axis_position_mode[i] {
                let target = self.axis_target_speeds[i];
                if self.axis_speeds[i] != target {
                    let mut output = self.axes[i].get_output();
                    output.disble_ramp = false;
                    output.go_counter = false;
                    output.frequency_value = target;
                    self.axes[i].set_output(output);
                    self.axis_speeds[i] = target;
                    changed = true;
                }
            }

            if input.ramp_active {
                changed = true;
            }
        }
        changed
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
        self.namespace
            .emit(SchneidemaschineV0Events::DebugPto(event));
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
