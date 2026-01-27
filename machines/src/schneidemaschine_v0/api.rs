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

/// Events emitted by the machine
pub enum SchneidemaschineV0Events {
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
    /// Set speed for a single axis
    SetAxisSpeed { index: usize, speed: i32 },
    /// Stop all axes
    StopAllAxes,
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
            Mutation::StopAllAxes => self.stop_all_axes(),
        }
        Ok(())
    }

    fn api_event_namespace(&mut self) -> Option<Namespace> {
        self.namespace.namespace.clone()
    }
}
