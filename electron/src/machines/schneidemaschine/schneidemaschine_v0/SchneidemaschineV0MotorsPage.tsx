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
    stopAxis,
    getAxisSpeedMmS,
    getAxisPositionMm,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
  } = useSchneidemaschineV0();

  // Local state for target speed (what user enters)
  const [targetSpeed, setTargetSpeed] = useState<number>(50);

  // Check if motor is running (speed > 0)
  const currentSpeed = getAxisSpeedMmS(MOTOR_AXIS_INDEX) ?? 0;
  const isRunning = currentSpeed > 0;

  // Start motor with target speed
  const handleStart = () => {
    if (targetSpeed > 0) {
      setAxisSpeedMmS(MOTOR_AXIS_INDEX, targetSpeed);
    }
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

        {/* Empty card for layout balance */}
        <ControlCard title="Info">
          <div className="text-muted-foreground">
            <p>Motor: CL57T Stepper</p>
            <p>Max: {MAX_SPEED_MM_S} mm/s</p>
            <p>Spindel: 10mm Lead</p>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
