use super::{AxisTeachPositions, BbmAutomatikV2, TeachSlot};
use crate::{MachineApi, MachineMessage};
use control_core::socketio::{
    event::{Event, GenericEvent},
    namespace::{
        CacheFn, CacheableEvents, Namespace, NamespaceCacheingLogic, cache_first_and_last_event,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// State event - contains controllable values (outputs, speeds)
#[derive(Serialize, Debug, Clone)]
pub struct StateEvent {
    pub output_states: [bool; 8],
    pub axis_speeds: [i32; 3],
    pub axis_target_speeds: [i32; 3],
    pub axis_accelerations: [f32; 3],
    pub axis_target_positions: [i32; 3], // Signed to support negative positions
    pub axis_position_mode: [bool; 3],
    pub axis_homing_active: [bool; 3],
    /// true per axis once homing has completed (revoked on step-loss).
    /// While any axis is false, ALL movement is blocked (global homing gate).
    pub axis_homed: [bool; 3],
    pub axis_soft_limit_max: [Option<f32>; 3],
    pub axis_soft_limit_min: [Option<f32>; 3],
    pub axis_alarm_active: [bool; 3],
    /// true when a TDC move ended >1 mm away from its target — position
    /// integrity lost, axis must be re-homed (clears the flag).
    pub axis_step_loss: [bool; 3],
    pub door_interlock_active: bool,
    /// true when the Schieber is currently blocked (Drücker extended above its
    /// start, interlock A).
    pub schieber_interlock_active: bool,
    /// true when the Drücker is currently blocked (Schieber away from its
    /// start, interlock B).
    pub druecker_interlock_active: bool,
    pub auto_running: bool,
    pub auto_current_set: u32,
    pub auto_current_block: u32,
    pub auto_current_cycle: u32,
    pub auto_total_sets: u32,
    /// Per-axis teach-in positions (persisted to disk)
    pub teach_positions: [AxisTeachPositions; 3],
}

impl StateEvent {
    pub fn build(&self) -> Event<Self> {
        Event::new("StateEvent", self.clone())
    }
}

/// Live values event - contains sensor readings and positions
#[derive(Serialize, Debug, Clone)]
pub struct LiveValuesEvent {
    pub input_states: [bool; 8],
    pub axis_positions: [i32; 3], // Signed to support negative positions
}

impl LiveValuesEvent {
    pub fn build(&self) -> Event<Self> {
        Event::new("LiveValuesEvent", self.clone())
    }
}

/// Events emitted by the machine
pub enum BbmAutomatikV2Events {
    State(Event<StateEvent>),
    LiveValues(Event<LiveValuesEvent>),
}

/// Mutations (commands from UI to machine)
#[derive(Deserialize)]
#[serde(tag = "action", content = "value")]
pub enum Mutation {
    /// Set a single digital output
    SetOutput { index: usize, on: bool },
    /// Set all digital outputs
    SetAllOutputs { on: bool },
    /// Set speed for a single axis (in Hz)
    SetAxisSpeed { index: usize, speed: i32 },
    /// Set speed for a single axis (in mm/s) - for linear axes
    SetAxisSpeedMmS { index: usize, speed_mm_s: f32 },
    /// Set speed for a single axis (in RPM) - for rotation axes
    SetAxisSpeedRpm { index: usize, rpm: f32 },
    /// Set acceleration for a single axis (in mm/s²)
    SetAxisAcceleration { index: usize, accel_mm_s2: f32 },
    /// Move axis to a target position (in mm) with given speed (mm/s)
    MoveToPosition {
        index: usize,
        position_mm: f32,
        speed_mm_s: f32,
    },
    /// Relative jog: move `delta_mm` from current position at given speed.
    /// Uses speed mode (signed direction) and stops in software once the
    /// counter has moved the requested delta. For the +/- JOG buttons.
    JogRelative {
        index: usize,
        delta_mm: f32,
        speed_mm_s: f32,
    },
    /// Stop a single axis
    StopAxis { index: usize },
    /// Stop all axes
    StopAllAxes,
    /// Set Bürstenmotor on/off
    SetBuerstenmotor { on: bool },
    /// Set Rüttelmotor on/off
    SetRuettelmotor { on: bool },
    /// Set Pneumatik valve on/off
    SetPneumatik { on: bool },
    /// Set Schaltschrank-Lüfter on/off
    SetLuefter { on: bool },
    /// Set Ampel (traffic light) state
    SetAmpel { rot: bool, gelb: bool, gruen: bool },
    /// Start homing sequence for an axis
    StartHoming { index: usize },
    /// Cancel homing for an axis
    CancelHoming { index: usize },
    /// Reset all driver alarms
    ResetAlarms,
    /// Start auto-sequence with speed preset and number of sets
    StartAutoSequence { speed_preset: String, total_sets: u32 },
    /// Stop auto-sequence
    StopAutoSequence,
    /// Capture the current axis position into the given teach slot
    SaveTeachPosition { axis: usize, slot: TeachSlot },
    /// Clear a teach slot (set back to empty)
    ClearTeachPosition { axis: usize, slot: TeachSlot },
    /// Rename a custom teach slot (Custom1 / Custom2 only)
    RenameCustomPosition {
        axis: usize,
        slot: TeachSlot,
        name: String,
    },
    /// Drive axis to a stored teach position
    GoToTeachPosition {
        axis: usize,
        slot: TeachSlot,
        speed_mm_s: f32,
    },
    /// Set (or clear) the upper soft-limit for an axis.
    /// `max_mm: null` removes the limit; otherwise clamps moves to that
    /// logical position. Persisted alongside teach positions.
    SetSoftLimitMax {
        axis: usize,
        max_mm: Option<f32>,
    },
    /// Set (or clear) the lower soft-limit for an axis.
    SetSoftLimitMin {
        axis: usize,
        min_mm: Option<f32>,
    },
    /// Teach-in: capture the current axis position as the upper soft-limit.
    TeachSoftLimitMax { axis: usize },
    /// Teach-in: capture the current axis position as the lower soft-limit.
    TeachSoftLimitMin { axis: usize },
}

#[derive(Debug, Clone)]
pub struct BbmAutomatikV2Namespace {
    pub namespace: Option<Namespace>,
}

impl NamespaceCacheingLogic<BbmAutomatikV2Events> for BbmAutomatikV2Namespace {
    fn emit(&mut self, events: BbmAutomatikV2Events) {
        let event = Arc::new(events.event_value());
        let buffer_fn = events.event_cache_fn();
        if let Some(ns) = &mut self.namespace {
            ns.emit(event, &buffer_fn);
        }
    }
}

impl CacheableEvents<BbmAutomatikV2Events> for BbmAutomatikV2Events {
    fn event_value(&self) -> GenericEvent {
        match self {
            Self::State(event) => event.clone().into(),
            Self::LiveValues(event) => event.clone().into(),
        }
    }

    fn event_cache_fn(&self) -> CacheFn {
        cache_first_and_last_event()
    }
}

impl MachineApi for BbmAutomatikV2 {
    fn api_get_sender(&self) -> smol::channel::Sender<MachineMessage> {
        self.api_sender.clone()
    }

    fn api_mutate(&mut self, request_body: Value) -> Result<(), anyhow::Error> {
        let mutation: Mutation = serde_json::from_value(request_body)?;
        match mutation {
            Mutation::SetOutput { index, on } => self.set_output(index, on),
            Mutation::SetAllOutputs { on } => self.set_all_outputs(on),
            Mutation::SetAxisSpeed { index, speed } => self.set_axis_speed(index, speed),
            Mutation::SetAxisSpeedMmS { index, speed_mm_s } => {
                self.set_axis_speed_mm_s(index, speed_mm_s)
            }
            Mutation::SetAxisSpeedRpm { index, rpm } => self.set_axis_speed_rpm(index, rpm),
            Mutation::SetAxisAcceleration { index, accel_mm_s2 } => {
                self.set_axis_acceleration(index, accel_mm_s2)
            }
            Mutation::MoveToPosition {
                index,
                position_mm,
                speed_mm_s,
            } => self.move_to_position_mm(index, position_mm, speed_mm_s),
            Mutation::JogRelative {
                index,
                delta_mm,
                speed_mm_s,
            } => self.jog_relative(index, delta_mm, speed_mm_s),
            Mutation::StopAxis { index } => self.stop_axis(index),
            Mutation::StopAllAxes => self.stop_all_axes(),
            Mutation::SetBuerstenmotor { on } => self.set_buerstenmotor(on),
            Mutation::SetRuettelmotor { on } => self.set_ruettelmotor(on),
            Mutation::SetPneumatik { on } => self.set_pneumatik(on),
            Mutation::SetLuefter { on } => self.set_luefter(on),
            Mutation::SetAmpel { rot, gelb, gruen } => self.set_ampel(rot, gelb, gruen),
            Mutation::StartHoming { index } => self.start_homing(index),
            Mutation::CancelHoming { index } => self.cancel_homing(index),
            Mutation::ResetAlarms => self.reset_alarms(),
            Mutation::StartAutoSequence { speed_preset, total_sets } => {
                self.start_auto_sequence(&speed_preset, total_sets);
            }
            Mutation::StopAutoSequence => self.stop_auto_sequence(),
            Mutation::SaveTeachPosition { axis, slot } => {
                self.save_teach_position(axis, slot)
            }
            Mutation::ClearTeachPosition { axis, slot } => {
                self.clear_teach_position(axis, slot)
            }
            Mutation::RenameCustomPosition { axis, slot, name } => {
                self.rename_custom_teach_position(axis, slot, name)
            }
            Mutation::GoToTeachPosition {
                axis,
                slot,
                speed_mm_s,
            } => self.goto_teach_position(axis, slot, speed_mm_s),
            Mutation::SetSoftLimitMax { axis, max_mm } => {
                self.set_soft_limit_max(axis, max_mm)
            }
            Mutation::SetSoftLimitMin { axis, min_mm } => {
                self.set_soft_limit_min(axis, min_mm)
            }
            Mutation::TeachSoftLimitMax { axis } => self.teach_soft_limit_max(axis),
            Mutation::TeachSoftLimitMin { axis } => self.teach_soft_limit_min(axis),
        }
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<Namespace> {
        self.namespace.namespace.clone()
    }
}
