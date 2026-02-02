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

// Motor constants
const PULSES_PER_MM = 20;
const MAX_SPEED_MM_S = 230;
const DEFAULT_ACCELERATION_MM_S2 = 100;
const MAX_ACCELERATION_MM_S2 = 500;
const MIN_ACCELERATION_MM_S2 = 1;

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

  // SetAxisSpeedMmS mutation
  const setAxisSpeedMmSSchema = z.object({
    action: z.literal("SetAxisSpeedMmS"),
    value: z.object({
      index: z.number(),
      speed_mm_s: z.number(),
    }),
  });
  const { request: requestSetAxisSpeedMmS } =
    useMachineMutation(setAxisSpeedMmSSchema);

  const setAxisSpeedMmS = (index: number, speed_mm_s: number) => {
    updateStateOptimistically(
      (current) => {
        if (current.data.axis_speeds && index >= 0 && index < 2) {
          // Convert mm/s to Hz for optimistic state
          current.data.axis_speeds[index] = Math.round(speed_mm_s * PULSES_PER_MM);
        }
      },
      () =>
        requestSetAxisSpeedMmS({
          machine_identification_unique,
          data: {
            action: "SetAxisSpeedMmS",
            value: { index, speed_mm_s },
          },
        }),
    );
  };

  // StopAllAxes mutation
  const stopAllAxesSchema = z.object({
    action: z.literal("StopAllAxes"),
  });
  const { request: requestStopAllAxes } = useMachineMutation(stopAllAxesSchema);

  const stopAllAxes = () => {
    updateStateOptimistically(
      (current) => {
        current.data.axis_speeds = [0, 0];
      },
      () =>
        requestStopAllAxes({
          machine_identification_unique,
          data: { action: "StopAllAxes" },
        }),
    );
  };

  // Stop single axis helper
  const stopAxis = (index: number) => {
    setAxisSpeedMmS(index, 0);
  };

  // MoveToPosition mutation
  const moveToPositionSchema = z.object({
    action: z.literal("MoveToPosition"),
    value: z.object({
      index: z.number(),
      position_mm: z.number(),
      speed_mm_s: z.number(),
    }),
  });
  const { request: requestMoveToPosition } =
    useMachineMutation(moveToPositionSchema);

  const moveToPosition = (index: number, position_mm: number, speed_mm_s: number) => {
    updateStateOptimistically(
      (current) => {
        if (current.data.axis_target_positions && index >= 0 && index < 2) {
          current.data.axis_target_positions[index] = Math.round(position_mm * PULSES_PER_MM);
          current.data.axis_position_mode[index] = true;
        }
      },
      () =>
        requestMoveToPosition({
          machine_identification_unique,
          data: {
            action: "MoveToPosition",
            value: { index, position_mm, speed_mm_s },
          },
        }),
    );
  };

  // SetAxisAcceleration mutation
  const setAxisAccelerationSchema = z.object({
    action: z.literal("SetAxisAcceleration"),
    value: z.object({
      index: z.number(),
      accel_mm_s2: z.number(),
    }),
  });
  const { request: requestSetAxisAcceleration } =
    useMachineMutation(setAxisAccelerationSchema);

  const setAxisAcceleration = (index: number, accel_mm_s2: number) => {
    updateStateOptimistically(
      (current) => {
        if (current.data.axis_accelerations && index >= 0 && index < 2) {
          current.data.axis_accelerations[index] = accel_mm_s2;
        }
      },
      () =>
        requestSetAxisAcceleration({
          machine_identification_unique,
          data: {
            action: "SetAxisAcceleration",
            value: { index, accel_mm_s2 },
          },
        }),
    );
  };

  // Helper to get axis speed in mm/s
  const getAxisSpeedMmS = (index: number): number | undefined => {
    const speedHz = stateOptimistic.value?.data.axis_speeds[index];
    return speedHz !== undefined ? speedHz / PULSES_PER_MM : undefined;
  };

  // Helper to get axis position in mm
  const getAxisPositionMm = (index: number): number | undefined => {
    const pulses = liveValues?.data.axis_positions[index];
    return pulses !== undefined ? pulses / PULSES_PER_MM : undefined;
  };

  // Helper to get axis acceleration in mm/s²
  const getAxisAcceleration = (index: number): number | undefined => {
    return stateOptimistic.value?.data.axis_accelerations[index];
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

    // Digital output actions
    setOutput,
    toggleOutput,

    // Motor control actions
    setAxisSpeedMmS,
    setAxisAcceleration,
    moveToPosition,
    stopAxis,
    stopAllAxes,

    // Motor helper functions
    getAxisSpeedMmS,
    getAxisPositionMm,
    getAxisAcceleration,

    // Constants
    MAX_SPEED_MM_S,
    MAX_ACCELERATION_MM_S2,
    MIN_ACCELERATION_MM_S2,
    DEFAULT_ACCELERATION_MM_S2,
    PULSES_PER_MM,
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
