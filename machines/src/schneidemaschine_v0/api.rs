use super::SchneidemaschineV0;
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
    pub axis_speeds: [i32; 2],
    pub axis_target_speeds: [i32; 2],
    pub axis_accelerations: [f32; 2],
    pub axis_target_positions: [u32; 2],
    pub axis_position_mode: [bool; 2],
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
    pub axis_positions: [u32; 2],
}

impl LiveValuesEvent {
    pub fn build(&self) -> Event<Self> {
        Event::new("LiveValuesEvent", self.clone())
    }
}

/// Debug event for PTO channel - comprehensive EtherCAT status
#[derive(Serialize, Debug, Clone, Default)]
pub struct DebugPtoEvent {
    pub channel: u8,
    // Output (what we send to the device)
    pub frequency_setpoint_hz: i32,
    pub frequency_setpoint_mm_s: f32,
    pub target_position_pulses: u32,
    pub target_position_mm: f32,
    pub disable_ramp: bool,
    pub set_counter_request: bool,
    pub set_counter_value: u32,
    // Input (feedback from device)
    pub actual_position_pulses: u32,
    pub actual_position_mm: f32,
    pub ramp_active: bool,
    pub error: bool,
    pub sync_error: bool,
    pub counter_overflow: bool,
    pub counter_underflow: bool,
    pub set_counter_done: bool,
    pub input_t: bool,
    pub input_z: bool,
    pub select_end_counter: bool,
}

impl DebugPtoEvent {
    pub fn build(&self) -> Event<Self> {
        Event::new("DebugPtoEvent", self.clone())
    }
}

/// Events emitted by the machine
pub enum SchneidemaschineV0Events {
    State(Event<StateEvent>),
    LiveValues(Event<LiveValuesEvent>),
    DebugPto(Event<DebugPtoEvent>),
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
    /// Set speed for a single axis (in mm/s)
    SetAxisSpeedMmS { index: usize, speed_mm_s: f32 },
    /// Set acceleration for a single axis (in mm/s²)
    SetAxisAcceleration { index: usize, accel_mm_s2: f32 },
    /// Move axis to a target position (in mm) with given speed (mm/s)
    MoveToPosition { index: usize, position_mm: f32, speed_mm_s: f32 },
    /// Stop all axes
    StopAllAxes,
    /// Request debug info for a PTO channel (emits DebugPtoEvent)
    DebugPto { index: usize },
    /// Log all debug info to server console
    DebugLogAll,
}

#[derive(Debug, Clone)]
pub struct SchneidemaschineV0Namespace {
    pub namespace: Option<Namespace>,
}

impl NamespaceCacheingLogic<SchneidemaschineV0Events> for SchneidemaschineV0Namespace {
    fn emit(&mut self, events: SchneidemaschineV0Events) {
        let event = Arc::new(events.event_value());
        let buffer_fn = events.event_cache_fn();
        if let Some(ns) = &mut self.namespace {
            ns.emit(event, &buffer_fn);
        }
    }
}

impl CacheableEvents<SchneidemaschineV0Events> for SchneidemaschineV0Events {
    fn event_value(&self) -> GenericEvent {
        match self {
            Self::State(event) => event.clone().into(),
            Self::LiveValues(event) => event.clone().into(),
            Self::DebugPto(event) => event.clone().into(),
        }
    }

    fn event_cache_fn(&self) -> CacheFn {
        cache_first_and_last_event()
    }
}

impl MachineApi for SchneidemaschineV0 {
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
            Mutation::SetAxisAcceleration { index, accel_mm_s2 } => {
                self.set_axis_acceleration(index, accel_mm_s2)
            }
            Mutation::MoveToPosition { index, position_mm, speed_mm_s } => {
                self.move_to_position_mm(index, position_mm, speed_mm_s)
            }
            Mutation::StopAllAxes => self.stop_all_axes(),
            Mutation::DebugPto { index } => self.emit_debug_pto(index),
            Mutation::DebugLogAll => self.log_debug_all(),
        }
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<Namespace> {
        self.namespace.namespace.clone()
    }
}
