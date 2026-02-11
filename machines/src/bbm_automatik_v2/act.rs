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

        // Driver alarm check (highest priority - like Arduino checkDriverAlarms())
        let alarm_triggered = self.check_driver_alarms();
        if alarm_triggered {
            self.emit_state();
        }

        // Door interlock check (second highest priority)
        let door_triggered = self.check_door_interlock();
        if door_triggered {
            self.emit_state();
        }

        // Hardware monitor: watch hardware status, no timing needed
        let status_changed = self.update_hardware_monitor();
        if status_changed {
            self.emit_state();
        }

        // Check homing status (reference switches)
        self.update_homing();

        // Auto-sequence state machine
        let auto_changed = self.update_auto_sequence();
        if auto_changed {
            self.emit_state();
        }

        // Emit state and live values at ~30 Hz
        if now.duration_since(self.last_state_emit) > Duration::from_secs_f64(1.0 / 30.0) {
            self.emit_live_values();
            self.last_state_emit = now;
        }

        // Periodic debug log to console (every 1 second when any axis is moving)
        let any_axis_moving = self.axis_speeds.iter().any(|&s| s != 0);
        if any_axis_moving {
            let should_log = match self.last_debug_log {
                Some(last) => now.duration_since(last) > DEBUG_LOG_INTERVAL,
                None => true,
            };
            if should_log {
                self.last_debug_log = Some(now);
                // Log info for moving axes
                let axis_names = ["MT", "Schieber", "Drücker", "Bürste"];
                for (i, &speed) in self.axis_speeds.iter().enumerate() {
                    if speed != 0 {
                        let pos = self.axes[i].get_position();
                        tracing::info!("[BBM {}] freq={}Hz pos={}p", axis_names[i], speed, pos);
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
                let state = serde_json::to_value(self.get_state()).unwrap_or_else(|e| {
                    tracing::error!("[BbmAutomatikV2] Failed to serialize state: {}", e);
                    serde_json::Value::Null
                });
                let live_values =
                    serde_json::to_value(self.get_live_values()).unwrap_or_else(|e| {
                        tracing::error!("[BbmAutomatikV2] Failed to serialize live values: {}", e);
                        serde_json::Value::Null
                    });
                let _ = sender.send_blocking(MachineValues { state, live_values });
                sender.close();
            }
        }
    }
}
