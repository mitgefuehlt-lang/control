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

  // Local state for target speed and acceleration (what user enters)
  const [targetSpeed, setTargetSpeed] = useState<number>(50);
  const [targetAcceleration, setTargetAcceleration] = useState<number>(
    getAxisAcceleration(MOTOR_AXIS_INDEX) ?? 100,
  );

  // Check if motor is running (speed > 0)
  const currentSpeed = getAxisSpeedMmS(MOTOR_AXIS_INDEX) ?? 0;
  const currentAcceleration = getAxisAcceleration(MOTOR_AXIS_INDEX) ?? 100;
  const isRunning = currentSpeed > 0;

  // Start motor with target speed (also applies acceleration first)
  const handleStart = () => {
    if (targetSpeed > 0) {
      // Apply acceleration setting first
      if (targetAcceleration !== currentAcceleration) {
        setAxisAcceleration(MOTOR_AXIS_INDEX, targetAcceleration);
      }
      setAxisSpeedMmS(MOTOR_AXIS_INDEX, targetSpeed);
    }
  };

  // Apply acceleration change
  const handleAccelerationChange = (accel: number) => {
    setTargetAcceleration(accel);
    setAxisAcceleration(MOTOR_AXIS_INDEX, accel);
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
            {/* Speed Input */}
            <Label label="Ziel-Geschwindigkeit">
              <EditValue
                value={targetSpeed}
                title="Ziel-Geschwindigkeit"
                min={1}
                max={MAX_SPEED_MM_S}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s`}
                onChange={(speed) => setTargetSpeed(speed)}
              />
            </Label>

            {/* Acceleration Input */}
            <Label label="Beschleunigung">
              <EditValue
                value={targetAcceleration}
                title="Beschleunigung"
                min={MIN_ACCELERATION_MM_S2}
                max={MAX_ACCELERATION_MM_S2}
                step={10}
                renderValue={(v) => `${roundToDecimals(v, 0)} mm/s²`}
                onChange={handleAccelerationChange}
              />
            </Label>

            {/* Start/Stop Buttons */}
            <div className="flex gap-4">
              <TouchButton
                variant="default"
                icon="lu:Play"
                onClick={handleStart}
                disabled={isDisabled || isRunning}
                isLoading={isLoading}
                className="flex-1 h-16 text-lg bg-green-600 hover:bg-green-700"
              >
                START
              </TouchButton>

              <TouchButton
                variant="destructive"
                icon="lu:Square"
                onClick={handleStop}
                disabled={isDisabled || !isRunning}
                isLoading={isLoading}
                className="flex-1 h-16 text-lg"
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
            {isRunning && (
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
