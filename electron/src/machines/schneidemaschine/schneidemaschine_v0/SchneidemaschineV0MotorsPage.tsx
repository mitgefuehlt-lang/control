import React from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useSchneidemaschineV0 } from "./useSchneidemaschineV0";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { roundToDecimals } from "@/lib/decimal";
import { useState } from "react";

// Motor is connected to EL2522 Channel 2 = index 1
const MOTOR_AXIS_INDEX = 1;

export function SchneidemaschineV0MotorsPage() {
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
  } = useSchneidemaschineV0();

  // Local state for inputs (what user enters)
  const [inputSpeed, setInputSpeed] = useState<number>(50);
  const [inputAcceleration, setInputAcceleration] = useState<number>(100);
  const [inputPosition, setInputPosition] = useState<number>(0);

  // Get actual values from server state
  const currentSpeed = getAxisSpeedMmS(MOTOR_AXIS_INDEX) ?? 0;
  const currentAcceleration = getAxisAcceleration(MOTOR_AXIS_INDEX) ?? 100;
  const currentPosition = getAxisPositionMm(MOTOR_AXIS_INDEX) ?? 0;

  // Check server's target speed to determine if motor is commanded to run
  const serverTargetSpeedHz = state?.axis_target_speeds[MOTOR_AXIS_INDEX] ?? 0;
  const isMotorCommanded = serverTargetSpeedHz !== 0;

  // Start motor with input speed (continuous run)
  const handleStart = () => {
    if (inputSpeed > 0) {
      setAxisAcceleration(MOTOR_AXIS_INDEX, inputAcceleration);
      setAxisSpeedMmS(MOTOR_AXIS_INDEX, inputSpeed);
    }
  };

  // Move to target position
  const handleMoveToPosition = () => {
    setAxisAcceleration(MOTOR_AXIS_INDEX, inputAcceleration);
    moveToPosition(MOTOR_AXIS_INDEX, inputPosition, inputSpeed);
  };

  // Stop motor
  const handleStop = () => {
    stopAxis(MOTOR_AXIS_INDEX);
  };

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Axis 1 Motor Control */}
        <ControlCard title="Achse 1 - Motor">
          <div className="flex flex-col gap-6">
            {/* Speed, Acceleration, Position Inputs - side by side */}
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

            {/* Start/Stop/Position Buttons */}
            <div className="flex gap-4">
              <TouchButton
                variant="default"
                icon="lu:Play"
                onClick={handleStart}
                disabled={isDisabled || isMotorCommanded}
                isLoading={isLoading}
                className="flex-1 h-14 text-base bg-green-600 hover:bg-green-700"
              >
                START
              </TouchButton>

              <TouchButton
                variant="default"
                icon="lu:MapPin"
                onClick={handleMoveToPosition}
                disabled={isDisabled || isMotorCommanded}
                isLoading={isLoading}
                className="flex-1 h-14 text-base bg-blue-600 hover:bg-blue-700"
              >
                ZUR POSITION
              </TouchButton>

              <TouchButton
                variant="destructive"
                icon="lu:Square"
                onClick={handleStop}
                disabled={isDisabled || !isMotorCommanded}
                isLoading={isLoading}
                className="flex-1 h-14 text-base"
              >
                STOP
              </TouchButton>
            </div>

            {/* Current Status */}
            <div className="grid grid-cols-2 gap-4 pt-4 border-t">
              <Label label="Aktuelle Geschwindigkeit">
                <div className="font-mono text-2xl">
                  {roundToDecimals(currentSpeed, 1)} mm/s
                </div>
              </Label>
              <Label label="Position">
                <div className="font-mono text-2xl">
                  {getAxisPositionMm(MOTOR_AXIS_INDEX) !== undefined
                    ? `${roundToDecimals(getAxisPositionMm(MOTOR_AXIS_INDEX)!, 1)} mm`
                    : "-- mm"}
                </div>
              </Label>
            </div>

            {/* Running Indicator */}
            {isMotorCommanded && (
              <div className="text-center text-green-600 font-semibold animate-pulse">
                Motor läuft
              </div>
            )}
          </div>
        </ControlCard>

        {/* Info card */}
        <ControlCard title="Info">
          <div className="text-muted-foreground space-y-1">
            <p>Motor: CL57T Stepper</p>
            <p>Max Geschw.: {MAX_SPEED_MM_S} mm/s</p>
            <p>Max Beschl.: {MAX_ACCELERATION_MM_S2} mm/s²</p>
            <p>Spindel: 10mm Lead</p>
            <div className="pt-2 border-t mt-2">
              <p className="text-xs">
                Aktuelle Beschl.: {roundToDecimals(currentAcceleration, 0)} mm/s²
              </p>
            </div>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
