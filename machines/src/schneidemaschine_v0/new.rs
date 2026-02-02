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
use ethercat_hal::coe::ConfigurableDevice;
use ethercat_hal::devices::el1008::{EL1008, EL1008Port, EL1008_IDENTITY_A};
use ethercat_hal::devices::el2008::{EL2008, EL2008Port, EL2008_IDENTITY_A, EL2008_IDENTITY_B};
use ethercat_hal::devices::el2522::{
    EL2522, EL2522ChannelConfiguration, EL2522Configuration, EL2522OperatingMode, EL2522Port,
    EL2522_IDENTITY_A,
};
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
            // ========== Digital Inputs (1x EL1008) ==========
            let el1008 = get_ethercat_device::<EL1008>(
                hardware,
                params,
                roles::DIGITAL_INPUT,
                [EL1008_IDENTITY_A].to_vec(),
            )
            .await?
            .0;

            // Create DigitalInput array for 8 inputs
            let digital_inputs = [
                DigitalInput::new(el1008.clone(), EL1008Port::DI1),
                DigitalInput::new(el1008.clone(), EL1008Port::DI2),
                DigitalInput::new(el1008.clone(), EL1008Port::DI3),
                DigitalInput::new(el1008.clone(), EL1008Port::DI4),
                DigitalInput::new(el1008.clone(), EL1008Port::DI5),
                DigitalInput::new(el1008.clone(), EL1008Port::DI6),
                DigitalInput::new(el1008.clone(), EL1008Port::DI7),
                DigitalInput::new(el1008.clone(), EL1008Port::DI8),
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

            // ========== Pulse Train Outputs (1x EL2522) ==========
            let (el2522, subdevice) = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?;

            // Configure EL2522 for Pulse+Direction mode on Channel 2
            // Mechanics: 200 pulses/rev, 10mm lead -> 20 pulses/mm
            // Software ramping is used for acceleration control (hardware ramp disabled)
            let el2522_config = EL2522Configuration {
                // Channel 1: Default (not used currently)
                channel1_configuration: EL2522ChannelConfiguration::default(),
                // Channel 2: CL57T Stepper Driver - Pulse+Direction
                channel2_configuration: EL2522ChannelConfiguration {
                    // Pulse + Direction mode for stepper driver
                    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
                    // Disable hardware ramp - we use software ramping for dynamic acceleration
                    ramp_function_active: false,
                    // Direct frequency input (Hz value directly controls frequency)
                    direct_input_mode: true,
                    // Base frequency 1: Max frequency in Hz
                    // 230 mm/s * 20 pulses/mm = 4600 Hz, use 5000 Hz as base
                    base_frequency_1: 5000,
                    // Frequency factor for direct input mode (100 = 1:1)
                    frequency_factor: 100,
                    // Enable travel distance control for position targeting
                    travel_distance_control: true,
                    // Disable watchdog for testing
                    watchdog_timer_deactive: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            // Write configuration to EL2522 via CoE (SDO)
            el2522
                .write()
                .await
                .write_config(&subdevice, &el2522_config)
                .await?;

            tracing::info!(
                "[SchneidemaschineV0] EL2522 configured: Channel 2 = PulseDirection mode, base_freq=5000Hz, software ramp enabled"
            );

            // Create PulseTrainOutput array for 2 axes
            let axes = [
                PulseTrainOutput::new(el2522.clone(), EL2522Port::PTO1),
                PulseTrainOutput::new(el2522.clone(), EL2522Port::PTO2),
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
                axis_speeds: [0; 2],
                axis_target_speeds: [0; 2],
                axis_accelerations: [100.0; 2],  // Default: 100 mm/s²
                axis_target_positions: [0; 2],
                axis_position_mode: [false; 2],
                last_ramp_update: Instant::now(),
            };

            machine.emit_state();
            Ok(machine)
        })
    }
}
