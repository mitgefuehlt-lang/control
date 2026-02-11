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

        // Hardware monitor: watch hardware status, no timing needed
        let status_changed = self.update_hardware_monitor();
        if status_changed {
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
                // Log info for the moving axis
                for (i, &speed) in self.axis_speeds.iter().enumerate() {
                    if speed != 0 {
                        let pto_info = self.get_debug_pto(i);
                        tracing::info!(
                            "[Achse{}] freq={}Hz ({:.1}mm/s) pos={}p ({:.1}mm) ramp={} err={}",
                            i + 1,
                            pto_info.frequency_setpoint_hz,
                            pto_info.frequency_setpoint_mm_s,
                            pto_info.actual_position_pulses,
                            pto_info.actual_position_mm,
                            pto_info.ramp_active,
                            pto_info.error
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
                let state = serde_json::to_value(self.get_state()).unwrap_or_else(|e| {
                    tracing::error!("[SchneidemaschineV0] Failed to serialize state: {}", e);
                    serde_json::Value::Null
                });
                let live_values =
                    serde_json::to_value(self.get_live_values()).unwrap_or_else(|e| {
                        tracing::error!(
                            "[SchneidemaschineV0] Failed to serialize live values: {}",
                            e
                        );
                        serde_json::Value::Null
                    });
                let _ = sender.send_blocking(MachineValues { state, live_values });
                sender.close();
            }
        }
    }
}
