import React from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS, AXIS_NAMES } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { roundToDecimals } from "@/lib/decimal";
import { create } from "zustand";
import { useState } from "react";

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
    startHoming,
    cancelHoming,
    isAxisHoming,
    getAxisSpeedMmS,
    getAxisSpeedRpm,
    getAxisPositionMm,
    getAxisSoftLimitMax,
    getAxisAlarmActive,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
    MAX_SPEED_RPM,
    MAX_ACCELERATION_MM_S2,
    MIN_ACCELERATION_MM_S2,
  } = useBbmAutomatikV2();

  const isAlarm = getAxisAlarmActive(axisIndex);

  const softLimitMax = getAxisSoftLimitMax(axisIndex);

  const axisInputs = useBbmMotorsUiStore(
    (store) => store.axes[axisIndex] ?? DEFAULT_AXIS_INPUTS,
  );
  const setAxisValue = useBbmMotorsUiStore((store) => store.setAxisValue);

  // All hooks must be called before conditionals (React rules)
  const [direction, setDirection] = useState<"cw" | "ccw">("cw");
  const [error, setError] = useState<string | null>(null);

  const inputSpeed = axisInputs.speed;
  const inputAcceleration = axisInputs.acceleration;
  const inputPosition = axisInputs.position;
  const inputStepSize = axisInputs.step;

  // Get actual values from server state
  const currentSpeed = getAxisSpeedMmS(axisIndex) ?? 0;
  const currentSpeedRpm = getAxisSpeedRpm(axisIndex) ?? 0;
  const currentPosition = getAxisPositionMm(axisIndex) ?? 0;

  // Check server's target speed to determine if motor is commanded to run
  const serverTargetSpeedHz = state?.axis_target_speeds[axisIndex] ?? 0;
  const isMotorCommanded = serverTargetSpeedHz !== 0;

  // Rotation axis handlers
  const handleStartRpm = () => {
    if (inputSpeed > 0) {
      setError(null);
      const rpm = direction === "cw" ? inputSpeed : -inputSpeed;
      setAxisSpeedRpm(axisIndex, rpm);
    }
  };

  const handleStopRotation = () => {
    setError(null);
    stopAxis(axisIndex);
  };

  // Linear axis handlers
  const handleStartLinear = () => {
    if (inputSpeed > 0) {
      setError(null);
      setAxisAcceleration(axisIndex, inputAcceleration);
      setAxisSpeedMmS(axisIndex, inputSpeed);
    }
  };

  const handleStopLinear = () => {
    setError(null);
    stopAxis(axisIndex);
  };

  const handleMoveToPosition = () => {
    setError(null);
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, inputPosition, inputSpeed);
  };

  const handleJogPlus = () => {
    // Round to avoid float accumulation errors
    const targetPos = Math.round(currentPosition + inputStepSize);
    setError(null);
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, targetPos, inputSpeed);
  };

  const handleJogMinus = () => {
    // Round to avoid float accumulation errors (negative positions allowed)
    const targetPos = Math.round(currentPosition - inputStepSize);
    setError(null);
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, targetPos, inputSpeed);
  };

  // Homing state
  const isHoming = isAxisHoming(axisIndex);

  const handleHoming = () => {
    setError(null);
    if (isHoming) {
      // Cancel homing if already running
      cancelHoming(axisIndex);
    } else {
      // Start homing
      startHoming(axisIndex);
    }
  };

  if (isRotation) {
    return (
      <ControlCard title={`${axisName} (Rotation)`}>
        <div className="flex flex-col gap-4">
          {/* Driver alarm banner */}
          {isAlarm && (
            <div className="bg-red-600 text-white text-center font-bold py-2 rounded animate-pulse">
              TREIBER ALARM
            </div>
          )}
          {/* Direction + Speed in row */}
          <div className="grid grid-cols-2 gap-2">
            <Label label="Richtung">
              <div className="flex gap-1">
                <TouchButton
                  variant={direction === "ccw" ? "default" : "outline"}
                  icon="lu:RotateCcw"
                  onClick={() => setDirection("ccw")}
                  disabled={isDisabled || isMotorCommanded}
                  className={`flex-1 h-10 ${direction === "ccw" ? "bg-blue-600 hover:bg-blue-700" : ""}`}
                >
                  CCW
                </TouchButton>
                <TouchButton
                  variant={direction === "cw" ? "default" : "outline"}
                  icon="lu:RotateCw"
                  onClick={() => setDirection("cw")}
                  disabled={isDisabled || isMotorCommanded}
                  className={`flex-1 h-10 ${direction === "cw" ? "bg-blue-600 hover:bg-blue-700" : ""}`}
                >
                  CW
                </TouchButton>
              </div>
            </Label>

            <Label label="Drehzahl">
              <EditValue
                value={inputSpeed}
                title="Drehzahl"
                defaultValue={DEFAULT_AXIS_INPUTS.speed}
                resetPlacement="header"
                compact
                min={1}
                max={MAX_SPEED_RPM}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} RPM`}
                onChange={(speed) => setAxisValue(axisIndex, "speed", speed)}
              />
            </Label>
          </div>

          {/* Buttons */}
          <div className="flex gap-2">
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
              onClick={handleStopRotation}
              disabled={isDisabled || !isMotorCommanded}
              isLoading={isLoading}
              className={`flex-1 h-12 ${!isMotorCommanded && !isDisabled ? "bg-gray-400 hover:bg-gray-400 border-gray-400" : ""}`}
            >
              STOP
            </TouchButton>
          </div>

          {/* Error display */}
          {error && (
            <div className="text-center text-red-600 font-semibold">
              {error}
            </div>
          )}

          {/* Current Status - larger */}
          <div className="pt-3 border-t">
            <span className="text-muted-foreground text-sm">Drehzahl: </span>
            <span className="font-mono text-lg font-semibold">{roundToDecimals(Math.abs(currentSpeedRpm), 1)} RPM</span>
            {currentSpeedRpm !== 0 && (
              <span className="ml-2 text-muted-foreground">
                ({currentSpeedRpm > 0 ? "CW" : "CCW"})
              </span>
            )}
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

  // Full UI for linear axes - improved layout
  return (
    <ControlCard title={`${axisName} (Linear)`}>
      <div className="flex flex-col gap-4">
        {/* Driver alarm banner */}
        {isAlarm && (
          <div className="bg-red-600 text-white text-center font-bold py-2 rounded animate-pulse">
            TREIBER ALARM
          </div>
        )}
        {/* Inputs in 2x2 grid for better readability */}
        <div className="grid grid-cols-2 gap-2">
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

          <Label label="Sollpos.">
            <EditValue
              value={inputPosition}
              title="Sollposition"
              compact
              defaultValue={DEFAULT_AXIS_INPUTS.position}
              resetPlacement="header"
              min={0}
              max={softLimitMax ?? 500}
              step={10}
              renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
              onChange={(pos) =>
                setAxisValue(axisIndex, "position", pos)
              }
            />
          </Label>

          <Label label="Schritt">
            <EditValue
              value={inputStepSize}
              title="Schrittweite"
              compact
              defaultValue={DEFAULT_AXIS_INPUTS.step}
              resetPlacement="header"
              min={0}
              max={softLimitMax ?? 200}
              step={1}
              renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
              onChange={(step) => setAxisValue(axisIndex, "step", step)}
            />
          </Label>
        </div>

        {/* Row 1: START / STOP */}
        <div className="flex gap-2">
          <TouchButton
            variant="default"
            icon="lu:Play"
            onClick={handleStartLinear}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-green-600 hover:bg-green-700"
          >
            START
          </TouchButton>

          <TouchButton
            variant="destructive"
            icon="lu:Square"
            onClick={handleStopLinear}
            disabled={isDisabled || !isMotorCommanded}
            isLoading={isLoading}
            className={`flex-1 h-12 ${!isMotorCommanded && !isDisabled ? "bg-gray-400 hover:bg-gray-400 border-gray-400 text-gray-600" : ""}`}
          >
            STOP
          </TouchButton>
        </div>

        {/* Row 2: JOG- / POSITION / JOG+ / HOMING */}
        <div className="flex gap-2">
          <TouchButton
            variant="default"
            onClick={handleJogMinus}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-blue-600 hover:bg-blue-700"
          >
            - JOG
          </TouchButton>

          <TouchButton
            variant="default"
            icon="lu:MapPin"
            onClick={handleMoveToPosition}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-blue-600 hover:bg-blue-700"
          >
            FAHRE
          </TouchButton>

          <TouchButton
            variant="default"
            onClick={handleJogPlus}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-blue-600 hover:bg-blue-700"
          >
            + JOG
          </TouchButton>

          <TouchButton
            variant={isHoming ? "destructive" : "default"}
            icon={isHoming ? "lu:Square" : "lu:House"}
            onClick={handleHoming}
            disabled={isDisabled || (isMotorCommanded && !isHoming)}
            isLoading={isLoading}
            className={`flex-1 h-12 ${isHoming ? "animate-pulse" : "bg-amber-500 hover:bg-amber-600 text-black"}`}
          >
            {isHoming ? "STOP" : "HOME"}
          </TouchButton>
        </div>

        {/* Error display */}
        {error && (
          <div className="text-center text-red-600 font-semibold">
            {error}
          </div>
        )}

        {/* Homing status */}
        {isHoming && (
          <div className="text-center text-amber-600 font-semibold animate-pulse">
            Referenzfahrt läuft...
          </div>
        )}

        {/* Current Status - larger and more prominent */}
        <div className="flex justify-between pt-3 border-t">
          <div>
            <span className="text-muted-foreground text-sm">Geschw: </span>
            <span className="font-mono text-lg font-semibold">{roundToDecimals(currentSpeed, 1)} mm/s</span>
          </div>
          <div>
            <span className="text-muted-foreground text-sm">Pos: </span>
            <span className="font-mono text-lg font-semibold">{roundToDecimals(currentPosition, 1)} mm</span>
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
    resetAlarms,
    isAnyAlarmActive,
    state,
    isDisabled,
    isLoading,
    OUTPUT,
  } = useBbmAutomatikV2();

  const ruettelmotorOn = state?.output_states[OUTPUT.RUETTELMOTOR] ?? false;
  const hasAlarm = isAnyAlarmActive();

  return (
    <Page>
      {/* Global alarm reset banner */}
      {hasAlarm && (
        <div className="mb-4 flex items-center justify-between bg-red-600 text-white rounded-lg px-4 py-3">
          <span className="font-bold text-lg animate-pulse">TREIBER ALARM AKTIV</span>
          <TouchButton
            variant="outline"
            icon="lu:RotateCcw"
            onClick={() => resetAlarms()}
            className="bg-white text-red-600 hover:bg-red-100 border-white font-bold"
          >
            ALARM RESET
          </TouchButton>
        </div>
      )}

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
