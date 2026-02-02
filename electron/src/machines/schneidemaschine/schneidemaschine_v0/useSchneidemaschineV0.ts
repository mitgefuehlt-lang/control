import { toastError } from "@/components/Toast";
import { useMachineMutate as useMachineMutation } from "@/client/useClient";
import { MachineIdentificationUnique } from "@/machines/types";
import { schneidemaschineV0 } from "@/machines/properties";
import { schneidemaschineV0SerialRoute } from "@/routes/routes";
import { z } from "zod";
import {
  useSchneidemaschineV0Namespace,
  StateEvent,
} from "./schneidemaschineV0Namespace";
import { useEffect, useMemo } from "react";
import { useStateOptimistic } from "@/lib/useStateOptimistic";
import { produce } from "immer";

function useSchneidemaschine(
  machine_identification_unique: MachineIdentificationUnique,
) {
  const { state, defaultState, liveValues } = useSchneidemaschineV0Namespace(
    machine_identification_unique,
  );

  // Optimistic state management
  const stateOptimistic = useStateOptimistic<StateEvent>();

  useEffect(() => {
    if (state) {
      stateOptimistic.setReal(state);
    }
  }, [state]);

  // Helper function for optimistic updates
  const updateStateOptimistically = (
    producer: (current: StateEvent) => void,
    serverRequest: () => Promise<any>,
  ) => {
    const currentState = stateOptimistic.value;
    if (currentState && !stateOptimistic.isOptimistic) {
      stateOptimistic.setOptimistic(produce(currentState, producer));
    }
    serverRequest()
      .then((response) => {
        if (!response.success) stateOptimistic.resetToReal();
      })
      .catch(() => stateOptimistic.resetToReal());
  };

  // SetOutput mutation - matches Rust: #[serde(tag = "action", content = "value")]
  // Format: { "action": "SetOutput", "value": { "index": 0, "on": true } }
  const setOutputSchema = z.object({
    action: z.literal("SetOutput"),
    value: z.object({
      index: z.number(),
      on: z.boolean(),
    }),
  });
  const { request: requestSetOutput } = useMachineMutation(setOutputSchema);

  const setOutput = (index: number, on: boolean) => {
    updateStateOptimistically(
      (current) => {
        if (current.data.output_states && index >= 0 && index < 8) {
          current.data.output_states[index] = on;
        }
      },
      () =>
        requestSetOutput({
          machine_identification_unique,
          data: {
            action: "SetOutput",
            value: { index, on },
          },
        }),
    );
  };

  // Toggle helper
  const toggleOutput = (index: number) => {
    const currentValue = stateOptimistic.value?.data.output_states[index];
    if (currentValue !== undefined) {
      setOutput(index, !currentValue);
    }
  };

  return {
    // State
    state: stateOptimistic.value?.data,
    defaultState: defaultState?.data,

    // Live values
    liveValues: liveValues?.data,

    // Loading states
    isLoading: stateOptimistic.isOptimistic,
    isDisabled: !stateOptimistic.isInitialized,

    // Actions
    setOutput,
    toggleOutput,
  };
}

export function useSchneidemaschineV0() {
  const { serial: serialString } = schneidemaschineV0SerialRoute.useParams();

  const machineIdentification: MachineIdentificationUnique = useMemo(() => {
    const serial = parseInt(serialString);

    if (isNaN(serial)) {
      toastError(
        "Invalid Serial Number",
        `"${serialString}" is not a valid serial number.`,
      );

      return {
        machine_identification: {
          vendor: 0,
          machine: 0,
        },
        serial: 0,
      };
    }

    return {
      machine_identification: schneidemaschineV0.machine_identification,
      serial,
    };
  }, [serialString]);

  return useSchneidemaschine(machineIdentification);
}
