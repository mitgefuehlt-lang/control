use crate::bbm_automatik_v2::api::{BbmAutomatikV2Events, LiveValuesEvent, StateEvent};
use crate::machine_identification::{MachineIdentification, MachineIdentificationUnique};
use crate::{AsyncThreadMessage, BBM_AUTOMATIK_V2, Machine, MachineMessage, VENDOR_QITECH};
use control_core::socketio::namespace::NamespaceCacheingLogic;
use ethercat_hal::io::digital_input::DigitalInput;
use ethercat_hal::io::digital_output::DigitalOutput;
use ethercat_hal::io::pulse_train_output::PulseTrainOutput;
use serde::{Deserialize, Serialize};
use smol::channel::{Receiver, Sender};
use std::time::Instant;

pub mod act;
pub mod api;
pub mod new;

use crate::bbm_automatik_v2::api::BbmAutomatikV2Namespace;

/// Device Roles for BbmAutomatikV2
/// Hardware: 2x EL2522 (3 Achsen), EL1008, EL2008
pub mod roles {
    pub const DIGITAL_INPUT: u16 = 1; // EL1008 - 8x DI (3x Alarm, 3x Referenzschalter NC, 1x Türsensor)
    pub const DIGITAL_OUTPUT: u16 = 2; // EL2008 - 8x DO (3x Ampel, 1x Bürstenmotor, 1x Rüttelmotor, 1x Pneumatik, 1x Lüfter)
    pub const PTO_1: u16 = 3; // EL2522 #1 - Kanal 1: MT, Kanal 2: Schieber
    pub const PTO_2: u16 = 4; // EL2522 #2 - Kanal 1: Drücker, Kanal 2: unused
}

/// Axis indices (PTO axes only - Bürste is now a digital output)
pub mod axes {
    pub const MT: usize = 0; // Magazin Transporter (Linear)
    pub const SCHIEBER: usize = 1; // Schieber (Linear)
    pub const DRUECKER: usize = 2; // Drücker (Linear)
}

/// Digital input indices (0-based array index, DI1 = index 0)
pub mod inputs {
    pub const ALARM_MT: usize = 0; // CL75t Alarm Transporter (DI1 = index 0)
    pub const ALARM_SCHIEBER: usize = 1; // CL75t Alarm Schieber (DI2 = index 1)
    pub const ALARM_DRUECKER: usize = 2; // CL75t Alarm Drücker (DI3 = index 2)
    pub const REF_MT: usize = 3; // Referenzschalter Transporter (DI4 = index 3, NC: true=frei, false=Endlage)
    pub const REF_SCHIEBER: usize = 4; // Referenzschalter Schieber (DI5 = index 4, NC: true=frei, false=Endlage)
    pub const REF_DRUECKER: usize = 5; // Referenzschalter Drücker (DI6 = index 5, NC: true=frei, false=Endlage)
    pub const TUER: usize = 6; // Türsensor (DI7 = index 6)
}

/// Digital output indices
pub mod outputs {
    pub const AMPEL_GRUEN: usize = 0; // Ampel Grün (DO1 = index 0)
    pub const AMPEL_GELB: usize = 1; // Ampel Gelb (DO2 = index 1)
    pub const AMPEL_ROT: usize = 2; // Ampel Rot (DO3 = index 2)
    pub const BUERSTENMOTOR: usize = 3; // Bürstenmotor on/off (DO4 = index 3)
    pub const RUETTELMOTOR: usize = 4; // Rüttelmotor (DO5 = index 4)
    pub const PNEUMATIK: usize = 5; // Pneumatik 3/2-Ventil (DO6 = index 5)
    pub const LUEFTER: usize = 6; // Schaltschrank-Lüfter (DO7 = index 6)
}

/// Soft limits per axis in mm (0 = home position after homing)
/// Values from Arduino BBMx22_Automatik_Code.ino v3.2
pub mod soft_limits {
    pub const MT_MAX_MM: f32 = 230.0;
    pub const SCHIEBER_MAX_MM: f32 = 53.0;
    pub const DRUECKER_MAX_MM: f32 = 107.0;
    pub const MIN_MM: f32 = 0.0;

    /// Get max position for axis in mm (None = no limit)
    pub fn max_position_mm(axis: usize) -> Option<f32> {
        match axis {
            super::axes::MT => Some(MT_MAX_MM),
            super::axes::SCHIEBER => Some(SCHIEBER_MAX_MM),
            super::axes::DRUECKER => Some(DRUECKER_MAX_MM),
            _ => None,
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

/// Speed presets for auto-sequence (mm/s)
pub mod speed_presets {
    #[derive(Debug, Clone, Copy)]
    pub struct SpeedPreset {
        pub mt_mm_s: f32,
        pub schieber_mm_s: f32,
        pub druecker_mm_s: f32,
    }
    pub const SLOW: SpeedPreset = SpeedPreset {
        mt_mm_s: 30.0,
        schieber_mm_s: 40.0,
        druecker_mm_s: 40.0,
    };
    pub const MEDIUM: SpeedPreset = SpeedPreset {
        mt_mm_s: 60.0,
        schieber_mm_s: 80.0,
        druecker_mm_s: 80.0,
    };
    pub const FAST: SpeedPreset = SpeedPreset {
        mt_mm_s: 100.0,
        schieber_mm_s: 150.0,
        druecker_mm_s: 150.0,
    };
}

/// Position constants (mm) from Arduino v3.2
pub mod auto_positions {
    pub const MT_START: f32 = 5.0;
    pub const MT_RUN: f32 = 34.5;
    pub const MT_ADVANCE_PER_CYCLE: f32 = 10.0;
    pub const SCHIEBER_START: f32 = 7.0;
    pub const SCHIEBER_TARGET: f32 = 51.0;
    pub const SCHIEBER_WOBBLE: f32 = 1.5;
    pub const DRUECKER_START: f32 = 60.0;
    pub const DRUECKER_TARGET: f32 = 105.0;
    pub const CYCLES_PER_BLOCK: u32 = 19;
    pub const BLOCKS_PER_SET: u32 = 3;
}

/// Cycle step within one fill cycle
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoCycleStep {
    /// Step 1a: Wobble schieber out (+wobble from start)
    WobbleOut,
    /// Step 1b: Wobble schieber back (-wobble from start)
    WobbleBack,
    /// Step 2: Schieber to target (filters fall into magazine)
    SchieberToTarget,
    /// Step 3: Drücker pushes hanging filters
    DrueckerToTarget,
    /// Step 4: Drücker + Schieber return in parallel, MT advances
    ParallelReturn,
    /// Wait for all parallel moves to complete
    WaitParallelComplete,
}

/// Top-level auto-sequence state
#[derive(Debug, Clone)]
pub struct AutoSequenceState {
    pub speed_preset_name: String,
    pub speed: speed_presets::SpeedPreset,
    pub total_sets: u32,
    pub current_set: u32,
    pub current_block: u32,
    pub current_cycle: u32,
    pub current_step: AutoCycleStep,
    pub mt_current_run_pos: f32,
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

// ============ Teach / Calibration ============

/// A user-saved (teach-in) position with a name. Used for the 2 freely
/// nameable slots per axis. Start/Ziel are stored as bare `Option<f32>`
/// because their name is fixed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedTeachPosition {
    pub name: String,
    pub position_mm: f32,
}

/// All teach-in positions for one axis. Persisted to disk so calibration
/// survives reboots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AxisTeachPositions {
    #[serde(default)]
    pub start_mm: Option<f32>,
    #[serde(default)]
    pub ziel_mm: Option<f32>,
    #[serde(default)]
    pub custom1: Option<NamedTeachPosition>,
    #[serde(default)]
    pub custom2: Option<NamedTeachPosition>,
}

/// Identifier for a teach slot (Start/Ziel are fixed; Custom1/Custom2 are
/// freely nameable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachSlot {
    Start,
    Ziel,
    Custom1,
    Custom2,
}

/// Calibration file load/save. Calibration lives at
/// `$STATE_DIRECTORY/bbm-automatik-v2-calibration.json` (systemd sets
/// STATE_DIRECTORY for our service via `StateDirectory=qitech`). On dev
/// machines without that env var we fall back to the OS temp dir so tests
/// don't litter `/var/lib/qitech`.
pub mod calibration {
    use super::AxisTeachPositions;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    const FILENAME: &str = "bbm-automatik-v2-calibration.json";

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct CalibrationFile {
        #[serde(default)]
        axes: [AxisTeachPositions; 3],
    }

    fn path() -> PathBuf {
        if let Ok(dir) = std::env::var("STATE_DIRECTORY") {
            return PathBuf::from(dir).join(FILENAME);
        }
        if cfg!(target_os = "linux") {
            PathBuf::from("/var/lib/qitech").join(FILENAME)
        } else {
            std::env::temp_dir().join(FILENAME)
        }
    }

    pub fn load() -> [AxisTeachPositions; 3] {
        let p = path();
        match std::fs::read_to_string(&p) {
            Ok(s) => match serde_json::from_str::<CalibrationFile>(&s) {
                Ok(f) => {
                    tracing::info!(
                        "[BbmAutomatikV2] Loaded calibration from {}",
                        p.display()
                    );
                    f.axes
                }
                Err(e) => {
                    tracing::warn!(
                        "[BbmAutomatikV2] Calibration file at {} is corrupt ({}) - starting empty",
                        p.display(),
                        e
                    );
                    Default::default()
                }
            },
            Err(_) => {
                tracing::info!(
                    "[BbmAutomatikV2] No calibration file at {} - starting empty",
                    p.display()
                );
                Default::default()
            }
        }
    }

    /// Atomically persist the calibration. Errors are logged, not propagated -
    /// the user can re-save and the in-memory state is still correct.
    pub fn save(axes: &[AxisTeachPositions; 3]) {
        let p = path();
        if let Some(parent) = p.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!(
                    "[BbmAutomatikV2] Failed to create calibration dir {}: {}",
                    parent.display(),
                    e
                );
                return;
            }
        }

        let file = CalibrationFile { axes: axes.clone() };
        let json = match serde_json::to_string_pretty(&file) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("[BbmAutomatikV2] Failed to serialize calibration: {}", e);
                return;
            }
        };

        let tmp = p.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, &json) {
            tracing::error!(
                "[BbmAutomatikV2] Failed to write {}: {}",
                tmp.display(),
                e
            );
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &p) {
            tracing::error!(
                "[BbmAutomatikV2] Failed to commit calibration to {}: {}",
                p.display(),
                e
            );
        }
    }
}

/// Virtual zero offset for the EL2522 hardware position counter.
///
/// The EL2522 stores its position counter as a `u32` and Travel Distance
/// Control compares `target_counter_value` vs the live counter UNSIGNED.
/// To drive to logically negative positions we initialise the hardware
/// counter to this offset; logical position is then
/// `hw_counter - axis_position_offset[i]` (interpreted i32). With both
/// values comfortably in the lower half of u32, the unsigned compare
/// picks the physically correct direction every time and TDC brakes
/// hardware-precisely in both directions.
///
/// This is the canonical pattern Beckhoff documents in the EL252x
/// manual (§6.4.3 "Connection of the EL2522 in the NC", §6.5.1.2 "Travel
/// Distance Control") — TwinCAT NC owns the signed logical position
/// internally and writes only positive u32 values to the terminal.
///
/// 1_000_000 pulses at 20 pulses/mm = 50_000 mm headroom in either
/// direction, vastly more than any physical axis on this machine.
pub const POSITION_OFFSET_PULSES: u32 = 1_000_000;

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

/// Alarm polarity: CL75t without pull-ups = active HIGH (true = alarm, false/0V = no alarm)
const ALARM_ACTIVE_LOW: bool = false;

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

    // Pulse Train Outputs (2x EL2522 = 3 channels used)
    // Axis 0: MT (EL2522 #1, Ch1)
    // Axis 1: Schieber (EL2522 #1, Ch2)
    // Axis 2: Drücker (EL2522 #2, Ch1)
    pub axes: [PulseTrainOutput; 3],
    pub axis_speeds: [i32; 3],
    pub axis_target_speeds: [i32; 3],
    pub axis_accelerations: [f32; 3],
    pub axis_target_positions: [i32; 3],
    pub axis_position_mode: [bool; 3],
    /// Ignore select_end_counter for N cycles after starting a new move
    /// (hardware needs time to process go_counter and clear the old signal)
    pub axis_position_ignore_cycles: [u8; 3],

    // Hardware ramp control
    pub sdo_write_u16: Option<crate::SdoWriteU16Fn>,
    pub pto_subdevice_indices: [usize; 2],

    // Homing state
    pub axis_homing_phase: [HomingPhase; 3],
    pub axis_homing_retract_target: [u32; 3],
    /// true once homing Phase 3 (SettingZero) has completed for this axis.
    /// Soft limits are only enforced after this flag is set.
    pub axis_homed: [bool; 3],

    /// Virtual zero offset per axis (see [`POSITION_OFFSET_PULSES`]).
    /// Initialised to `POSITION_OFFSET_PULSES` at machine construction so
    /// negative logical jogs work even before homing; reapplied on Homing
    /// Phase 3 so logical 0 coincides with the physical home position.
    pub axis_position_offset: [u32; 3],

    // Driver alarm state (CL75t alarm pins)
    /// true = alarm active (axis stopped), per axis
    pub axis_alarm_active: [bool; 3],

    // Door interlock
    pub door_interlock_active: bool,

    // Auto-sequence state machine
    pub auto_sequence: Option<AutoSequenceState>,

    // Calibration / teach-in positions per axis (persisted to disk)
    pub teach_positions: [AxisTeachPositions; 3],

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
            ],
            axis_alarm_active: self.axis_alarm_active,
            door_interlock_active: self.door_interlock_active,
            auto_running: self.auto_sequence.is_some(),
            auto_current_set: self.auto_sequence.as_ref().map(|s| s.current_set).unwrap_or(0),
            auto_current_block: self.auto_sequence.as_ref().map(|s| s.current_block).unwrap_or(0),
            auto_current_cycle: self.auto_sequence.as_ref().map(|s| s.current_cycle).unwrap_or(0),
            auto_total_sets: self.auto_sequence.as_ref().map(|s| s.total_sets).unwrap_or(0),
            teach_positions: self.teach_positions.clone(),
        }
    }

    /// Get live values (sensor readings, positions)
    pub fn get_live_values(&self) -> LiveValuesEvent {
        // Read digital inputs
        let mut input_states = [false; 8];
        for (i, di) in self.digital_inputs.iter().enumerate() {
            input_states[i] = di.get_value().unwrap_or(false);
        }

        // Read axis positions in logical pulses (signed, hw counter minus offset)
        let mut positions = [0i32; 3];
        for i in 0..self.axes.len() {
            positions[i] = self.current_logical_pulses(i);
        }

        LiveValuesEvent {
            input_states,
            axis_positions: positions,
        }
    }

    /// Current axis position in logical pulses (signed). With the virtual
    /// zero offset applied this returns the actual logical position; if
    /// the offset hasn't been applied yet (offset == 0) the raw counter
    /// is returned reinterpreted as i32.
    pub fn current_logical_pulses(&self, axis: usize) -> i32 {
        self.axes[axis]
            .get_position()
            .wrapping_sub(self.axis_position_offset[axis]) as i32
    }

    /// Current axis position in mm (logical, signed).
    pub fn current_logical_mm(&self, axis: usize) -> f32 {
        self.current_logical_pulses(axis) as f32 / mechanics::PULSES_PER_MM
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
    /// For rotation axes without ball screw
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
                // Which EL2522? Axis 0,1 = EL2522#1, Axis 2 = EL2522#2
                let el2522_idx = if index < 2 { 0 } else { 1 };
                let subdevice_index = self.pto_subdevice_indices[el2522_idx];

                // PTO Base Index: Channel 1 = 0x8000, Channel 2 = 0x8010
                let pto_base = if index % 2 == 0 { 0x8000u16 } else { 0x8010u16 };

                tracing::debug!(
                    "[BbmAutomatikV2] SDO write axis {}: ramp rising={}ms falling={}ms (accel={:.0} mm/s²)",
                    index,
                    rising_ms,
                    falling_ms,
                    clamped
                );

                // Rising ramp (0x14)
                sdo_write(subdevice_index, pto_base, 0x14, rising_ms);
                // Falling ramp (0x15)
                sdo_write(subdevice_index, pto_base, 0x15, falling_ms);
            } else {
                tracing::warn!(
                    "[BbmAutomatikV2] SDO write not available - acceleration change will not take effect"
                );
            }
            self.emit_state();
        }
    }

    /// Move to a logical target position in mm using hardware Travel
    /// Distance Control. The hardware ramps up, brakes, and stops
    /// hardware-precisely at the target in BOTH directions thanks to the
    /// virtual zero offset (see [`POSITION_OFFSET_PULSES`]): logical
    /// targets are translated to `logical + offset` in u32 hardware
    /// space, so the EL2522's unsigned compare always picks the
    /// physically correct direction.
    pub fn move_to_position_mm(&mut self, index: usize, position_mm: f32, speed_mm_s: f32) {
        if index >= self.axes.len() {
            return;
        }

        // Only the upper soft limit is enforced. Lower limit (MIN_MM) is
        // intentionally not enforced so the user can drive into negative
        // territory (calibration, recalibration, testing the lower end).
        let enforce_max =
            self.axis_homed[index] && self.axis_homing_phase[index] == HomingPhase::Idle;
        let clamped_mm = if enforce_max {
            match soft_limits::max_position_mm(index) {
                Some(max) => position_mm.min(max),
                None => position_mm,
            }
        } else {
            position_mm
        };

        if (clamped_mm - position_mm).abs() > 0.1 {
            tracing::warn!(
                "[BbmAutomatikV2] Axis {} position clamped: {:.1} mm -> {:.1} mm (soft max)",
                index,
                position_mm,
                clamped_mm
            );
        }

        // Round AFTER scaling to pulses so we keep sub-mm precision
        // (43.5 mm * 20 pulses/mm = exactly 870 pulses). Rounding mm
        // first would quantize the target to whole millimetres.
        let target_logical_pulses =
            (clamped_mm * mechanics::PULSES_PER_MM).round() as i32;
        let current_logical_pulses = self.current_logical_pulses(index);
        // Hardware target = logical + offset, computed in i64 to avoid any
        // i32/u32 ambiguity, then narrowed once we know it fits.
        let target_hw_u32 = (target_logical_pulses as i64
            + self.axis_position_offset[index] as i64) as u32;
        let speed_hz = mechanics::mm_per_s_to_hz(speed_mm_s.abs());

        // Direction in logical space (just for UI sign on axis_speeds).
        let direction = if target_logical_pulses >= current_logical_pulses {
            1
        } else {
            -1
        };

        // Activate position mode and remember the logical target so step
        // loss detection can later compare apples to apples.
        self.axis_target_positions[index] = target_logical_pulses;
        self.axis_position_mode[index] = true;
        // Ignore select_end_counter for 5 cycles (~3.5ms at 700µs cycle)
        // so hardware has time to process the new go_counter and clear
        // the stale "target reached" signal from the previous move.
        self.axis_position_ignore_cycles[index] = 5;

        // Hardware output: go_counter + target position + speed magnitude.
        // In Travel Distance Control mode the EL2522 picks direction from
        // the unsigned compare target_counter_value vs counter. With the
        // virtual offset applied, both values stay comfortably positive
        // and direction is always physically correct. frequency_value is
        // magnitude only — the sign is NOT used for direction in TDC.
        let mut output = self.axes[index].get_output();
        output.go_counter = true;
        output.disble_ramp = false;
        output.frequency_value = speed_hz;
        output.target_counter_value = target_hw_u32;
        self.axes[index].set_output(output);

        // Sync software state (signed speed for UI direction indicator).
        self.axis_target_speeds[index] = speed_hz * direction;
        self.axis_speeds[index] = speed_hz * direction;

        self.emit_state();
        tracing::info!(
            "[BbmAutomatikV2] Axis {} moving to {:.3} mm ({} logical pulses, hw target {}) at {:.1} mm/s",
            index,
            clamped_mm,
            target_logical_pulses,
            target_hw_u32,
            speed_mm_s
        );
    }

    /// Relative jog by `delta_mm`. Now a thin wrapper around
    /// [`Self::move_to_position_mm`] — with the virtual zero offset in
    /// place, TDC handles both directions hardware-precisely. Used by
    /// the +/- JOG buttons.
    pub fn jog_relative(&mut self, index: usize, delta_mm: f32, speed_mm_s: f32) {
        if index >= self.axes.len() {
            return;
        }
        let target_mm = self.current_logical_mm(index) + delta_mm;
        self.move_to_position_mm(index, target_mm, speed_mm_s);
    }

    /// Hardware ramp monitor: watches the EL2522 status flags and pushes
    /// frequency setpoints to the hardware. Two modes:
    ///
    /// - **Position mode** (TDC): wait for `select_end_counter` to flag
    ///   "target reached", then clear `go_counter` and log step-loss if
    ///   actual deviated from target.
    /// - **Speed mode**: used by homing (and by anything else writing
    ///   `axis_target_speeds` directly). Enforces the upper soft limit
    ///   when homed, then forwards the target frequency to hardware.
    pub fn update_hardware_monitor(&mut self) -> bool {
        let mut changed = false;
        for i in 0..self.axis_speeds.len() {
            let input = self.axes[i].get_input();

            // Auto-clear a pending set_counter once hardware confirms.
            // Both machine init (virtual-zero offset write) and homing
            // Phase 3 raise set_counter; this single path turns it back
            // off so the EL2522 doesn't keep clamping the counter to the
            // set value forever.
            if input.set_counter_done {
                let output = self.axes[i].get_output();
                if output.set_counter {
                    self.axes[i].clear_set_counter();
                }
            }

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
                    let actual_pos = self.current_logical_pulses(i);
                    let target_pos = self.axis_target_positions[i];
                    let deviation = (actual_pos - target_pos).abs();
                    if deviation > 2 {
                        tracing::warn!(
                            "[Axis {}] STEP LOSS DETECTED: target={} actual={} deviation={} pulses ({:.2} mm)",
                            i,
                            target_pos,
                            actual_pos,
                            deviation,
                            deviation as f32 / mechanics::PULSES_PER_MM
                        );
                    } else {
                        tracing::info!(
                            "[Axis {}] Target reached: {} pulses (actual: {}, deviation: {})",
                            i,
                            target_pos,
                            actual_pos,
                            deviation
                        );
                    }
                }
            }

            // ====== SPEED MODE: Send target frequency to hardware ======
            if !self.axis_position_mode[i] {
                // Soft upper limit only applies once axis is homed AND not
                // currently homing. Lower limit is intentionally not
                // enforced — user wants negative travel for calibration.
                let enforce_max =
                    self.axis_homed[i] && self.axis_homing_phase[i] == HomingPhase::Idle;
                if enforce_max {
                    if let Some(max_mm) = soft_limits::max_position_mm(i) {
                        let current_mm = self.current_logical_mm(i);
                        if current_mm >= max_mm
                            && self.axis_target_speeds[i] > 0
                        {
                            self.axis_target_speeds[i] = 0;
                            tracing::warn!(
                                "[BbmAutomatikV2] Axis {} soft max reached at {:.1} mm - stopping",
                                i,
                                current_mm
                            );
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

    /// Set Bürstenmotor on/off
    pub fn set_buerstenmotor(&mut self, on: bool) {
        self.set_output(outputs::BUERSTENMOTOR, on);
    }

    /// Set Rüttelmotor on/off
    pub fn set_ruettelmotor(&mut self, on: bool) {
        self.set_output(outputs::RUETTELMOTOR, on);
    }

    /// Set Pneumatik valve on/off
    pub fn set_pneumatik(&mut self, on: bool) {
        self.set_output(outputs::PNEUMATIK, on);
    }

    /// Set Schaltschrank-Lüfter on/off
    pub fn set_luefter(&mut self, on: bool) {
        self.set_output(outputs::LUEFTER, on);
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

        let mut any_new_alarm = false;

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
                any_new_alarm = true;
            }
        }

        if any_new_alarm {
            self.stop_all_axes();
        }
        any_new_alarm
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

        self.axis_alarm_active = [false; 3];
        tracing::info!("[BbmAutomatikV2] All alarms reset");
        self.emit_state();
    }

    /// Check if axis is at home position (reference switch triggered)
    /// Ref switches are NC (normally closed): 24V/true = free, 0V/false = end position reached
    /// So sensor is triggered (at home) when value is false (signal interrupted)
    pub fn is_axis_homed(&self, axis: usize) -> bool {
        match axis {
            axes::MT => !self.digital_inputs[inputs::REF_MT]
                .get_value()
                .unwrap_or(true),
            axes::SCHIEBER => !self.digital_inputs[inputs::REF_SCHIEBER]
                .get_value()
                .unwrap_or(true),
            axes::DRUECKER => !self.digital_inputs[inputs::REF_DRUECKER]
                .get_value()
                .unwrap_or(true),
            _ => false,
        }
    }

    // ============ Homing Functions ============

    /// Start homing sequence for an axis
    /// Sequence: 1) Move negative until sensor, 2) Retract 2mm, 3) Set position to 0
    pub fn start_homing(&mut self, index: usize) {
        if index >= self.axes.len() {
            tracing::warn!(
                "[BbmAutomatikV2] Cannot home axis {} (invalid axis)",
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

                        // Calculate retract target in hw-counter space. Hw
                        // counter never wraps over realistic homing distances
                        // (RETRACT_DISTANCE_MM is small), so a plain u32 add
                        // is safe.
                        let current_hw = self.axes[i].get_position();
                        let retract_pulses =
                            (homing::RETRACT_DISTANCE_MM * mechanics::PULSES_PER_MM) as u32;
                        self.axis_homing_retract_target[i] =
                            current_hw.wrapping_add(retract_pulses);

                        // Start Phase 2: Retract
                        self.axis_homing_phase[i] = HomingPhase::Retracting;

                        // Move positive (away from sensor)
                        let retract_hz = mechanics::mm_per_s_to_hz(homing::HOMING_SPEED_MM_S);
                        self.axis_target_speeds[i] = retract_hz;

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing Phase 2: Retracting {:.1}mm (target hw {})",
                            i,
                            homing::RETRACT_DISTANCE_MM,
                            self.axis_homing_retract_target[i]
                        );
                        self.emit_state();
                    }
                }

                HomingPhase::Retracting => {
                    // Check if we reached the retract target (compare in
                    // signed delta space so wraparound doesn't fool us).
                    let current_hw = self.axes[i].get_position();
                    let delta = current_hw
                        .wrapping_sub(self.axis_homing_retract_target[i])
                        as i32;
                    if delta >= 0 {
                        // Stop the axis
                        self.axis_speeds[i] = 0;
                        self.axis_target_speeds[i] = 0;
                        let mut output = self.axes[i].get_output();
                        output.disble_ramp = false;
                        output.go_counter = false;
                        output.frequency_value = 0;

                        // Start Phase 3: Set the hw counter to the virtual
                        // offset so logical 0 is right here (physical
                        // sensor + 2 mm retract). axis_position_offset is
                        // updated eagerly so logical reads are correct
                        // immediately; the SettingZero phase just waits
                        // for the hardware to confirm via set_counter_done.
                        output.set_counter = true;
                        output.set_counter_value = POSITION_OFFSET_PULSES;
                        self.axes[i].set_output(output);
                        self.axis_position_offset[i] = POSITION_OFFSET_PULSES;
                        self.axis_homing_phase[i] = HomingPhase::SettingZero;

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing Phase 3: Setting hw counter to offset {} (logical 0)",
                            i,
                            POSITION_OFFSET_PULSES
                        );
                        self.emit_state();
                    }
                }

                HomingPhase::SettingZero => {
                    // Wait for the hardware to apply the set_counter write.
                    let input = self.axes[i].get_input();
                    if input.set_counter_done
                        || input.counter_value == self.axis_position_offset[i]
                    {
                        // Clear the set_counter flag
                        self.axes[i].clear_set_counter();

                        // Homing complete!
                        self.axis_homing_phase[i] = HomingPhase::Idle;
                        // From here on, soft limits apply (position counter is now calibrated).
                        self.axis_homed[i] = true;

                        tracing::info!(
                            "[BbmAutomatikV2] Axis {} homing COMPLETE - logical position is now 0",
                            i
                        );
                        self.emit_state();
                    }
                }
            }
        }
    }

    // ============ Door Interlock ============

    /// Door interlock: if door opens during operation, emergency-stop all axes
    /// Returns true if interlock state changed (for UI update)
    pub fn check_door_interlock(&mut self) -> bool {
        let door_closed = self.are_doors_closed();
        let any_moving = self.axis_speeds.iter().any(|&s| s != 0)
            || self.auto_sequence.is_some();

        if !door_closed && any_moving && !self.door_interlock_active {
            tracing::warn!("[BbmAutomatikV2] !!! DOOR OPEN - Emergency stop !!!");
            self.door_interlock_active = true;
            self.stop_all_axes();
            // Abort auto sequence if running
            if self.auto_sequence.is_some() {
                self.auto_sequence = None;
                self.set_ruettelmotor(false);
                self.set_ampel(true, false, false);
            }
            return true;
        }

        // Auto-reset when door closes again
        if door_closed && self.door_interlock_active {
            self.door_interlock_active = false;
            tracing::info!("[BbmAutomatikV2] Door closed - interlock reset");
            return true;
        }
        false
    }

    // ============ Auto-Sequence State Machine ============

    /// True if axis is mid-move (Travel Distance Control in flight).
    /// Used by the auto-sequence state machine to know when to advance.
    #[inline]
    fn is_axis_moving(&self, index: usize) -> bool {
        self.axis_position_mode[index]
    }

    /// Update auto-sequence state machine (called from act() loop)
    /// Returns true if state changed (for UI update)
    pub fn update_auto_sequence(&mut self) -> bool {
        let seq = match &self.auto_sequence {
            Some(s) => s.clone(),
            None => return false,
        };

        match seq.current_step {
            AutoCycleStep::WobbleOut => {
                if !self.is_axis_moving(axes::SCHIEBER) {
                    // Wobble out complete, now wobble back
                    self.move_to_position_mm(
                        axes::SCHIEBER,
                        auto_positions::SCHIEBER_START - auto_positions::SCHIEBER_WOBBLE,
                        seq.speed.schieber_mm_s,
                    );
                    self.auto_sequence.as_mut().unwrap().current_step =
                        AutoCycleStep::WobbleBack;
                    return true;
                }
            }
            AutoCycleStep::WobbleBack => {
                if !self.is_axis_moving(axes::SCHIEBER) {
                    // Wobble done, schieber to target
                    self.move_to_position_mm(
                        axes::SCHIEBER,
                        auto_positions::SCHIEBER_TARGET,
                        seq.speed.schieber_mm_s,
                    );
                    self.auto_sequence.as_mut().unwrap().current_step =
                        AutoCycleStep::SchieberToTarget;
                    return true;
                }
            }
            AutoCycleStep::SchieberToTarget => {
                if !self.is_axis_moving(axes::SCHIEBER) {
                    // Schieber at target, now drücker pushes
                    self.move_to_position_mm(
                        axes::DRUECKER,
                        auto_positions::DRUECKER_TARGET,
                        seq.speed.druecker_mm_s,
                    );
                    self.auto_sequence.as_mut().unwrap().current_step =
                        AutoCycleStep::DrueckerToTarget;
                    return true;
                }
            }
            AutoCycleStep::DrueckerToTarget => {
                if !self.is_axis_moving(axes::DRUECKER) {
                    // Drücker done, parallel return
                    self.move_to_position_mm(
                        axes::DRUECKER,
                        auto_positions::DRUECKER_START,
                        seq.speed.druecker_mm_s,
                    );
                    self.move_to_position_mm(
                        axes::SCHIEBER,
                        auto_positions::SCHIEBER_START,
                        seq.speed.schieber_mm_s,
                    );
                    let new_mt_pos =
                        seq.mt_current_run_pos - auto_positions::MT_ADVANCE_PER_CYCLE;
                    self.move_to_position_mm(axes::MT, new_mt_pos, seq.speed.mt_mm_s);
                    let s = self.auto_sequence.as_mut().unwrap();
                    s.mt_current_run_pos = new_mt_pos;
                    s.current_step = AutoCycleStep::ParallelReturn;
                    return true;
                }
            }
            AutoCycleStep::ParallelReturn | AutoCycleStep::WaitParallelComplete => {
                // Wait for all 3 moves to complete
                let schieber_done = !self.is_axis_moving(axes::SCHIEBER);
                let druecker_done = !self.is_axis_moving(axes::DRUECKER);
                let mt_done = !self.is_axis_moving(axes::MT);
                if schieber_done && druecker_done && mt_done {
                    return self.advance_auto_sequence();
                }
            }
        }
        false
    }

    /// Advance to next cycle/block/set or finish
    fn advance_auto_sequence(&mut self) -> bool {
        let seq = self.auto_sequence.as_mut().unwrap();
        seq.current_cycle += 1;

        if seq.current_cycle >= auto_positions::CYCLES_PER_BLOCK {
            // Block complete
            seq.current_cycle = 0;
            seq.current_block += 1;

            if seq.current_block >= auto_positions::BLOCKS_PER_SET {
                // Set complete
                seq.current_block = 0;
                seq.current_set += 1;

                if seq.current_set >= seq.total_sets {
                    // ALL DONE
                    tracing::info!("[BbmAutomatikV2] Auto sequence COMPLETE");
                    self.set_ruettelmotor(false);
                    self.set_ampel(false, false, true); // Green = done
                    self.auto_sequence = None;
                    return true;
                }
            }
            // New block: reset MT position
            seq.mt_current_run_pos = auto_positions::MT_RUN;
        }

        self.start_auto_cycle();
        true
    }

    /// Start a single fill cycle (wobble -> schieber -> drücker -> return)
    fn start_auto_cycle(&mut self) {
        let seq = self.auto_sequence.as_ref().unwrap();
        let speed = seq.speed;

        // Start wobble: move schieber +wobble from start
        self.move_to_position_mm(
            axes::SCHIEBER,
            auto_positions::SCHIEBER_START + auto_positions::SCHIEBER_WOBBLE,
            speed.schieber_mm_s,
        );

        self.auto_sequence.as_mut().unwrap().current_step = AutoCycleStep::WobbleOut;
    }

    /// Start auto-sequence with given speed preset and number of sets
    pub fn start_auto_sequence(&mut self, speed_preset: &str, total_sets: u32) {
        // Safety checks
        if !self.are_doors_closed() {
            tracing::warn!("[BbmAutomatikV2] Cannot start: doors not closed");
            return;
        }
        if self.axis_alarm_active.iter().any(|&a| a) {
            tracing::warn!("[BbmAutomatikV2] Cannot start: alarm active");
            return;
        }
        if self.auto_sequence.is_some() {
            tracing::warn!("[BbmAutomatikV2] Cannot start: already running");
            return;
        }

        let speed = match speed_preset {
            "medium" => speed_presets::MEDIUM,
            "fast" => speed_presets::FAST,
            _ => speed_presets::SLOW,
        };

        // Initialize sequence
        self.auto_sequence = Some(AutoSequenceState {
            speed_preset_name: speed_preset.to_string(),
            speed,
            total_sets,
            current_set: 0,
            current_block: 0,
            current_cycle: 0,
            current_step: AutoCycleStep::WaitParallelComplete,
            mt_current_run_pos: auto_positions::MT_RUN,
        });

        // Start: Rüttler on, Ampel gelb (running)
        self.set_ruettelmotor(true);
        self.set_ampel(false, true, false);

        // Move all axes to start positions
        self.move_to_position_mm(axes::MT, auto_positions::MT_RUN, speed.mt_mm_s);
        self.move_to_position_mm(
            axes::SCHIEBER,
            auto_positions::SCHIEBER_START,
            speed.schieber_mm_s,
        );
        self.move_to_position_mm(
            axes::DRUECKER,
            auto_positions::DRUECKER_START,
            speed.druecker_mm_s,
        );

        tracing::info!(
            "[BbmAutomatikV2] Auto sequence started: preset={}, sets={}",
            speed_preset,
            total_sets
        );
        self.emit_state();
    }

    /// Stop auto-sequence and all axes
    pub fn stop_auto_sequence(&mut self) {
        if self.auto_sequence.is_some() {
            self.auto_sequence = None;
            self.stop_all_axes();
            self.set_ruettelmotor(false);
            self.set_ampel(true, false, false); // Red = stopped
            tracing::info!("[BbmAutomatikV2] Auto sequence stopped by user");
            self.emit_state();
        }
    }

    // ============ Teach / Calibration ============

    /// Default placeholder name when a custom slot is saved for the first time.
    fn default_custom_name(slot: TeachSlot) -> &'static str {
        match slot {
            TeachSlot::Custom1 => "Position 1",
            TeachSlot::Custom2 => "Position 2",
            _ => "",
        }
    }

    /// Teach-in: capture the current axis position into the given slot. For
    /// Custom1/Custom2 the existing name is kept; if the slot was empty a
    /// default name ("Position 1" / "Position 2") is assigned.
    pub fn save_teach_position(&mut self, axis: usize, slot: TeachSlot) {
        if axis >= self.axes.len() {
            return;
        }
        let pos_mm = self.current_logical_mm(axis);
        let pos_mm = (pos_mm * 1000.0).round() / 1000.0; // 1µm precision

        let t = &mut self.teach_positions[axis];
        match slot {
            TeachSlot::Start => t.start_mm = Some(pos_mm),
            TeachSlot::Ziel => t.ziel_mm = Some(pos_mm),
            TeachSlot::Custom1 | TeachSlot::Custom2 => {
                let slot_ref = match slot {
                    TeachSlot::Custom1 => &mut t.custom1,
                    TeachSlot::Custom2 => &mut t.custom2,
                    _ => unreachable!(),
                };
                match slot_ref {
                    Some(existing) => existing.position_mm = pos_mm,
                    None => {
                        *slot_ref = Some(NamedTeachPosition {
                            name: Self::default_custom_name(slot).to_string(),
                            position_mm: pos_mm,
                        });
                    }
                }
            }
        }

        tracing::info!(
            "[BbmAutomatikV2] Teach axis {} slot {:?} = {:.3} mm",
            axis,
            slot,
            pos_mm
        );
        calibration::save(&self.teach_positions);
        self.emit_state();
    }

    /// Clear a teach slot (sets it back to None / removes the value).
    pub fn clear_teach_position(&mut self, axis: usize, slot: TeachSlot) {
        if axis >= self.teach_positions.len() {
            return;
        }
        let t = &mut self.teach_positions[axis];
        match slot {
            TeachSlot::Start => t.start_mm = None,
            TeachSlot::Ziel => t.ziel_mm = None,
            TeachSlot::Custom1 => t.custom1 = None,
            TeachSlot::Custom2 => t.custom2 = None,
        }
        tracing::info!("[BbmAutomatikV2] Cleared axis {} slot {:?}", axis, slot);
        calibration::save(&self.teach_positions);
        self.emit_state();
    }

    /// Rename a custom slot. Ignored for Start/Ziel (their names are fixed)
    /// and ignored if the custom slot is empty.
    pub fn rename_custom_teach_position(
        &mut self,
        axis: usize,
        slot: TeachSlot,
        name: String,
    ) {
        if axis >= self.teach_positions.len() {
            return;
        }
        let t = &mut self.teach_positions[axis];
        let trimmed = name.trim();
        // Reject empty / oversized names so the UI can't make the slot
        // unselectable.
        if trimmed.is_empty() || trimmed.len() > 32 {
            tracing::warn!(
                "[BbmAutomatikV2] Reject rename axis {} slot {:?}: invalid name length",
                axis,
                slot
            );
            return;
        }
        let slot_ref = match slot {
            TeachSlot::Custom1 => &mut t.custom1,
            TeachSlot::Custom2 => &mut t.custom2,
            _ => {
                tracing::warn!(
                    "[BbmAutomatikV2] Cannot rename non-custom slot {:?}",
                    slot
                );
                return;
            }
        };
        match slot_ref {
            Some(p) => p.name = trimmed.to_string(),
            None => {
                tracing::warn!(
                    "[BbmAutomatikV2] Cannot rename axis {} slot {:?}: not yet saved",
                    axis,
                    slot
                );
                return;
            }
        }
        calibration::save(&self.teach_positions);
        self.emit_state();
    }

    /// Drive to a saved teach position. No-op if slot is empty.
    pub fn goto_teach_position(&mut self, axis: usize, slot: TeachSlot, speed_mm_s: f32) {
        if axis >= self.teach_positions.len() {
            return;
        }
        let t = &self.teach_positions[axis];
        let pos = match slot {
            TeachSlot::Start => t.start_mm,
            TeachSlot::Ziel => t.ziel_mm,
            TeachSlot::Custom1 => t.custom1.as_ref().map(|p| p.position_mm),
            TeachSlot::Custom2 => t.custom2.as_ref().map(|p| p.position_mm),
        };
        let pos = match pos {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "[BbmAutomatikV2] Cannot go to axis {} slot {:?}: empty",
                    axis,
                    slot
                );
                return;
            }
        };
        tracing::info!(
            "[BbmAutomatikV2] Goto axis {} slot {:?} -> {:.3} mm at {:.1} mm/s",
            axis,
            slot,
            pos,
            speed_mm_s
        );
        self.move_to_position_mm(axis, pos, speed_mm_s);
    }
}
