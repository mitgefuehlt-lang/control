import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS, AXIS_NAMES } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { roundToDecimals } from "@/lib/decimal";
import { useState } from "react";

interface AxisControlProps {
  axisIndex: number;
  axisName: string;
  isRotation?: boolean;
}

function AxisControl({ axisIndex, axisName, isRotation = false }: AxisControlProps) {
  const {
    state,
    setAxisSpeedMmS,
    setAxisAcceleration,
    moveToPosition,
    stopAxis,
    getAxisSpeedMmS,
    getAxisPositionMm,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
    MAX_ACCELERATION_MM_S2,
    MIN_ACCELERATION_MM_S2,
  } = useBbmAutomatikV2();

  // Local state for inputs
  const [inputSpeed, setInputSpeed] = useState<number>(50);
  const [inputAcceleration, setInputAcceleration] = useState<number>(100);
  const [inputPosition, setInputPosition] = useState<number>(0);
  const [inputStepSize, setInputStepSize] = useState<number>(10);

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
    // Simplified UI for rotation axis (Bürste)
    return (
      <ControlCard title={`${axisName} (Rotation)`}>
        <div className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-4">
            <Label label="Drehzahl">
              <EditValue
                value={inputSpeed}
                title="Drehzahl"
                min={1}
                max={100}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} Hz`}
                onChange={(speed) => setInputSpeed(speed)}
              />
            </Label>
          </div>

          <div className="flex gap-4">
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
              className={`flex-1 h-12 ${!isMotorCommanded && !isDisabled ? "bg-gray-400 hover:bg-gray-400 border-gray-400" : ""}`}
            >
              STOP
            </TouchButton>
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

  // Full UI for linear axes - two column layout
  return (
    <ControlCard title={`${axisName} (Linear)`}>
      <div className="flex flex-col gap-4">
        {/* Two column layout */}
        <div className="grid grid-cols-2 gap-6">
          {/* LEFT COLUMN: Speed/Acceleration/Start/Stop */}
          <div className="flex flex-col gap-4">
            <Label label="Geschwindigkeit">
              <EditValue
                value={inputSpeed}
                title="Geschwindigkeit"
                min={1}
                max={MAX_SPEED_MM_S}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s`}
                onChange={(speed) => setInputSpeed(speed)}
              />
            </Label>

            <Label label="Beschleunigung">
              <EditValue
                value={inputAcceleration}
                title="Beschleunigung"
                min={MIN_ACCELERATION_MM_S2}
                max={MAX_ACCELERATION_MM_S2}
                step={10}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s²`}
                onChange={(accel) => setInputAcceleration(accel)}
              />
            </Label>

            <div className="flex gap-2 mt-2">
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
            </div>
          </div>

          {/* RIGHT COLUMN: Position/Step/Jog/Homing */}
          <div className="flex flex-col gap-4">
            <Label label="Sollposition">
              <EditValue
                value={inputPosition}
                title="Sollposition"
                min={0}
                max={10000}
                step={10}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
                onChange={(pos) => setInputPosition(pos)}
              />
            </Label>

            <Label label="Schrittweite">
              <EditValue
                value={inputStepSize}
                title="Schrittweite"
                min={1}
                max={1000}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
                onChange={(step) => setInputStepSize(step)}
              />
            </Label>

            <div className="flex gap-2 mt-2">
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
                variant="outline"
                icon="lu:Minus"
                onClick={handleJogMinus}
                disabled={isDisabled || isMotorCommanded}
                isLoading={isLoading}
                className="h-12 px-4"
              >
                JOG-
              </TouchButton>

              <TouchButton
                variant="outline"
                icon="lu:Plus"
                onClick={handleJogPlus}
                disabled={isDisabled || isMotorCommanded}
                isLoading={isLoading}
                className="h-12 px-4"
              >
                JOG+
              </TouchButton>

              <TouchButton
                variant="default"
                icon="lu:Home"
                onClick={handleHoming}
                disabled={isDisabled || isMotorCommanded}
                isLoading={isLoading}
                className="h-12 px-4 bg-yellow-500 hover:bg-yellow-600 text-black"
              >
                HOME
              </TouchButton>
            </div>
          </div>
        </div>

        {/* Current Status */}
        <div className="grid grid-cols-2 gap-4 pt-3 border-t">
          <Label label="Aktuelle Geschwindigkeit">
            <div className="font-mono text-xl">
              {roundToDecimals(currentSpeed, 1)} mm/s
            </div>
          </Label>
          <Label label="Position">
            <div className="font-mono text-xl">
              {roundToDecimals(currentPosition, 1)} mm
            </div>
          </Label>
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
