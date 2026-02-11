import React from "react";
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
    state,
    liveValues,
    isDisabled,
    isLoading,
    INPUT,
    isDoorInterlockActive,
    isAutoRunning,
    isAnyAlarmActive,
    startAutoSequence,
    stopAutoSequence,
  } = useBbmAutomatikV2();

  // Local state
  const [speedPreset, setSpeedPreset] = useState<SpeedPreset>("slow");
  const [magazinSets, setMagazinSets] = useState<number>(1);

  // Server state
  const autoRunning = isAutoRunning();
  const doorInterlock = isDoorInterlockActive();
  const hasAlarm = isAnyAlarmActive();

  // Progress from server
  const currentSet = state?.auto_current_set ?? 0;
  const currentBlock = state?.auto_current_block ?? 0;
  const currentCycle = state?.auto_current_cycle ?? 0;
  const totalSets = state?.auto_total_sets ?? magazinSets;

  // Door sensor
  const doorClosed = liveValues?.input_states[INPUT.TUER] ?? false;
  const canStart = doorClosed && !hasAlarm && !autoRunning && !doorInterlock;

  const handleStart = () => {
    startAutoSequence(speedPreset, magazinSets);
  };

  const handleStop = () => {
    stopAutoSequence();
  };

  return (
    <Page>
      {/* Door interlock banner */}
      {doorInterlock && (
        <div className="mb-4 animate-pulse rounded-lg bg-red-600 px-4 py-3 text-center text-lg font-bold text-white">
          TÜR OFFEN - NOTFALL-STOPP AKTIV
        </div>
      )}

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
                    disabled={autoRunning}
                    className={`h-12 flex-1 ${
                      speedPreset === preset
                        ? preset === "slow"
                          ? "bg-green-600 hover:bg-green-700"
                          : preset === "medium"
                            ? "bg-yellow-600 hover:bg-yellow-700"
                            : "bg-red-600 hover:bg-red-700"
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
                disabled={isDisabled || !canStart}
                isLoading={isLoading}
                className="h-14 flex-1 bg-green-600 text-lg hover:bg-green-700"
              >
                START
              </TouchButton>

              <TouchButton
                variant="destructive"
                icon="lu:Square"
                onClick={handleStop}
                disabled={isDisabled || !autoRunning}
                isLoading={isLoading}
                className={`h-14 flex-1 text-lg ${!autoRunning && !isDisabled ? "border-gray-400 bg-gray-400 text-gray-600 hover:bg-gray-400" : ""}`}
              >
                STOP
              </TouchButton>
            </div>
          </div>
        </ControlCard>

        {/* Sicherheit & Status */}
        <ControlCard title="Sicherheit & Fortschritt">
          <div className="flex flex-col gap-4">
            {/* Door sensor */}
            <div className="space-y-2">
              <Label label="Türsensor">
                <div className="flex gap-4">
                  <div
                    className={`flex items-center gap-2 rounded px-3 py-2 ${
                      doorClosed
                        ? "bg-green-100 text-green-800"
                        : "bg-red-100 text-red-800"
                    }`}
                  >
                    <div
                      className={`h-3 w-3 rounded-full ${
                        doorClosed ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    Tür: {doorClosed ? "Geschlossen" : "Offen"}
                  </div>
                </div>
              </Label>
            </div>

            {!doorClosed && !autoRunning && (
              <div className="rounded bg-red-100 p-3 font-semibold text-red-800">
                Tür muss geschlossen sein bevor Automatik gestartet werden kann!
              </div>
            )}

            {hasAlarm && (
              <div className="rounded bg-red-100 p-3 font-semibold text-red-800">
                Treiber-Alarm aktiv - bitte zuerst zurücksetzen!
              </div>
            )}

            {/* Progress */}
            <div className="space-y-2 border-t pt-4">
              <Label label="Fortschritt">
                <div className="grid grid-cols-3 gap-4">
                  <div className="bg-muted rounded p-2 text-center">
                    <div className="font-mono text-2xl">
                      {currentSet}/{autoRunning ? totalSets : magazinSets}
                    </div>
                    <div className="text-muted-foreground text-xs">Set</div>
                  </div>
                  <div className="bg-muted rounded p-2 text-center">
                    <div className="font-mono text-2xl">{currentBlock}/3</div>
                    <div className="text-muted-foreground text-xs">Block</div>
                  </div>
                  <div className="bg-muted rounded p-2 text-center">
                    <div className="font-mono text-2xl">{currentCycle}/19</div>
                    <div className="text-muted-foreground text-xs">Zyklus</div>
                  </div>
                </div>
              </Label>
            </div>

            {autoRunning && (
              <div className="animate-pulse text-center text-lg font-semibold text-green-600">
                Automatik läuft...
              </div>
            )}
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
