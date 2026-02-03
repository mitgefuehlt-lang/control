use super::BbmAutomatikV2;
use crate::{MachineAct, MachineMessage, MachineValues};
use std::time::{Duration, Instant};

/// Debug log interval (1 second)
const DEBUG_LOG_INTERVAL: Duration = Duration::from_secs(1);

impl MachineAct for BbmAutomatikV2 {
    fn act(&mut self, now: Instant) {
        // Process incoming messages
        if let Ok(msg) = self.api_receiver.try_recv() {
            self.act_machine_message(msg);
        }

        // Software ramp: update speeds towards targets based on acceleration
        let dt = now.duration_since(self.last_ramp_update).as_secs_f32();
        if dt > 0.001 {
            // At least 1ms passed
            let speed_changed = self.update_software_ramp(dt);
            self.last_ramp_update = now;

            // Emit state when speed changes during ramping
            if speed_changed {
                self.emit_state();
            }
        }

        // Emit state and live values at ~30 Hz
        if now.duration_since(self.last_state_emit) > Duration::from_secs_f64(1.0 / 30.0) {
            self.emit_live_values();
            self.last_state_emit = now;
        }

        // Periodic debug log to console (every 1 second when any axis is moving)
        let any_axis_moving = self.axis_speeds.iter().any(|&s| s != 0);
        if any_axis_moving {
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
                // Log info for moving axes
                let axis_names = ["MT", "Schieber", "Drücker", "Bürste"];
                for (i, &speed) in self.axis_speeds.iter().enumerate() {
                    if speed != 0 {
                        let pos = self.axes[i].get_position();
                        tracing::info!(
                            "[BBM {}] freq={}Hz pos={}p",
                            axis_names[i],
                            speed,
                            pos
                        );
                    }
                }
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
