use super::SchneidemaschineV0;
use crate::{MachineAct, MachineMessage, MachineValues};
use std::time::{Duration, Instant};

/// Debug log interval (1 second)
const DEBUG_LOG_INTERVAL: Duration = Duration::from_secs(1);

impl MachineAct for SchneidemaschineV0 {
    fn act(&mut self, now: Instant) {
        // Process incoming messages
        if let Ok(msg) = self.api_receiver.try_recv() {
            self.act_machine_message(msg);
        }

        // Simple IO logic: DI1 -> DO1 (press = output on)
        let input_pressed = self.digital_inputs[0].get_value().unwrap_or(false);
        if input_pressed != self.output_states[0] {
            self.set_output(0, input_pressed);
        }

        // Emit state and live values at ~30 Hz
        if now.duration_since(self.last_state_emit) > Duration::from_secs_f64(1.0 / 30.0) {
            self.emit_live_values();
            // Also emit debug info for PTO channel 2 (the active one)
            self.emit_debug_pto(1);
            self.last_state_emit = now;
        }

        // Periodic debug log to console (every 1 second when axis is moving)
        if self.axis_speeds[1] != 0 {
            static mut LAST_DEBUG: Option<Instant> = None;
            let should_log = unsafe {
                match LAST_DEBUG {
                    Some(last) => now.duration_since(last) > DEBUG_LOG_INTERVAL,
                    None => true,
                }
            };
            if should_log {
                unsafe {
                    LAST_DEBUG = Some(now);
                }
                let debug = self.get_debug_pto(1);
                tracing::info!(
                    "[PTO2] freq={}Hz pos={}p ({:.1}mm) ramp={} err={}",
                    debug.frequency_setpoint_hz,
                    debug.actual_position_pulses,
                    debug.actual_position_mm,
                    debug.ramp_active,
                    debug.error
                );
            }
        }
    }

    fn act_machine_message(&mut self, msg: MachineMessage) {
        match msg {
            MachineMessage::SubscribeNamespace(namespace) => {
                self.namespace.namespace = Some(namespace);
                self.emit_state();
                self.emit_live_values();
            }
            MachineMessage::UnsubscribeNamespace => {
                self.namespace.namespace = None;
            }
            MachineMessage::HttpApiJsonRequest(value) => {
                use crate::MachineApi;
                let _res = self.api_mutate(value);
            }
            MachineMessage::ConnectToMachine(_machine_connection) => {
                // Does not connect to other machines; do nothing
            }
            MachineMessage::DisconnectMachine(_machine_connection) => {
                // Does not connect to other machines; do nothing
            }
            MachineMessage::RequestValues(sender) => {
                sender
                    .send_blocking(MachineValues {
                        state: serde_json::to_value(self.get_state())
                            .expect("Failed to serialize state"),
                        live_values: serde_json::to_value(self.get_live_values())
                            .expect("Failed to serialize live values"),
                    })
                    .expect("Failed to send values");
                sender.close();
            }
        }
    }
}
