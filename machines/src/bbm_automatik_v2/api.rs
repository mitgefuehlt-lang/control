use super::BbmAutomatikV2;
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
    pub axis_speeds: [i32; 4],
    pub axis_target_speeds: [i32; 4],
    pub axis_accelerations: [f32; 4],
    pub axis_target_positions: [i32; 4],  // Signed to support negative positions
    pub axis_position_mode: [bool; 4],
    pub axis_homing_active: [bool; 4],
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
    pub axis_positions: [i32; 4],  // Signed to support negative positions
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
    MoveToPosition { index: usize, position_mm: f32, speed_mm_s: f32 },
    /// Stop a single axis
    StopAxis { index: usize },
    /// Stop all axes
    StopAllAxes,
    /// Set Rüttelmotor on/off
    SetRuettelmotor { on: bool },
    /// Set Ampel (traffic light) state
    SetAmpel { rot: bool, gelb: bool, gruen: bool },
    /// Start homing sequence for an axis
    StartHoming { index: usize },
    /// Cancel homing for an axis
    CancelHoming { index: usize },
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
            Mutation::SetAxisSpeedRpm { index, rpm } => {
                self.set_axis_speed_rpm(index, rpm)
            }
            Mutation::SetAxisAcceleration { index, accel_mm_s2 } => {
                self.set_axis_acceleration(index, accel_mm_s2)
            }
            Mutation::MoveToPosition { index, position_mm, speed_mm_s } => {
                self.move_to_position_mm(index, position_mm, speed_mm_s)
            }
            Mutation::StopAxis { index } => self.stop_axis(index),
            Mutation::StopAllAxes => self.stop_all_axes(),
            Mutation::SetRuettelmotor { on } => self.set_ruettelmotor(on),
            Mutation::SetAmpel { rot, gelb, gruen } => self.set_ampel(rot, gelb, gruen),
            Mutation::StartHoming { index } => self.start_homing(index),
            Mutation::CancelHoming { index } => self.cancel_homing(index),
        }
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<Namespace> {
        self.namespace.namespace.clone()
    }
}
