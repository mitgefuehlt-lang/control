import { toastError } from "@/components/Toast";
import { useMachineMutate as useMachineMutation } from "@/client/useClient";
import { MachineIdentificationUnique } from "@/machines/types";
import { bbmAutomatikV2 } from "@/machines/properties";
import { bbmAutomatikV2SerialRoute } from "@/routes/routes";
import { z } from "zod";
import {
  useBbmAutomatikV2Namespace,
  StateEvent,
} from "./bbmAutomatikV2Namespace";
import { useEffect, useMemo } from "react";
import { useStateOptimistic } from "@/lib/useStateOptimistic";
import { produce } from "immer";

// Motor constants
const PULSES_PER_MM = 20;
const PULSES_PER_REV = 200;
const MAX_SPEED_MM_S = 250;
const MAX_SPEED_RPM = 100;  // Max RPM for rotation axes
const DEFAULT_ACCELERATION_MM_S2 = 100;
const MAX_ACCELERATION_MM_S2 = 500;
const MIN_ACCELERATION_MM_S2 = 1;

// Axis indices
export const AXIS = {
  MT: 0,
  SCHIEBER: 1,
  DRUECKER: 2,
  BUERSTE: 3,
} as const;

// Axis names for display
export const AXIS_NAMES = ["Transporter", "Schieber", "Drücker", "Bürste"] as const;

// Digital input indices
export const INPUT = {
  REF_MT: 0,
  REF_SCHIEBER: 1,
  REF_DRUECKER: 2,
  TUER_1: 3,
  TUER_2: 4,
} as const;

// Digital output indices
export const OUTPUT = {
  RUETTELMOTOR: 0,
  AMPEL_ROT: 1,
  AMPEL_GELB: 2,
  AMPEL_GRUEN: 3,
} as const;

function useBbmAutomatik(
  machine_identification_unique: MachineIdentificationUnique,
) {
  const { state, defaultState, liveValues } = useBbmAutomatikV2Namespace(
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

  // SetOutput mutation
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
        if (current.data.axis_speeds && index >= 0 && index < 4) {
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

  // SetAxisSpeedRpm mutation (for rotation axes)
  const setAxisSpeedRpmSchema = z.object({
    action: z.literal("SetAxisSpeedRpm"),
    value: z.object({
      index: z.number(),
      rpm: z.number(),
    }),
  });
  const { request: requestSetAxisSpeedRpm } =
    useMachineMutation(setAxisSpeedRpmSchema);

  const setAxisSpeedRpm = (index: number, rpm: number) => {
    updateStateOptimistically(
      (current) => {
        if (current.data.axis_speeds && index >= 0 && index < 4) {
          // RPM to Hz: rpm * 200 / 60
          current.data.axis_speeds[index] = Math.round(rpm * PULSES_PER_REV / 60);
        }
      },
      () =>
        requestSetAxisSpeedRpm({
          machine_identification_unique,
          data: {
            action: "SetAxisSpeedRpm",
            value: { index, rpm },
          },
        }),
    );
  };

  // StopAxis mutation
  const stopAxisSchema = z.object({
    action: z.literal("StopAxis"),
    value: z.object({
      index: z.number(),
    }),
  });
  const { request: requestStopAxis } = useMachineMutation(stopAxisSchema);

  const stopAxis = (index: number) => {
    updateStateOptimistically(
      (current) => {
        current.data.axis_speeds[index] = 0;
        current.data.axis_target_speeds[index] = 0;
      },
      () =>
        requestStopAxis({
          machine_identification_unique,
          data: { action: "StopAxis", value: { index } },
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
        current.data.axis_speeds = [0, 0, 0, 0];
        current.data.axis_target_speeds = [0, 0, 0, 0];
      },
      () =>
        requestStopAllAxes({
          machine_identification_unique,
          data: { action: "StopAllAxes" },
        }),
    );
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
        if (current.data.axis_target_positions && index >= 0 && index < 4) {
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
        if (current.data.axis_accelerations && index >= 0 && index < 4) {
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

  // SetRuettelmotor mutation
  const setRuettelmotorSchema = z.object({
    action: z.literal("SetRuettelmotor"),
    value: z.object({
      on: z.boolean(),
    }),
  });
  const { request: requestSetRuettelmotor } =
    useMachineMutation(setRuettelmotorSchema);

  const setRuettelmotor = (on: boolean) => {
    updateStateOptimistically(
      (current) => {
        current.data.output_states[OUTPUT.RUETTELMOTOR] = on;
      },
      () =>
        requestSetRuettelmotor({
          machine_identification_unique,
          data: { action: "SetRuettelmotor", value: { on } },
        }),
    );
  };

  // SetAmpel mutation
  const setAmpelSchema = z.object({
    action: z.literal("SetAmpel"),
    value: z.object({
      rot: z.boolean(),
      gelb: z.boolean(),
      gruen: z.boolean(),
    }),
  });
  const { request: requestSetAmpel } = useMachineMutation(setAmpelSchema);

  const setAmpel = (rot: boolean, gelb: boolean, gruen: boolean) => {
    updateStateOptimistically(
      (current) => {
        current.data.output_states[OUTPUT.AMPEL_ROT] = rot;
        current.data.output_states[OUTPUT.AMPEL_GELB] = gelb;
        current.data.output_states[OUTPUT.AMPEL_GRUEN] = gruen;
      },
      () =>
        requestSetAmpel({
          machine_identification_unique,
          data: { action: "SetAmpel", value: { rot, gelb, gruen } },
        }),
    );
  };

  // StartHoming mutation
  const startHomingSchema = z.object({
    action: z.literal("StartHoming"),
    value: z.object({
      index: z.number(),
    }),
  });
  const { request: requestStartHoming } = useMachineMutation(startHomingSchema);

  const startHoming = (index: number) => {
    updateStateOptimistically(
      (current) => {
        current.data.axis_homing_active[index] = true;
      },
      () =>
        requestStartHoming({
          machine_identification_unique,
          data: { action: "StartHoming", value: { index } },
        }),
    );
  };

  // CancelHoming mutation
  const cancelHomingSchema = z.object({
    action: z.literal("CancelHoming"),
    value: z.object({
      index: z.number(),
    }),
  });
  const { request: requestCancelHoming } = useMachineMutation(cancelHomingSchema);

  const cancelHoming = (index: number) => {
    updateStateOptimistically(
      (current) => {
        current.data.axis_homing_active[index] = false;
      },
      () =>
        requestCancelHoming({
          machine_identification_unique,
          data: { action: "CancelHoming", value: { index } },
        }),
    );
  };

  // Helper to check if axis is homing
  const isAxisHoming = (index: number): boolean => {
    return stateOptimistic.value?.data.axis_homing_active[index] ?? false;
  };

  // Helper to get axis speed in mm/s (for linear axes)
  const getAxisSpeedMmS = (index: number): number | undefined => {
    const speedHz = stateOptimistic.value?.data.axis_speeds[index];
    return speedHz !== undefined ? speedHz / PULSES_PER_MM : undefined;
  };

  // Helper to get axis speed in RPM (for rotation axes)
  const getAxisSpeedRpm = (index: number): number | undefined => {
    const speedHz = stateOptimistic.value?.data.axis_speeds[index];
    return speedHz !== undefined ? speedHz * 60 / PULSES_PER_REV : undefined;
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

  // Check if doors are closed
  const areDoorsClosedFn = (): boolean => {
    if (!liveValues) return false;
    return liveValues.data.input_states[INPUT.TUER_1] &&
           liveValues.data.input_states[INPUT.TUER_2];
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
    setAxisSpeedRpm,
    setAxisAcceleration,
    moveToPosition,
    stopAxis,
    stopAllAxes,

    // Convenience functions
    setRuettelmotor,
    setAmpel,
    areDoorsClosed: areDoorsClosedFn,

    // Homing functions
    startHoming,
    cancelHoming,
    isAxisHoming,

    // Motor helper functions
    getAxisSpeedMmS,
    getAxisSpeedRpm,
    getAxisPositionMm,
    getAxisAcceleration,

    // Constants
    AXIS,
    AXIS_NAMES,
    INPUT,
    OUTPUT,
    MAX_SPEED_MM_S,
    MAX_SPEED_RPM,
    MAX_ACCELERATION_MM_S2,
    MIN_ACCELERATION_MM_S2,
    DEFAULT_ACCELERATION_MM_S2,
    PULSES_PER_MM,
  };
}

export function useBbmAutomatikV2() {
  const { serial: serialString } = bbmAutomatikV2SerialRoute.useParams();

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
      machine_identification: bbmAutomatikV2.machine_identification,
      serial,
    };
  }, [serialString]);

  return useBbmAutomatik(machineIdentification);
}
