import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS, AXIS_NAMES } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { roundToDecimals } from "@/lib/decimal";
import { create } from "zustand";

interface AxisControlProps {
  axisIndex: number;
  axisName: string;
  isRotation?: boolean;
}

type AxisInputKey = "speed" | "acceleration" | "position" | "step";

type AxisInputState = {
  speed: number;
  acceleration: number;
  position: number;
  step: number;
};

type MotorsUiState = {
  axes: Record<number, AxisInputState>;
  setAxisValue: (axis: number, key: AxisInputKey, value: number) => void;
};

const DEFAULT_AXIS_INPUTS: AxisInputState = {
  speed: 10,
  acceleration: 50,
  position: 0,
  step: 10,
};

const useBbmMotorsUiStore = create<MotorsUiState>((set) => ({
  axes: {},
  setAxisValue: (axis, key, value) =>
    set((state) => {
      const current = state.axes[axis] ?? DEFAULT_AXIS_INPUTS;
      return {
        axes: {
          ...state.axes,
          [axis]: { ...current, [key]: value },
        },
      };
    }),
}));

function AxisControl({ axisIndex, axisName, isRotation = false }: AxisControlProps) {
  const {
    state,
    setAxisSpeedMmS,
    setAxisSpeedRpm,
    setAxisAcceleration,
    moveToPosition,
    stopAxis,
    getAxisSpeedMmS,
    getAxisSpeedRpm,
    getAxisPositionMm,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
    MAX_SPEED_RPM,
    MAX_ACCELERATION_MM_S2,
    MIN_ACCELERATION_MM_S2,
  } = useBbmAutomatikV2();

  const axisInputs = useBbmMotorsUiStore(
    (store) => store.axes[axisIndex] ?? DEFAULT_AXIS_INPUTS,
  );
  const setAxisValue = useBbmMotorsUiStore((store) => store.setAxisValue);

  const inputSpeed = axisInputs.speed;
  const inputAcceleration = axisInputs.acceleration;
  const inputPosition = axisInputs.position;
  const inputStepSize = axisInputs.step;

  // Get actual values from server state
  const currentSpeed = getAxisSpeedMmS(axisIndex) ?? 0;
  const currentPosition = getAxisPositionMm(axisIndex) ?? 0;

  // Check server's target speed to determine if motor is commanded to run
  const serverTargetSpeedHz = state?.axis_target_speeds[axisIndex] ?? 0;
  const isMotorCommanded = serverTargetSpeedHz !== 0;

  // Start motor with input speed (continuous run)
  const handleStart = () => {
    if (inputSpeed > 0) {
      setAxisAcceleration(axisIndex, inputAcceleration);
      setAxisSpeedMmS(axisIndex, inputSpeed);
    }
  };

  // Stop motor
  const handleStop = () => {
    stopAxis(axisIndex);
  };

  // Move to target position
  const handleMoveToPosition = () => {
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, inputPosition, inputSpeed);
  };

  // Jog positive (move by step size)
  const handleJogPlus = () => {
    const targetPos = currentPosition + inputStepSize;
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, targetPos, inputSpeed);
  };

  // Jog negative (move by step size)
  const handleJogMinus = () => {
    const targetPos = Math.max(0, currentPosition - inputStepSize);
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, targetPos, inputSpeed);
  };

  // Homing (move to 0)
  const handleHoming = () => {
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, 0, inputSpeed);
  };

  if (isRotation) {
    // Simplified UI for rotation axis (Bürste) - uses RPM
    const currentSpeedRpm = getAxisSpeedRpm(axisIndex) ?? 0;

    const handleStartRpm = () => {
      if (inputSpeed > 0) {
        setAxisSpeedRpm(axisIndex, inputSpeed);
      }
    };

    return (
      <ControlCard title={`${axisName} (Rotation)`}>
        <div className="flex flex-col gap-4">
          <Label label="Drehzahl">
            <EditValue
              value={inputSpeed}
              title="Drehzahl"
              defaultValue={DEFAULT_AXIS_INPUTS.speed}
              resetPlacement="header"
              min={1}
              max={MAX_SPEED_RPM}
              step={1}
              renderValue={(v) => `${roundToDecimals(v, 0)} RPM`}
              onChange={(speed) => setAxisValue(axisIndex, "speed", speed)}
            />
          </Label>

          <div className="flex gap-4">
            <TouchButton
              variant="default"
              icon="lu:Play"
              onClick={handleStartRpm}
              disabled={isDisabled || isMotorCommanded}
              isLoading={isLoading}
              className="flex-1 h-12 bg-green-600 hover:bg-green-700"
            >
              START
            </TouchButton>

            <TouchButton
              variant="destructive"
              icon="lu:Square"
              onClick={handleStop}
              disabled={isDisabled || !isMotorCommanded}
              isLoading={isLoading}
              className={`flex-1 h-12 ${!isMotorCommanded && !isDisabled ? "bg-gray-400 hover:bg-gray-400 border-gray-400" : ""}`}
            >
              STOP
            </TouchButton>
          </div>

          {/* Current Status */}
          <div className="pt-3 border-t text-sm">
            <span className="text-muted-foreground">Drehzahl: </span>
            <span className="font-mono">{roundToDecimals(currentSpeedRpm, 1)} RPM</span>
          </div>

          {isMotorCommanded && (
            <div className="text-center text-green-600 font-semibold animate-pulse">
              Motor läuft
            </div>
          )}
        </div>
      </ControlCard>
    );
  }

  // Full UI for linear axes - compact layout
  return (
    <ControlCard title={`${axisName} (Linear)`}>
      <div className="flex flex-col gap-4">
        {/* Inputs in single row - 4 columns */}
        <div className="grid grid-cols-4 gap-1">
          <div className="min-w-0">
            <Label label="Geschw.">
              <EditValue
                value={inputSpeed}
                title="Geschwindigkeit"
                compact
                defaultValue={DEFAULT_AXIS_INPUTS.speed}
                resetPlacement="header"
                min={1}
                max={MAX_SPEED_MM_S}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s`}
                onChange={(speed) => setAxisValue(axisIndex, "speed", speed)}
              />
            </Label>
          </div>

          <div className="min-w-0">
            <Label label="Beschl.">
              <EditValue
                value={inputAcceleration}
                title="Beschleunigung"
                compact
                defaultValue={DEFAULT_AXIS_INPUTS.acceleration}
                resetPlacement="header"
                min={MIN_ACCELERATION_MM_S2}
                max={MAX_ACCELERATION_MM_S2}
                step={10}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s²`}
                onChange={(accel) =>
                  setAxisValue(axisIndex, "acceleration", accel)
                }
              />
            </Label>
          </div>

          <div className="min-w-0">
            <Label label="Sollpos.">
              <EditValue
                value={inputPosition}
                title="Sollposition"
                compact
                defaultValue={DEFAULT_AXIS_INPUTS.position}
                resetPlacement="header"
                min={0}
                max={10000}
                step={10}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
                onChange={(pos) =>
                  setAxisValue(axisIndex, "position", pos)
                }
              />
            </Label>
          </div>

          <div className="min-w-0">
            <Label label="Schritt">
              <EditValue
                value={inputStepSize}
                title="Schrittweite"
                compact
                defaultValue={DEFAULT_AXIS_INPUTS.step}
                resetPlacement="header"
                min={1}
                max={1000}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
                onChange={(step) => setAxisValue(axisIndex, "step", step)}
              />
            </Label>
          </div>
        </div>

        {/* All buttons in one row */}
        <div className="flex gap-2">
          <TouchButton
            variant="default"
            icon="lu:Play"
            onClick={handleStart}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-green-600 hover:bg-green-700"
          >
            START
          </TouchButton>

          <TouchButton
            variant="destructive"
            icon="lu:Square"
            onClick={handleStop}
            disabled={isDisabled || !isMotorCommanded}
            isLoading={isLoading}
            className={`flex-1 h-12 ${!isMotorCommanded && !isDisabled ? "bg-gray-400 hover:bg-gray-400 border-gray-400 text-gray-600" : ""}`}
          >
            STOP
          </TouchButton>

          <TouchButton
            variant="default"
            icon="lu:MapPin"
            onClick={handleMoveToPosition}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-blue-600 hover:bg-blue-700"
          >
            POS
          </TouchButton>

          <TouchButton
            variant="default"
            icon="lu:Minus"
            onClick={handleJogMinus}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="h-12 px-4 bg-purple-600 hover:bg-purple-700"
          >
            JOG-
          </TouchButton>

          <TouchButton
            variant="default"
            icon="lu:Plus"
            onClick={handleJogPlus}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="h-12 px-4 bg-purple-600 hover:bg-purple-700"
          >
            JOG+
          </TouchButton>

          <TouchButton
            variant="default"
            icon="lu:Home"
            onClick={handleHoming}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-yellow-500 hover:bg-yellow-600 text-black"
          >
            HOMING
          </TouchButton>
        </div>

        {/* Current Status - compact */}
        <div className="flex gap-6 pt-3 border-t text-sm">
          <div>
            <span className="text-muted-foreground">Geschw: </span>
            <span className="font-mono">{roundToDecimals(currentSpeed, 1)} mm/s</span>
          </div>
          <div>
            <span className="text-muted-foreground">Pos: </span>
            <span className="font-mono">{roundToDecimals(currentPosition, 1)} mm</span>
          </div>
        </div>

        {isMotorCommanded && (
          <div className="text-center text-green-600 font-semibold animate-pulse">
            Motor läuft
          </div>
        )}
      </div>
    </ControlCard>
  );
}

export function BbmAutomatikV2MotorsPage() {
  const {
    setRuettelmotor,
    state,
    isDisabled,
    isLoading,
    OUTPUT,
  } = useBbmAutomatikV2();

  const ruettelmotorOn = state?.output_states[OUTPUT.RUETTELMOTOR] ?? false;

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* MT (Magazin Transporter) */}
        <AxisControl axisIndex={AXIS.MT} axisName={AXIS_NAMES[AXIS.MT]} />

        {/* Schieber */}
        <AxisControl axisIndex={AXIS.SCHIEBER} axisName={AXIS_NAMES[AXIS.SCHIEBER]} />

        {/* Drücker */}
        <AxisControl axisIndex={AXIS.DRUECKER} axisName={AXIS_NAMES[AXIS.DRUECKER]} />

        {/* Bürste (Rotation) */}
        <AxisControl axisIndex={AXIS.BUERSTE} axisName={AXIS_NAMES[AXIS.BUERSTE]} isRotation />

        {/* Rüttelmotor */}
        <ControlCard title="Rüttelmotor">
          <div className="flex flex-col gap-4">
            <TouchButton
              variant={ruettelmotorOn ? "destructive" : "default"}
              icon={ruettelmotorOn ? "lu:Square" : "lu:Play"}
              onClick={() => setRuettelmotor(!ruettelmotorOn)}
              disabled={isDisabled}
              isLoading={isLoading}
              className={`h-14 text-lg ${ruettelmotorOn ? "" : "bg-green-600 hover:bg-green-700"}`}
            >
              {ruettelmotorOn ? "AUS" : "AN"}
            </TouchButton>

            {ruettelmotorOn && (
              <div className="text-center text-green-600 font-semibold animate-pulse">
                Rüttelmotor aktiv
              </div>
            )}
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
