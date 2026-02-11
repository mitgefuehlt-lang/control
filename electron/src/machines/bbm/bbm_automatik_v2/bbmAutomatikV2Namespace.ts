/**
 * @file bbmAutomatikV2Namespace.ts
 * @description TypeScript implementation of BbmAutomatikV2 namespace with Zod schema validation.
 */

import { StoreApi } from "zustand";
import { create } from "zustand";
import { z } from "zod";
import {
  EventHandler,
  eventSchema,
  Event,
  NamespaceId,
  createNamespaceHookImplementation,
  ThrottledStoreUpdater,
} from "../../../client/socketioStore";
import { MachineIdentificationUnique } from "@/machines/types";

// ========== Event Schema Definitions ==========

/**
 * State event schema - controllable values
 * 4 axes (MT, Schieber, Drücker, Bürste)
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
  axis_speeds: z.tuple([z.number(), z.number(), z.number(), z.number()]),
  axis_target_speeds: z.tuple([z.number(), z.number(), z.number(), z.number()]),
  axis_accelerations: z.tuple([z.number(), z.number(), z.number(), z.number()]),
  axis_target_positions: z.tuple([
    z.number(),
    z.number(),
    z.number(),
    z.number(),
  ]),
  axis_position_mode: z.tuple([
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
  ]),
  axis_homing_active: z.tuple([
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
  ]),
  axis_soft_limit_max: z.tuple([
    z.number().nullable(),
    z.number().nullable(),
    z.number().nullable(),
    z.number().nullable(),
  ]),
  axis_alarm_active: z.tuple([
    z.boolean(),
    z.boolean(),
    z.boolean(),
    z.boolean(),
  ]),
  door_interlock_active: z.boolean(),
  auto_running: z.boolean(),
  auto_current_set: z.number(),
  auto_current_block: z.number(),
  auto_current_cycle: z.number(),
  auto_total_sets: z.number(),
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
  axis_positions: z.tuple([z.number(), z.number(), z.number(), z.number()]),
});

// ========== Event Schemas with Wrappers ==========
export const stateEventSchema = eventSchema(stateEventDataSchema);
export const liveValuesEventSchema = eventSchema(liveValuesEventDataSchema);

// ========== Type Inferences ==========
export type StateEventData = z.infer<typeof stateEventDataSchema>;
export type LiveValuesEventData = z.infer<typeof liveValuesEventDataSchema>;
export type StateEvent = z.infer<typeof stateEventSchema>;
export type LiveValuesEvent = z.infer<typeof liveValuesEventSchema>;

export type BbmAutomatikV2NamespaceStore = {
  // State event from server
  state: StateEvent | null;
  defaultState: StateEvent | null;

  // Live values (latest only)
  liveValues: LiveValuesEvent | null;
};

/**
 * Factory function to create a new BbmAutomatikV2 namespace store
 */
export const createBbmAutomatikV2NamespaceStore =
  (): StoreApi<BbmAutomatikV2NamespaceStore> => {
    return create<BbmAutomatikV2NamespaceStore>(() => {
      return {
        state: null,
        defaultState: null,
        liveValues: null,
      };
    });
  };

/**
 * Creates a message handler for BbmAutomatikV2 namespace events
 */
export function bbmAutomatikV2MessageHandler(
  store: StoreApi<BbmAutomatikV2NamespaceStore>,
  throttledUpdater: ThrottledStoreUpdater<BbmAutomatikV2NamespaceStore>,
): EventHandler {
  return (event: Event<any>) => {
    const eventName = event.name;

    const updateStore = (
      updater: (
        state: BbmAutomatikV2NamespaceStore,
      ) => BbmAutomatikV2NamespaceStore,
    ) => {
      throttledUpdater.updateWith(updater);
    };

    try {
      if (eventName === "StateEvent") {
        const stateEvent = stateEventSchema.parse(event);
        console.log("BbmAutomatikV2 StateEvent", stateEvent);
        updateStore((state) => ({
          ...state,
          state: stateEvent,
          // First state is default
          defaultState: state.defaultState ?? stateEvent,
        }));
      } else if (eventName === "LiveValuesEvent") {
        const liveValuesEvent = liveValuesEventSchema.parse(event);
        updateStore((state) => ({
          ...state,
          liveValues: liveValuesEvent,
        }));
      } else {
        // Log unknown events but don't throw error
        console.warn(`BbmAutomatikV2: Unknown event "${eventName}" ignored`);
      }
    } catch (error) {
      console.error(`Unexpected error processing ${eventName} event:`, error);
      throw error;
    }
  };
}

/**
 * Create the BbmAutomatikV2 namespace implementation
 */
const useBbmAutomatikV2NamespaceImplementation =
  createNamespaceHookImplementation<BbmAutomatikV2NamespaceStore>({
    createStore: createBbmAutomatikV2NamespaceStore,
    createEventHandler: bbmAutomatikV2MessageHandler,
  });

export function useBbmAutomatikV2Namespace(
  machine_identification_unique: MachineIdentificationUnique,
): BbmAutomatikV2NamespaceStore {
  const namespaceId: NamespaceId = {
    type: "machine",
    machine_identification_unique,
  };

  return useBbmAutomatikV2NamespaceImplementation(namespaceId);
}
