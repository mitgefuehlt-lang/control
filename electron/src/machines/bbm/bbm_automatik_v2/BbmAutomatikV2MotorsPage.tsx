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
    getAxisAcceleration,
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

  // Start motor backward
  const handleStartBackward = () => {
    if (inputSpeed > 0) {
      setAxisAcceleration(axisIndex, inputAcceleration);
      setAxisSpeedMmS(axisIndex, -inputSpeed);
    }
  };

  // Move to target position
  const handleMoveToPosition = () => {
    setAxisAcceleration(axisIndex, inputAcceleration);
    moveToPosition(axisIndex, inputPosition, inputSpeed);
  };

  // Stop motor
  const handleStop = () => {
    stopAxis(axisIndex);
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
              className="flex-1 h-12"
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

  // Full UI for linear axes
  return (
    <ControlCard title={`${axisName} (Linear)`}>
      <div className="flex flex-col gap-4">
        {/* Speed, Acceleration, Position Inputs */}
        <div className="grid grid-cols-3 gap-4">
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

          <Label label="Ziel-Position">
            <EditValue
              value={inputPosition}
              title="Ziel-Position"
              min={0}
              max={10000}
              step={10}
              renderValue={(v) => `${roundToDecimals(v, 0)} mm`}
              onChange={(pos) => setInputPosition(pos)}
            />
          </Label>
        </div>

        {/* Control Buttons */}
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
            variant="default"
            icon="lu:MapPin"
            onClick={handleMoveToPosition}
            disabled={isDisabled || isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12 bg-blue-600 hover:bg-blue-700"
          >
            ZUR POSITION
          </TouchButton>

          <TouchButton
            variant="destructive"
            icon="lu:Square"
            onClick={handleStop}
            disabled={isDisabled || !isMotorCommanded}
            isLoading={isLoading}
            className="flex-1 h-12"
          >
            STOP
          </TouchButton>

          <TouchButton
            variant="outline"
            icon="lu:Home"
            onClick={() => {}}
            disabled={true}
            className="h-12"
          >
            HOME
          </TouchButton>
        </div>

        {/* Current Status */}
        <div className="grid grid-cols-2 gap-4 pt-2 border-t">
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
