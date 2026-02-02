import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useSchneidemaschineV0 } from "./useSchneidemaschineV0";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { roundToDecimals } from "@/lib/decimal";

export function SchneidemaschineV0MotorsPage() {
  const {
    setAxisSpeedMmS,
    stopAxis,
    stopAllAxes,
    getAxisSpeedMmS,
    getAxisPositionMm,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
  } = useSchneidemaschineV0();

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Emergency Stop */}
        <ControlCard title="Emergency Stop">
          <TouchButton
            variant="destructive"
            icon="lu:OctagonX"
            onClick={stopAllAxes}
            disabled={isDisabled}
            isLoading={isLoading}
            className="h-32 w-full text-2xl"
          >
            STOPPEN
          </TouchButton>
        </ControlCard>

        {/* Axis 1 Motor (index 0) */}
        <ControlCard title="Achse 1 - Motor">
          <div className="flex flex-col gap-4">
            <Label label="Ziel-Geschwindigkeit">
              <EditValue
                value={getAxisSpeedMmS(0)}
                title="Achse 1 Geschwindigkeit"
                min={0}
                max={MAX_SPEED_MM_S}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 1)} mm/s`}
                onChange={(speed) => setAxisSpeedMmS(0, speed)}
              />
            </Label>
            <Label label="Position">
              <div className="font-mono text-2xl">
                {getAxisPositionMm(0) !== undefined
                  ? `${roundToDecimals(getAxisPositionMm(0)!, 1)} mm`
                  : "-- mm"}
              </div>
            </Label>
            <TouchButton
              variant="outline"
              icon="lu:Square"
              onClick={() => stopAxis(0)}
              disabled={isDisabled}
              isLoading={isLoading}
            >
              Stop Achse 1
            </TouchButton>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
