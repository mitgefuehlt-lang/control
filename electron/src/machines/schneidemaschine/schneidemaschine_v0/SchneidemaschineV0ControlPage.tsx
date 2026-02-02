import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useSchneidemaschineV0 } from "./useSchneidemaschineV0";
import { Button } from "@/components/ui/button";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { Badge } from "@/components/ui/badge";
import { roundToDecimals } from "@/lib/roundTo";

export function SchneidemaschineV0ControlPage() {
  const {
    state,
    liveValues,
    toggleOutput,
    setAxisSpeedMmS,
    stopAxis,
    stopAllAxes,
    getAxisSpeedMmS,
    getAxisPositionMm,
    isDisabled,
    isLoading,
    MAX_SPEED_MM_S,
  } = useSchneidemaschineV0();

  // Get output state for DO0
  const output0 = state?.output_states[0] ?? false;

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
            ALLE STOPPEN
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

        {/* Axis 2 Motor (index 1) */}
        <ControlCard title="Achse 2 - Motor">
          <div className="flex flex-col gap-4">
            <Label label="Ziel-Geschwindigkeit">
              <EditValue
                value={getAxisSpeedMmS(1)}
                title="Achse 2 Geschwindigkeit"
                min={0}
                max={MAX_SPEED_MM_S}
                step={1}
                renderValue={(v) => `${roundToDecimals(v, 1)} mm/s`}
                onChange={(speed) => setAxisSpeedMmS(1, speed)}
              />
            </Label>
            <Label label="Position">
              <div className="font-mono text-2xl">
                {getAxisPositionMm(1) !== undefined
                  ? `${roundToDecimals(getAxisPositionMm(1)!, 1)} mm`
                  : "-- mm"}
              </div>
            </Label>
            <TouchButton
              variant="outline"
              icon="lu:Square"
              onClick={() => stopAxis(1)}
              disabled={isDisabled}
              isLoading={isLoading}
            >
              Stop Achse 2
            </TouchButton>
          </div>
        </ControlCard>

        {/* Digital Inputs */}
        <ControlCard title="Digitale Eingänge">
          <div className="grid grid-cols-4 gap-3">
            {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => {
              const inputState = liveValues?.input_states[index] ?? false;
              return (
                <div key={index} className="flex flex-col items-center gap-1">
                  <span className="text-xs text-muted-foreground">
                    DI{index}
                  </span>
                  <Badge
                    className={
                      inputState
                        ? "bg-green-600 hover:bg-green-600"
                        : "bg-gray-400 hover:bg-gray-400"
                    }
                  >
                    {inputState ? "HIGH" : "LOW"}
                  </Badge>
                </div>
              );
            })}
          </div>
        </ControlCard>

        {/* Digital Output - existing button */}
        <ControlCard title="Digitaler Ausgang">
          <div className="flex flex-col items-center gap-4">
            <Button
              size="lg"
              variant={output0 ? "default" : "outline"}
              disabled={isDisabled}
              onClick={() => toggleOutput(0)}
              className="h-20 w-40 text-lg"
            >
              {output0 ? "AN" : "AUS"}
            </Button>
            <span className="text-sm text-muted-foreground">
              Digital Output 0
            </span>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
