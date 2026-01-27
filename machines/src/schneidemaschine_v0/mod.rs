use crate::machine_identification::{MachineIdentification, MachineIdentificationUnique};
use crate::schneidemaschine_v0::api::{LiveValuesEvent, SchneidemaschineV0Events, StateEvent};
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
    pub const DIGITAL_INPUT_1: u16 = 1;  // EL1008 #1
    pub const DIGITAL_INPUT_2: u16 = 2;  // EL1008 #2
    pub const DIGITAL_OUTPUT: u16 = 3;   // EL2008
    pub const PTO_1: u16 = 4;            // EL2522 #1
    pub const PTO_2: u16 = 5;            // EL2522 #2
    pub const PTO_3: u16 = 6;            // EL2522 #3
    pub const PTO_4: u16 = 7;            // EL2522 #4
    pub const PTO_5: u16 = 8;            // EL2522 #5
}

#[derive(Debug)]
pub struct SchneidemaschineV0 {
    pub api_receiver: Receiver<MachineMessage>,
    pub api_sender: Sender<MachineMessage>,
    pub machine_identification_unique: MachineIdentificationUnique,
    pub namespace: SchneidemaschineV0Namespace,
    pub last_state_emit: Instant,
    pub main_sender: Option<Sender<AsyncThreadMessage>>,

    // Digital Inputs (2x EL1008 = 16 inputs)
    pub digital_inputs: [DigitalInput; 16],

    // Digital Outputs (1x EL2008 = 8 outputs)
    pub digital_outputs: [DigitalOutput; 8],
    pub output_states: [bool; 8],

    // Pulse Train Outputs (5x EL2522 = 10 channels)
    pub axes: [PulseTrainOutput; 10],
    pub axis_speeds: [i32; 10],
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
        }
    }

    /// Get live values (sensor readings, positions)
    pub fn get_live_values(&self) -> LiveValuesEvent {
        // Read digital inputs
        let mut input_states = [false; 16];
        for (i, di) in self.digital_inputs.iter().enumerate() {
            input_states[i] = di.get_value().unwrap_or(false);
        }

        // Read axis positions from PTO feedback
        let mut positions = [0u32; 10];
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
}
