/**
 * @file schneidemaschineV0Namespace.ts
 * @description TypeScript implementation of SchneidemaschineV0 namespace with Zod schema validation.
 */

import { StoreApi } from "zustand";
import { create } from "zustand";
import { z } from "zod";
import {
  EventHandler,
  eventSchema,
  Event,
  handleUnhandledEventError,
  NamespaceId,
  createNamespaceHookImplementation,
  ThrottledStoreUpdater,
} from "../../../client/socketioStore";
import { MachineIdentificationUnique } from "@/machines/types";

// ========== Event Schema Definitions ==========

/**
 * State event schema - controllable values
 * Note: Backend does NOT send is_default_state for this machine
 */
export const stateEventDataSchema = z.object({
  output_states: z.tuple([
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
  ]),
  axis_speeds: z.tuple([z.number(), z.number()]),
  axis_target_speeds: z.tuple([z.number(), z.number()]),
  axis_accelerations: z.tuple([z.number(), z.number()]),
});

/**
 * Live values event schema - sensor data
 */
export const liveValuesEventDataSchema = z.object({
  input_states: z.tuple([
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
  ]),
  axis_positions: z.tuple([z.number(), z.number()]),
});

// ========== Event Schemas with Wrappers ==========
export const stateEventSchema = eventSchema(stateEventDataSchema);
export const liveValuesEventSchema = eventSchema(liveValuesEventDataSchema);

// ========== Type Inferences ==========
export type StateEventData = z.infer<typeof stateEventDataSchema>;
export type LiveValuesEventData = z.infer<typeof liveValuesEventDataSchema>;
export type StateEvent = z.infer<typeof stateEventSchema>;
export type LiveValuesEvent = z.infer<typeof liveValuesEventSchema>;

export type SchneidemaschineV0NamespaceStore = {
  // State event from server
  state: StateEvent | null;
  defaultState: StateEvent | null;

  // Live values (latest only, no timeseries for now)
  liveValues: LiveValuesEvent | null;
};

/**
 * Factory function to create a new SchneidemaschineV0 namespace store
 */
export const createSchneidemaschineV0NamespaceStore =
  (): StoreApi<SchneidemaschineV0NamespaceStore> => {
    return create<SchneidemaschineV0NamespaceStore>(() => {
      return {
        state: null,
        defaultState: null,
        liveValues: null,
      };
    });
  };

/**
 * Creates a message handler for SchneidemaschineV0 namespace events
 */
export function schneidemaschineV0MessageHandler(
  store: StoreApi<SchneidemaschineV0NamespaceStore>,
  throttledUpdater: ThrottledStoreUpdater<SchneidemaschineV0NamespaceStore>,
): EventHandler {
  return (event: Event<any>) => {
    const eventName = event.name;

    const updateStore = (
      updater: (
        state: SchneidemaschineV0NamespaceStore,
      ) => SchneidemaschineV0NamespaceStore,
    ) => {
      throttledUpdater.updateWith(updater);
    };

    try {
      if (eventName === "StateEvent") {
        const stateEvent = stateEventSchema.parse(event);
        console.log("SchneidemaschineV0 StateEvent", stateEvent);
        updateStore((state) => ({
          ...state,
          state: stateEvent,
          // No is_default_state in this machine, first state is default
          defaultState: state.defaultState ?? stateEvent,
        }));
      } else if (eventName === "LiveValuesEvent") {
        const liveValuesEvent = liveValuesEventSchema.parse(event);
        updateStore((state) => ({
          ...state,
          liveValues: liveValuesEvent,
        }));
      } else if (eventName === "DebugPtoEvent") {
        // Debug event - ignore for now, could be displayed in a debug panel later
        console.log("SchneidemaschineV0 DebugPtoEvent (ignored)", event);
      } else {
        // Log unknown events but don't throw error
        console.warn(`SchneidemaschineV0: Unknown event "${eventName}" ignored`);
      }
    } catch (error) {
      console.error(`Unexpected error processing ${eventName} event:`, error);
      throw error;
    }
  };
}

/**
 * Create the SchneidemaschineV0 namespace implementation
 */
const useSchneidemaschineV0NamespaceImplementation =
  createNamespaceHookImplementation<SchneidemaschineV0NamespaceStore>({
    createStore: createSchneidemaschineV0NamespaceStore,
    createEventHandler: schneidemaschineV0MessageHandler,
  });

export function useSchneidemaschineV0Namespace(
  machine_identification_unique: MachineIdentificationUnique,
): SchneidemaschineV0NamespaceStore {
  const namespaceId: NamespaceId = {
    type: "machine",
    machine_identification_unique,
  };

  return useSchneidemaschineV0NamespaceImplementation(namespaceId);
}
