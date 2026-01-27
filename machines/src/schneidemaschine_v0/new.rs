use crate::schneidemaschine_v0::api::SchneidemaschineV0Namespace;
use crate::schneidemaschine_v0::roles;
use crate::schneidemaschine_v0::SchneidemaschineV0;
use smol::block_on;
use std::time::Instant;

use crate::{
    MachineNewHardware, MachineNewParams, MachineNewTrait, get_ethercat_device,
    validate_no_role_dublicates, validate_same_machine_identification_unique,
};

use anyhow::Error;
use ethercat_hal::devices::el1008::{EL1008, EL1008Port, EL1008_IDENTITY_A};
use ethercat_hal::devices::el2008::{EL2008, EL2008Port, EL2008_IDENTITY_A, EL2008_IDENTITY_B};
use ethercat_hal::devices::el2522::{EL2522, EL2522Port, EL2522_IDENTITY_A};
use ethercat_hal::io::digital_input::DigitalInput;
use ethercat_hal::io::digital_output::DigitalOutput;
use ethercat_hal::io::pulse_train_output::PulseTrainOutput;

impl MachineNewTrait for SchneidemaschineV0 {
    fn new<'maindevice>(params: &MachineNewParams) -> Result<Self, Error> {
        // Validate general stuff
        let device_identification = params
            .device_group
            .iter()
            .map(|device_identification| device_identification.clone())
            .collect::<Vec<_>>();
        validate_same_machine_identification_unique(&device_identification)?;
        validate_no_role_dublicates(&device_identification)?;

        let hardware = match &params.hardware {
            MachineNewHardware::Ethercat(x) => x,
            _ => {
                return Err(anyhow::anyhow!(
                    "[{}::MachineNewTrait/SchneidemaschineV0::new] MachineNewHardware is not Ethercat",
                    module_path!()
                ));
            }
        };

        block_on(async {
            // ========== Digital Inputs (2x EL1008) ==========
            let el1008_1 = get_ethercat_device::<EL1008>(
                hardware,
                params,
                roles::DIGITAL_INPUT_1,
                [EL1008_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            let el1008_2 = get_ethercat_device::<EL1008>(
                hardware,
                params,
                roles::DIGITAL_INPUT_2,
                [EL1008_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            // Create DigitalInput array for all 16 inputs
            let digital_inputs = [
                // EL1008 #1 (DI 1-8)
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI1),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI2),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI3),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI4),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI5),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI6),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI7),
                DigitalInput::new(el1008_1.clone(), EL1008Port::DI8),
                // EL1008 #2 (DI 9-16)
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI1),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI2),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI3),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI4),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI5),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI6),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI7),
                DigitalInput::new(el1008_2.clone(), EL1008Port::DI8),
            ];

            // ========== Digital Outputs (1x EL2008) ==========
            let el2008 = get_ethercat_device::<EL2008>(
                hardware,
                params,
                roles::DIGITAL_OUTPUT,
                [EL2008_IDENTITY_A, EL2008_IDENTITY_B].to_vec(),
            )
            .await?
            .0;

            let digital_outputs = [
                DigitalOutput::new(el2008.clone(), EL2008Port::DO1),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO2),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO3),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO4),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO5),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO6),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO7),
                DigitalOutput::new(el2008.clone(), EL2008Port::DO8),
            ];

            // ========== Pulse Train Outputs (5x EL2522) ==========
            let el2522_1 = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_1,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            let el2522_2 = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_2,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            let el2522_3 = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_3,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            let el2522_4 = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_4,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            let el2522_5 = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_5,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            // Create PulseTrainOutput array for all 10 axes
            let axes = [
                // EL2522 #1 (Axis 1-2)
                PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO2),
                // EL2522 #2 (Axis 3-4)
                PulseTrainOutput::new(el2522_2.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522_2.clone(), EL2522Port::PTO2),
                // EL2522 #3 (Axis 5-6)
                PulseTrainOutput::new(el2522_3.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522_3.clone(), EL2522Port::PTO2),
                // EL2522 #4 (Axis 7-8)
                PulseTrainOutput::new(el2522_4.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522_4.clone(), EL2522Port::PTO2),
                // EL2522 #5 (Axis 9-10)
                PulseTrainOutput::new(el2522_5.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522_5.clone(), EL2522Port::PTO2),
            ];

            let (sender, receiver) = smol::channel::unbounded();
            let mut machine = Self {
                api_receiver: receiver,
                api_sender: sender,
                machine_identification_unique: params.get_machine_identification_unique(),
                namespace: SchneidemaschineV0Namespace {
                    namespace: params.namespace.clone(),
                },
                last_state_emit: Instant::now(),
                main_sender: params.main_thread_channel.clone(),
                digital_inputs,
                digital_outputs,
                output_states: [false; 8],
                axes,
                axis_speeds: [0; 10],
            };

            machine.emit_state();
            Ok(machine)
        })
    }
}
