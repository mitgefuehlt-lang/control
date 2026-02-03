import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2 } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { EditValue } from "@/control/EditValue";
import { Label } from "@/control/Label";
import { useState } from "react";

type SpeedPreset = "slow" | "medium" | "fast";

export function BbmAutomatikV2AutoPage() {
  const {
    liveValues,
    areDoorsClosed,
    isDisabled,
    isLoading,
    INPUT,
  } = useBbmAutomatikV2();

  // Local state
  const [speedPreset, setSpeedPreset] = useState<SpeedPreset>("medium");
  const [magazinSets, setMagazinSets] = useState<number>(1);
  const [isRunning, setIsRunning] = useState(false);

  // Progress (placeholder)
  const currentSet = 0;
  const currentBlock = 0;
  const currentCycle = 0;

  // Door sensors
  const door1Closed = liveValues?.input_states[INPUT.TUER_1] ?? false;
  const door2Closed = liveValues?.input_states[INPUT.TUER_2] ?? false;
  const doorsAreSafe = door1Closed && door2Closed;

  const handleStart = () => {
    if (!doorsAreSafe) {
      console.log("Türen müssen geschlossen sein!");
      return;
    }
    setIsRunning(true);
    console.log("Automatik Start", { speedPreset, magazinSets });
  };

  const handleStop = () => {
    setIsRunning(false);
    console.log("Automatik Stop");
  };

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Parameter */}
        <ControlCard title="Parameter">
          <div className="flex flex-col gap-6">
            <Label label="Geschwindigkeit">
              <div className="flex gap-2">
                {(["slow", "medium", "fast"] as SpeedPreset[]).map((preset) => (
                  <TouchButton
                    key={preset}
                    variant={speedPreset === preset ? "default" : "outline"}
                    onClick={() => setSpeedPreset(preset)}
                    disabled={isRunning}
                    className={`flex-1 h-12 ${
                      speedPreset === preset
                        ? preset === "slow"
                          ? "bg-yellow-600 hover:bg-yellow-700"
                          : preset === "medium"
                          ? "bg-blue-600 hover:bg-blue-700"
                          : "bg-green-600 hover:bg-green-700"
                        : ""
                    }`}
                  >
                    {preset === "slow"
                      ? "Langsam"
                      : preset === "medium"
                      ? "Mittel"
                      : "Schnell"}
                  </TouchButton>
                ))}
              </div>
            </Label>

            <Label label="Anzahl Magazin-Sets">
              <EditValue
                value={magazinSets}
                title="Magazin-Sets"
                min={1}
                max={100}
                step={1}
                renderValue={(v) => `${v} Set${v > 1 ? "s" : ""}`}
                onChange={(v) => setMagazinSets(v)}
              />
            </Label>

            <div className="flex gap-4 pt-4">
              <TouchButton
                variant="default"
                icon="lu:Play"
                onClick={handleStart}
                disabled={isDisabled || isRunning || !doorsAreSafe}
                isLoading={isLoading}
                className="flex-1 h-14 text-lg bg-green-600 hover:bg-green-700"
              >
                START
              </TouchButton>

              <TouchButton
                variant="destructive"
                icon="lu:Square"
                onClick={handleStop}
                disabled={isDisabled || !isRunning}
                isLoading={isLoading}
                className="flex-1 h-14 text-lg"
              >
                STOP
              </TouchButton>
            </div>
          </div>
        </ControlCard>

        {/* Sicherheit & Status */}
        <ControlCard title="Sicherheit & Fortschritt">
          <div className="flex flex-col gap-4">
            {/* Door sensors */}
            <div className="space-y-2">
              <Label label="Türsensoren">
                <div className="flex gap-4">
                  <div
                    className={`flex items-center gap-2 px-3 py-2 rounded ${
                      door1Closed ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"
                    }`}
                  >
                    <div
                      className={`w-3 h-3 rounded-full ${
                        door1Closed ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    Tür 1: {door1Closed ? "Geschlossen" : "Offen"}
                  </div>
                  <div
                    className={`flex items-center gap-2 px-3 py-2 rounded ${
                      door2Closed ? "bg-green-100 text-green-800" : "bg-red-100 text-red-800"
                    }`}
                  >
                    <div
                      className={`w-3 h-3 rounded-full ${
                        door2Closed ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    Tür 2: {door2Closed ? "Geschlossen" : "Offen"}
                  </div>
                </div>
              </Label>
            </div>

            {!doorsAreSafe && (
              <div className="bg-red-100 text-red-800 p-3 rounded font-semibold">
                Türen müssen geschlossen sein bevor Automatik gestartet werden kann!
              </div>
            )}

            {/* Progress */}
            <div className="pt-4 border-t space-y-2">
              <Label label="Fortschritt">
                <div className="grid grid-cols-3 gap-4">
                  <div className="text-center p-2 bg-muted rounded">
                    <div className="text-2xl font-mono">{currentSet}/{magazinSets}</div>
                    <div className="text-xs text-muted-foreground">Set</div>
                  </div>
                  <div className="text-center p-2 bg-muted rounded">
                    <div className="text-2xl font-mono">{currentBlock}/3</div>
                    <div className="text-xs text-muted-foreground">Block</div>
                  </div>
                  <div className="text-center p-2 bg-muted rounded">
                    <div className="text-2xl font-mono">{currentCycle}/19</div>
                    <div className="text-xs text-muted-foreground">Zyklus</div>
                  </div>
                </div>
              </Label>
            </div>

            {isRunning && (
              <div className="text-center text-green-600 font-semibold animate-pulse text-lg">
                Automatik läuft...
              </div>
            )}
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
