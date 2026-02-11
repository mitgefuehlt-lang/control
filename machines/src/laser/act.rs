use super::LaserMachine;
use crate::MachineAct;
use crate::MachineMessage;
use crate::MachineValues;
use std::time::{Duration, Instant};

/// Implements the `MachineAct` trait for the `LaserMachine`.
///
/// # Parameters
/// - `_now_ts`: The current timestamp of type `Instant`.
///
/// # Returns
/// A pinned `Future` that resolves to `()` and is `Send`-safe. The future encapsulates the asynchronous behavior of the `act` method.
///
/// # Description
/// This method is called to perform periodic actions for the `LaserMachine`. Specifically:
/// - It checks if the time elapsed since the last measurement emission exceeds ~33 milliseconds.
/// - If the condition is met, it emits live values at ~30 FPS.
///
/// The method ensures that the diameter value is updated approximately 30 times per second.
///
impl MachineAct for LaserMachine {
    fn act(&mut self, now: Instant) {
        let msg = self.api_receiver.try_recv();
        match msg {
            Ok(msg) => {
                let _res = self.act_machine_message(msg);
            }
            Err(_) => (),
        };
        self.update();

        if self.did_change_state {
            self.emit_state();
        }

        // more than 33ms have passed since last emit (30 "fps" target)
        if now.duration_since(self.last_measurement_emit) > Duration::from_secs_f64(1.0 / 30.0) {
            self.emit_live_values();
            self.last_measurement_emit = now;
        }
    }

    fn act_machine_message(&mut self, msg: MachineMessage) {
        match msg {
            MachineMessage::SubscribeNamespace(namespace) => {
                self.namespace.namespace = Some(namespace);
                self.emit_state();
            }
            MachineMessage::UnsubscribeNamespace => match &mut self.namespace.namespace {
                Some(namespace) => {
                    tracing::info!("UnsubscribeNamespace");
                    namespace.socket_queue_tx.close();
                    namespace.sockets.clear();
                    namespace.events.clear();
                }
                None => {
                    // Already unsubscribed, nothing to do
                }
            },
            MachineMessage::HttpApiJsonRequest(value) => {
                use crate::MachineApi;
                let _res = self.api_mutate(value);
            }
            MachineMessage::ConnectToMachine(_machine_connection) =>
                /*Doesnt connect to any Machine so do nothing*/
                {}
            MachineMessage::DisconnectMachine(_machine_connection) =>
                /*Doesnt connect to any Machine so do nothing*/
                {}
            MachineMessage::RequestValues(sender) => {
                let state = serde_json::to_value(self.get_state()).unwrap_or_else(|e| {
                    tracing::error!("[Laser] Failed to serialize state: {}", e);
                    serde_json::Value::Null
                });
                let live_values =
                    serde_json::to_value(self.get_live_values()).unwrap_or_else(|e| {
                        tracing::error!("[Laser] Failed to serialize live values: {}", e);
                        serde_json::Value::Null
                    });
                let _ = sender.send_blocking(MachineValues { state, live_values });
                sender.close();
            }
        }
    }
}
