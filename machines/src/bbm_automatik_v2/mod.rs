use crate::bbm_automatik_v2::api::{BbmAutomatikV2Events, LiveValuesEvent, StateEvent};
use crate::machine_identification::{MachineIdentification, MachineIdentificationUnique};
use crate::{AsyncThreadMessage, BBM_AUTOMATIK_V2, Machine, MachineMessage, VENDOR_QITECH};
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
/// Hardware: 2x EL2522 (4 Achsen), EL1008, EL2008
pub mod roles {
    pub const DIGITAL_INPUT: u16 = 1; // EL1008 - 8x DI (3x Referenzschalter, 2x Türsensoren)
    pub const DIGITAL_OUTPUT: u16 = 2; // EL2008 - 8x DO (1x Rüttelmotor, 3x Ampel)
    pub const PTO_1: u16 = 3; // EL2522 #1 - Kanal 1: MT, Kanal 2: Schieber
    pub const PTO_2: u16 = 4; // EL2522 #2 - Kanal 1: Drücker, Kanal 2: Bürste
}

/// Axis indices
pub mod axes {
    pub const MT: usize = 0; // Magazin Transporter (Linear)
    pub const SCHIEBER: usize = 1; // Schieber (Linear)
    pub const DRUECKER: usize = 2; // Drücker (Linear)
    pub const BUERSTE: usize = 3; // Bürste (Rotation)
}

/// Digital input indices (0-based array index, DI1 = index 0)
pub mod inputs {
    pub const REF_MT: usize = 0;        // Referenzschalter Transporter (DI1 = index 0)
    pub const REF_SCHIEBER: usize = 1;  // Referenzschalter Schieber (DI2 = index 1)
    pub const REF_DRUECKER: usize = 2;  // Referenzschalter Drücker (DI3 = index 2)
    pub const ALARM_MT: usize = 3;       // CL75t Alarm Transporter (DI4 = index 3)
    pub const ALARM_SCHIEBER: usize = 4; // CL75t Alarm Schieber (DI5 = index 4)
    pub const ALARM_DRUECKER: usize = 5; // CL75t Alarm Drücker (DI6 = index 5)
    pub const TUER: usize = 6;           // Türsensor (DI7 = index 6)
}

/// Digital output indices
pub mod outputs {
    pub const RUETTELMOTOR: usize = 0; // Rüttelmotor
    pub const AMPEL_ROT: usize = 1; // Ampel Rot
    pub const AMPEL_GELB: usize = 2; // Ampel Gelb
    pub const AMPEL_GRUEN: usize = 3; // Ampel Grün
}

/// Soft limits per axis in mm (0 = home position after homing)
/// Values from Arduino BBMx22_Automatik_Code.ino v3.2
pub mod soft_limits {
    pub const MT_MAX_MM: f32 = 230.0;
    pub const SCHIEBER_MAX_MM: f32 = 53.0;
    pub const DRUECKER_MAX_MM: f32 = 107.0;
    pub const MIN_MM: f32 = 0.0;

    /// Get max position for axis in mm (None = no limit, e.g. rotation)
    pub fn max_position_mm(axis: usize) -> Option<f32> {
        match axis {
            super::axes::MT => Some(MT_MAX_MM),
            super::axes::SCHIEBER => Some(SCHIEBER_MAX_MM),
            super::axes::DRUECKER => Some(DRUECKER_MAX_MM),
            _ => None, // Bürste = rotation, no limit
        }
    }
}

/// Homing configuration
pub mod homing {
    /// Homing speed in mm/s (slow for precision)
    pub const HOMING_SPEED_MM_S: f32 = 15.0;
    /// Retract distance after hitting sensor (mm)
    pub const RETRACT_DISTANCE_MM: f32 = 2.0;
}

/// Homing phases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HomingPhase {
    /// Not homing
    Idle,
    /// Phase 1: Moving negative until sensor triggers
    SearchingSensor,
    /// Phase 2: Retracting 2mm away from sensor
    Retracting,
    /// Phase 3: Setting position to 0
    SettingZero,
}

/// Mechanical constants for the linear axes
pub mod mechanics {
    /// Motor pulses per revolution (default stepper setting)
    pub const PULSES_PER_REV: u32 = 200;
    /// Ball screw lead in mm per revolution
    pub const LEAD_MM: f32 = 10.0;
    /// Calculated pulses per mm
    pub const PULSES_PER_MM: f32 = PULSES_PER_REV as f32 / LEAD_MM; // = 20.0

    /// Convert mm/s to frequency (Hz) - for linear axes with ball screw
    pub fn mm_per_s_to_hz(mm_per_s: f32) -> i32 {
        (mm_per_s * PULSES_PER_MM) as i32
    }

    /// Convert frequency (Hz) to mm/s - for linear axes
    pub fn hz_to_mm_per_s(hz: i32) -> f32 {
        hz as f32 / PULSES_PER_MM
    }

    /// Convert position (pulses) to mm
    pub fn pulses_to_mm(pulses: u32) -> f32 {
        pulses as f32 / PULSES_PER_MM
    }

    /// Convert RPM to frequency (Hz) - for rotation axes (no ball screw)
    /// RPM * 200 pulses/rev / 60 sec/min = Hz
    pub fn rpm_to_hz(rpm: f32) -> i32 {
        (rpm * PULSES_PER_REV as f32 / 60.0) as i32
    }

    /// Convert frequency (Hz) to RPM - for rotation axes
    pub fn hz_to_rpm(hz: i32) -> f32 {
        hz as f32 * 60.0 / PULSES_PER_REV as f32
    }
}

/// Alarm polarity: CL75t open-collector = active LOW (false = alarm)
const ALARM_ACTIVE_LOW: bool = true;

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
    pub axis_speeds: [i32; 4],
    pub axis_target_speeds: [i32; 4],
    pub axis_accelerations: [f32; 4],
    pub axis_target_positions: [i32; 4],
    pub axis_position_mode: [bool; 4],
    /// Ignore select_end_counter for N cycles after starting a new move
    /// (hardware needs time to process go_counter and clear the old signal)
    pub axis_position_ignore_cycles: [u8; 4],

    // Hardware ramp control
    pub sdo_write_u16: Option<crate::SdoWriteU16Fn>,
    pub pto_subdevice_indices: [usize; 2],

    // Homing state
    pub axis_homing_phase: [HomingPhase; 4],
    pub axis_homing_retract_target: [i32; 4],

    // Driver alarm state (CL75t alarm pins)
    /// true = alarm active (axis stopped), per axis
    pub axis_alarm_active: [bool; 4],

    // Debug logging
    pub last_debug_log: Option<Instant>,
}

impl std::fmt::Debug for BbmAutomatikV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BbmAutomatikV2")
    }
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
        // Convert homing phase to bool for UI (true = any homing phase active)
        let homing_active = [
            self.axis_homing_phase[0] != HomingPhase::Idle,
            self.axis_homing_phase[1] != HomingPhase::Idle,
            self.axis_homing_phase[2] != HomingPhase::Idle,
            self.axis_homing_phase[3] != HomingPhase::Idle,
        ];

        StateEvent {
            output_states: self.output_states,
            axis_speeds: self.axis_speeds,
            axis_target_speeds: self.axis_target_speeds,
            axis_accelerations: self.axis_accelerations,
            axis_target_positions: self.axis_target_positions,
            axis_position_mode: self.axis_position_mode,
            axis_homing_active: homing_active,
            axis_soft_limit_max: [
                soft_limits::max_position_mm(0),
                soft_limits::max_position_mm(1),
                soft_limits::max_position_mm(2),
                soft_limits::max_position_mm(3),
            ],
            axis_alarm_active: self.axis_alarm_active,
        }
    }

    /// Get live values (sensor readings, positions)
    pub fn get_live_values(&self) -> LiveValuesEvent {
        // Read digital inputs
        let mut input_states = [false; 8];
        for (i, di) in self.digital_inputs.iter().enumerate() {
            input_states[i] = di.get_value().unwrap_or(false);
        }

        // Read axis positions from PTO feedback (interpret u32 as i32 for negative positions)
        let mut positions = [0i32; 4];
        for (i, axis) in self.axes.iter().enumerate() {
            positions[i] = axis.get_position() as i32;
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
        self.namespace.emit(BbmAutomatikV2Events::LiveValues(event));
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
            // Cancel homing if active
            if self.axis_homing_phase[i] != HomingPhase::Idle {
                self.axis_homing_phase[i] = HomingPhase::Idle;
                tracing::info!("[BbmAutomatikV2] Axis {} homing cancelled by stop_all", i);
            }
            self.axis_speeds[i] = 0;
            self.axis_target_speeds[i] = 0;
            self.axis_position_mode[i] = false;

            // Hardware: disble_ramp breaks Travel Distance Control
            let mut output = self.axes[i].get_output();
            output.disble_ramp = true;
            output.go_counter = false;
            output.frequency_value = 0;
            self.axes[i].set_output(output);
        }
        self.emit_state();
    }

    /// Stop single axis - hardware immediate stop (also cancels homing if active)
    pub fn stop_axis(&mut self, index: usize) {
        if index < self.axis_speeds.len() {
            // Cancel homing if active
            if self.axis_homing_phase[index] != HomingPhase::Idle {
                self.axis_homing_phase[index] = HomingPhase::Idle;
                tracing::info!("[BbmAutomatikV2] Axis {} homing cancelled by stop", index);
            }
            self.axis_speeds[index] = 0;
            self.axis_target_speeds[index] = 0;
            self.axis_position_mode[index] = false;

            // Hardware: disble_ramp breaks Travel Distance Control
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
    /// For linear axes with ball screw
    pub fn set_axis_speed_mm_s(&mut self, index: usize, mm_per_s: f32) {
        if index < self.axis_target_speeds.len() {
            let hz = mechanics::mm_per_s_to_hz(mm_per_s);
            self.axis_target_speeds[index] = hz;
            // Hardware ramp accelerates/brakes automatically to target
            self.emit_state();
        }
    }

    /// Set target axis speed in RPM (hardware ramp handles transition)
    /// For rotation axes without ball screw (e.g., Bürste)
    pub fn set_axis_speed_rpm(&mut self, index: usize, rpm: f32) {
        if index < self.axis_target_speeds.len() {
            let hz = mechanics::rpm_to_hz(rpm);
            self.axis_target_speeds[index] = hz;
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

            // SDO-Write to the correct EL2522
            // NOTE: SdoWriteU16Fn returns () - errors are handled inside the callback.
            // If SDO write fails, the hardware keeps the old ramp values.
            if let Some(sdo_write) = &self.sdo_write_u16 {
                // Which EL2522? Axis 0,1 = EL2522#1, Axis 2,3 = EL2522#2
                let el2522_idx = if index < 2 { 0 } else { 1 };
                let subdevice_index = self.pto_subdevice_indices[el2522_idx];

                // PTO Base Index: Channel 1 = 0x8000, Channel 2 = 0x8010
                let pto_base = if index % 2 == 0 { 0x8000u16 } else { 0x8010u16 };

                tracing::debug!(
                    "[BbmAutomatikV2] SDO write axis {}: ramp rising={}ms falling={}ms (accel={:.0} mm/s²)",
                    index, rising_ms, falling_ms, clamped
                );

                // Rising ramp (0x14)
                sdo_write(subdevice_index, pto_base, 0x14, rising_ms);
                // Falling ramp (0x15)
                sdo_write(subdevice_index, pto_base, 0x15, falling_ms);
            } else {
                tracing::warn!("[BbmAutomatikV2] SDO write not available - acceleration change will not take effect");
            }
            self.emit_state();
        }
    }

    /// Move to a target position in mm using hardware Travel Distance Control
    /// Hardware ramps up, brakes, and stops exactly at target
    pub fn move_to_position_mm(&mut self, index: usize, position_mm: f32, speed_mm_s: f32) {
        if index < self.axes.len() {
            // Clamp to soft limits
            let clamped_mm = if let Some(max) = soft_limits::max_position_mm(index) {
                position_mm.clamp(soft_limits::MIN_MM, max)
            } else {
                position_mm
            };

            if (clamped_mm - position_mm).abs() > 0.1 {
                tracing::warn!(
                    "[BbmAutomatikV2] Axis {} position clamped: {:.1} mm -> {:.1} mm (soft limit)",
                    index, position_mm, clamped_mm
                );
            }

            let target_pulses = (clamped_mm.round() * mechanics::PULSES_PER_MM) as i32;
            let speed_hz = mechanics::mm_per_s_to_hz(speed_mm_s.abs());

            // Determine direction
            let current_pulses = self.axes[index].get_position() as i32;
            let direction = if target_pulses > current_pulses {
                1
            } else {
                -1
            };

            // Position mode activate
            self.axis_target_positions[index] = target_pulses;
            self.axis_position_mode[index] = true;
            // Ignore select_end_counter for 5 cycles (~3.5ms at 700µs cycle)
            // so hardware has time to process new go_counter and clear stale signal
            self.axis_position_ignore_cycles[index] = 5;

            // Hardware output: go_counter + target position + speed
            // In Travel Distance Control mode, the EL2522 hardware automatically
            // determines direction by comparing target_counter_value vs current position.
            // frequency_value must be POSITIVE (magnitude only) - sign is NOT used for direction.
            let mut output = self.axes[index].get_output();
            output.go_counter = true;
            output.disble_ramp = false;
            output.frequency_value = speed_hz;
            output.target_counter_value = target_pulses as u32;
            self.axes[index].set_output(output);

            // Sync software state (keep sign for UI display)
            self.axis_target_speeds[index] = speed_hz * direction;
            self.axis_speeds[index] = speed_hz * direction;

            self.emit_state();
            tracing::info!(
                "[BbmAutomatikV2] Axis {} moving to {:.0} mm ({} pulses) at {:.1} mm/s",
                index,
                clamped_mm.round(),
                target_pulses,
                speed_mm_s
            );
        }
    }

    /// Hardware ramp monitor: watches hardware status, does not set speeds
    /// Replaces the old update_software_ramp completely
    pub fn update_hardware_monitor(&mut self) -> bool {
        let mut changed = false;
        for i in 0..self.axis_speeds.len() {
            let input = self.axes[i].get_input();

            // ====== POSITION MODE: Target detection ======
            if self.axis_position_mode[i] {
                // Grace period after starting a new move: hardware needs time to
                // process go_counter and clear the stale select_end_counter signal
                if self.axis_position_ignore_cycles[i] > 0 {
                    self.axis_position_ignore_cycles[i] -= 1;
                } else if input.select_end_counter {
                    self.axis_speeds[i] = 0;
                    self.axis_target_speeds[i] = 0;
                    self.axis_position_mode[i] = false;

                    // Reset go_counter
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

            // ====== JOG MODE: Send speed directly to hardware ======
            if !self.axis_position_mode[i] {
                // Check soft limits during JOG
                if let Some(max_mm) = soft_limits::max_position_mm(i) {
                    let current_mm = self.axes[i].get_position() as i32 as f32 / mechanics::PULSES_PER_MM;
                    let target = self.axis_target_speeds[i];

                    // Stop if at max and moving positive, or at min and moving negative
                    if (current_mm >= max_mm && target > 0) || (current_mm <= soft_limits::MIN_MM && target < 0) {
                        if self.axis_target_speeds[i] != 0 {
                            self.axis_target_speeds[i] = 0;
                            tracing::warn!("[BbmAutomatikV2] Axis {} soft limit reached at {:.1} mm - stopping", i, current_mm);
                        }
                    }
                }

                let target = self.axis_target_speeds[i];
                if self.axis_speeds[i] != target {
                    // Send new target speed to hardware
                    // Hardware ramp accelerates/brakes automatically
                    let mut output = self.axes[i].get_output();
                    output.disble_ramp = false;
                    output.go_counter = false;
                    output.frequency_value = target;
                    self.axes[i].set_output(output);
                    self.axis_speeds[i] = target;
                    changed = true;
                }
            }

            // Status tracking for UI
            if input.ramp_active {
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

    /// Check if door sensor indicates safe (closed)
    pub fn are_doors_closed(&self) -> bool {
        self.digital_inputs[inputs::TUER]
            .get_value()
            .unwrap_or(false)
    }

    /// Check driver alarm pins and emergency-stop all axes if triggered
    /// Arduino equivalent: checkDriverAlarms() in BBMx22_Automatik_Code.ino v3.2
    pub fn check_driver_alarms(&mut self) -> bool {
        let alarm_inputs = [
            (axes::MT, inputs::ALARM_MT),
            (axes::SCHIEBER, inputs::ALARM_SCHIEBER),
            (axes::DRUECKER, inputs::ALARM_DRUECKER),
        ];

        for &(axis, input_idx) in &alarm_inputs {
            let raw = self.digital_inputs[input_idx]
                .get_value()
                .unwrap_or(!ALARM_ACTIVE_LOW);
            let is_alarm = if ALARM_ACTIVE_LOW { !raw } else { raw };

            if is_alarm && !self.axis_alarm_active[axis] {
                tracing::error!(
                    "[BbmAutomatikV2] !!! ALARM: Axis {} driver alarm triggered !!!",
                    axis
                );
                self.axis_alarm_active[axis] = true;
                self.stop_all_axes();
                return true;
            }
        }
        false
    }

    /// Reset all driver alarm states (only if physical alarm pins are inactive)
    pub fn reset_alarms(&mut self) {
        let had_alarm = self.axis_alarm_active.iter().any(|&a| a);
        if !had_alarm {
            return;
        }

        // Check if any physical alarm is still active before resetting
        let alarm_inputs = [
            (axes::MT, inputs::ALARM_MT),
            (axes::SCHIEBER, inputs::ALARM_SCHIEBER),
            (axes::DRUECKER, inputs::ALARM_DRUECKER),
        ];

        for &(axis, input_idx) in &alarm_inputs {
            let raw = self.digital_inputs[input_idx]
                .get_value()
                .unwrap_or(!ALARM_ACTIVE_LOW);
            let still_alarm = if ALARM_ACTIVE_LOW { !raw } else { raw };

            if still_alarm {
                tracing::warn!(
                    "[BbmAutomatikV2] Cannot reset alarms - Axis {} alarm still active on hardware",
                    axis
                );
                self.emit_state();
                return;
            }
        }

        self.axis_alarm_active = [false; 4];
        tracing::info!("[BbmAutomatikV2] All alarms reset");
        self.emit_state();
    }

    /// Check if axis is at home position (reference switch active)
    pub fn is_axis_homed(&self, axis: usize) -> bool {
        match axis {
            axes::MT => self.digital_inputs[inputs::REF_MT]
                .get_value()
                .unwrap_or(false),
            axes::SCHIEBER => self.digital_inputs[inputs::REF_SCHIEBER]
                .get_value()
                .unwrap_or(false),
            axes::DRUECKER => self.digital_inputs[inputs::REF_DRUECKER]
                .get_value()
                .unwrap_or(false),
            _ => false, // Bürste has no home switch
        }
    }

    // ============ Homing Functions ============

    /// Start homing sequence for an axis
    /// Sequence: 1) Move negative until sensor, 2) Retract 2mm, 3) Set position to 0
    pub fn start_homing(&mut self, index: usize) {
        if index >= self.axes.len() || index == axes::BUERSTE {
            tracing::warn!(
                "[BbmAutomatikV2] Cannot home axis {} (invalid or rotation axis)",
                index
            );
            return;
        }

        // If already homing, ignore
        if self.axis_homing_phase[index] != HomingPhase::Idle {
            tracing::warn!("[BbmAutomatikV2] Axis {} already homing", index);
            return;
        }

        // Start Phase 1: Search for sensor (move negative)
        self.axis_homing_phase[index] = HomingPhase::SearchingSensor;
        self.axis_position_mode[index] = false;

        // Set slow homing speed in negative direction
        let homing_hz = -mechanics::mm_per_s_to_hz(homing::HOMING_SPEED_MM_S);
        self.axis_target_speeds[index] = homing_hz;

        self.emit_state();
        tracing::info!(
            "[BbmAutomatikV2] Axis {} homing Phase 1: Searching sensor at {} Hz ({:.1} mm/s)",
            index,
            homing_hz,
            homing::HOMING_SPEED_MM_S
        );
    }

    /// Cancel homing for an axis
    pub fn cancel_homing(&mut self, index: usize) {
        if index < self.axes.len() && self.axis_homing_phase[index] != HomingPhase::Idle {
            self.axis_homing_phase[index] = HomingPhase::Idle;
            self.stop_axis(index);
            tracing::info!("[BbmAutomatikV2] Axis {} homing cancelled", index);
        }
    }

    /// Update homing state machine
    /// Called from act() loop
    pub fn update_homing(&mut self) {
        for i in 0..self.axes.len() {
            match self.axis_homing_phase[i] {
                HomingPhase::Idle => continue,

                HomingPhase::SearchingSensor => {
                    // Check if reference switch is triggered
                    if self.is_axis_homed(i) {
                        // Stop the axis
                        self.axis_speeds[i] = 0;
                        self.axis_target_speeds[i] = 0;
                        let mut output = self.axes[i].get_output();
                        output.disble_ramp = false;
                        output.go_counter = false;
                        output.frequency_value = 0;
                        self.axes[i].set_output(output);

                        // Calculate retract target: current position + 2mm
                        let current_pos = self.axes[i].get_position() as i32;
                        let retract_pulses =
                            (homing::RETRACT_DISTANCE_MM * mechanics::PULSES_PER_MM) as i32;
                        self.axis_homing_retract_target[i] = current_pos + retract_pulses;

                        // Start Phase 2: Retract
                        self.axis_homing_phase[i] = HomingPhase::Retracting;

                        // Move positive (away from sensor)
                        let retract_hz = mechanics::mm_per_s_to_hz(homing::HOMING_SPEED_MM_S);
                        self.axis_target_speeds[i] = retract_hz;

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing Phase 2: Retracting {:.1}mm (target: {} pulses)",
                            i,
                            homing::RETRACT_DISTANCE_MM,
                            self.axis_homing_retract_target[i]
                        );
                        self.emit_state();
                    }
                }

                HomingPhase::Retracting => {
                    // Check if we reached the retract target
                    let current_pos = self.axes[i].get_position() as i32;
                    if current_pos >= self.axis_homing_retract_target[i] {
                        // Stop the axis
                        self.axis_speeds[i] = 0;
                        self.axis_target_speeds[i] = 0;
                        let mut output = self.axes[i].get_output();
                        output.disble_ramp = false;
                        output.go_counter = false;
                        output.frequency_value = 0;
                        self.axes[i].set_output(output);

                        // Start Phase 3: Set zero
                        self.axis_homing_phase[i] = HomingPhase::SettingZero;

                        // Reset position counter to 0
                        self.axes[i].reset_position();

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing Phase 3: Setting position to 0",
                            i
                        );
                        self.emit_state();
                    }
                }

                HomingPhase::SettingZero => {
                    // Check if set_counter is done (wait one cycle)
                    let input = self.axes[i].get_input();
                    if input.set_counter_done || input.counter_value == 0 {
                        // Clear the set_counter flag
                        self.axes[i].clear_set_counter();

                        // Homing complete!
                        self.axis_homing_phase[i] = HomingPhase::Idle;

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing COMPLETE - position is now 0",
                            i
                        );
                        self.emit_state();
                    }
                }
            }
        }
    }
}
