use crate::bbm_automatik_v2::api::BbmAutomatikV2Namespace;
use crate::bbm_automatik_v2::roles;
use crate::bbm_automatik_v2::BbmAutomatikV2;
use smol::block_on;
use std::time::Instant;

use crate::{
    get_ethercat_device, validate_no_role_dublicates, validate_same_machine_identification_unique,
    MachineNewHardware, MachineNewParams, MachineNewTrait,
};

use anyhow::Error;
use ethercat_hal::coe::ConfigurableDevice;
use ethercat_hal::devices::el1008::{EL1008Port, EL1008, EL1008_IDENTITY_A};
use ethercat_hal::devices::el2008::{EL2008Port, EL2008, EL2008_IDENTITY_A, EL2008_IDENTITY_B};
use ethercat_hal::devices::el2522::{
    EL2522ChannelConfiguration, EL2522Configuration, EL2522OperatingMode, EL2522Port, EL2522,
    EL2522_IDENTITY_A,
};
use ethercat_hal::io::digital_input::DigitalInput;
use ethercat_hal::io::digital_output::DigitalOutput;
use ethercat_hal::io::pulse_train_output::PulseTrainOutput;

impl MachineNewTrait for BbmAutomatikV2 {
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
                    "[{}::MachineNewTrait/BbmAutomatikV2::new] MachineNewHardware is not Ethercat",
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

            // ========== Pulse Train Outputs #1 (1x EL2522) ==========
            // Channel 1: MT (Magazin Transporter) - Linear
            // Channel 2: Schieber - Linear
            let (el2522_1, subdevice_1, subdevice_index_1) = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_1,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?;

            // Configure EL2522 #1 for both channels - Hardware ramp enabled
            let el2522_1_config = EL2522Configuration {
                channel1_configuration: EL2522ChannelConfiguration {
                    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
                    ramp_function_active: true,
                    direct_input_mode: true,
                    base_frequency_1: 5000,
                    frequency_factor: 100,
                    travel_distance_control: true,
                    watchdog_timer_deactive: true,
                    ramp_time_constant_rising: 2500,
                    ramp_time_constant_falling: 2250,
                    ..Default::default()
                },
                channel2_configuration: EL2522ChannelConfiguration {
                    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
                    ramp_function_active: true,
                    direct_input_mode: true,
                    base_frequency_1: 5000,
                    frequency_factor: 100,
                    travel_distance_control: true,
                    watchdog_timer_deactive: true,
                    ramp_time_constant_rising: 2500,
                    ramp_time_constant_falling: 2250,
                    ..Default::default()
                },
                ..Default::default()
            };

            el2522_1
                .write()
                .await
                .write_config(&subdevice_1, &el2522_1_config)
                .await?;

            tracing::info!("[BbmAutomatikV2] EL2522 #1 configured: Ch1=MT, Ch2=Schieber");

            // ========== Pulse Train Outputs #2 (1x EL2522) ==========
            // Channel 1: Drücker - Linear
            // Channel 2: Bürste - Rotation
            let (el2522_2, subdevice_2, subdevice_index_2) = get_ethercat_device::<EL2522>(
                hardware,
                params,
                roles::PTO_2,
                [EL2522_IDENTITY_A].to_vec(),
            )
            .await?;

            // Configure EL2522 #2 - Hardware ramp enabled
            let el2522_2_config = EL2522Configuration {
                // Channel 1: Drücker (Linear)
                channel1_configuration: EL2522ChannelConfiguration {
                    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
                    ramp_function_active: true,
                    direct_input_mode: true,
                    base_frequency_1: 5000,
                    frequency_factor: 100,
                    travel_distance_control: true,
                    watchdog_timer_deactive: true,
                    ramp_time_constant_rising: 2500,
                    ramp_time_constant_falling: 2250,
                    ..Default::default()
                },
                // Channel 2: Bürste (Rotation) - no position control needed
                channel2_configuration: EL2522ChannelConfiguration {
                    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
                    ramp_function_active: true,
                    direct_input_mode: true,
                    base_frequency_1: 5000,
                    frequency_factor: 100,
                    travel_distance_control: false, // No position control for rotation
                    watchdog_timer_deactive: true,
                    ramp_time_constant_rising: 2500,
                    ramp_time_constant_falling: 2250,
                    ..Default::default()
                },
                ..Default::default()
            };

            el2522_2
                .write()
                .await
                .write_config(&subdevice_2, &el2522_2_config)
                .await?;

            tracing::info!("[BbmAutomatikV2] EL2522 #2 configured: Ch1=Drücker, Ch2=Bürste");

            // Create PulseTrainOutput array for 4 axes
            let axes = [
                PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO1), // MT
                PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO2), // Schieber
                PulseTrainOutput::new(el2522_2.clone(), EL2522Port::PTO1), // Drücker
                PulseTrainOutput::new(el2522_2.clone(), EL2522Port::PTO2), // Bürste
            ];

            let (sender, receiver) = smol::channel::unbounded();
            let mut machine = Self {
                api_receiver: receiver,
                api_sender: sender,
                machine_identification_unique: params.get_machine_identification_unique(),
                namespace: BbmAutomatikV2Namespace {
                    namespace: params.namespace.clone(),
                },
                last_state_emit: Instant::now(),
                main_sender: params.main_thread_channel.clone(),
                digital_inputs,
                digital_outputs,
                output_states: [false; 8],
                axes,
                axis_speeds: [0; 4],
                axis_target_speeds: [0; 4],
                axis_accelerations: [100.0; 4], // Default: 100 mm/s²
                axis_target_positions: [0; 4],
                axis_position_mode: [false; 4],
                sdo_write_u16: params.sdo_write_u16.clone(),
                pto_subdevice_indices: [subdevice_index_1, subdevice_index_2],
                axis_homing_phase: [
                    crate::bbm_automatik_v2::HomingPhase::Idle,
                    crate::bbm_automatik_v2::HomingPhase::Idle,
                    crate::bbm_automatik_v2::HomingPhase::Idle,
                    crate::bbm_automatik_v2::HomingPhase::Idle,
                ],
                axis_homing_retract_target: [0; 4],
            };

            machine.emit_state();
            Ok(machine)
        })
    }
}
