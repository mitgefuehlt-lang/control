import React from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2 } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { Label } from "@/control/Label";
import { useState } from "react";

type SpeedPreset = "slow" | "medium" | "fast";

export function BbmAutomatikV2TestPage() {
  const {
    isDisabled,
    isLoading,
    isAutoRunning,
    isDoorInterlockActive,
    isAnyAlarmActive,
    getMissingAutoTeachPositions,
    areAllAxesHomed,
    getUnhomedAxisNames,
    startAutoSequence,
    stopAutoSequence,
    stopAllAxes,
    startHoming,
    AXIS,
  } = useBbmAutomatikV2();

  const [speedPreset, setSpeedPreset] = useState<SpeedPreset>("slow");

  const autoRunning = isAutoRunning();
  const doorInterlock = isDoorInterlockActive();
  const hasAlarm = isAnyAlarmActive();
  // The sequence drives the teached calibration positions; it cannot start
  // until all required positions are set (mirrors the backend guard).
  const missingTeach = getMissingAutoTeachPositions();
  const teachComplete = missingTeach.length === 0;
  // Global homing gate: all axes must be referenced before any run.
  const allHomed = areAllAxesHomed();
  const unhomedAxes = getUnhomedAxisNames();
  const canStart =
    !autoRunning && !doorInterlock && !hasAlarm && teachComplete && allHomed;

  const handleSequence1x = () => {
    // 1x befüllen = 1 set (runs 1 block of 19 cycles)
    startAutoSequence(speedPreset, 1);
  };

  const handleSequence5x = () => {
    // 5x = 5 sets
    startAutoSequence(speedPreset, 5);
  };

  const handleSequenceMagazin = () => {
    // 1 Magazin = 1 set (3 blocks x 19 cycles)
    startAutoSequence(speedPreset, 1);
  };

  const handleReset = () => {
    stopAutoSequence();
    stopAllAxes();
    // Home all linear axes
    startHoming(AXIS.MT);
    startHoming(AXIS.SCHIEBER);
    startHoming(AXIS.DRUECKER);
  };

  return (
    <Page>
      {/* Door interlock banner */}
      {doorInterlock && (
        <div className="mb-4 animate-pulse rounded-lg bg-red-600 px-4 py-3 text-center text-lg font-bold text-white">
          TÜR OFFEN - NOTFALL-STOPP AKTIV
        </div>
      )}

      {/* Global homing gate: all axes must be referenced before a run. */}
      {!allHomed && (
        <div className="mb-4 rounded-lg bg-amber-500 px-4 py-3 text-black">
          <div className="font-bold">
            Start gesperrt - Achsen nicht referenziert
          </div>
          <div className="text-sm">
            Bitte zuerst referenzieren (Reset/HOME): {unhomedAxes.join(", ")}
          </div>
        </div>
      )}

      {/* Teach-in incomplete banner: the sequence uses the teached
          positions, so it can't run until all are calibrated. */}
      {!teachComplete && (
        <div className="mb-4 rounded-lg bg-amber-500 px-4 py-3 text-black">
          <div className="font-bold">
            Start gesperrt - Kalibrierung unvollständig
          </div>
          <div className="text-sm">
            Folgende Positionen müssen in der Kalibrierung geteacht werden:{" "}
            {missingTeach.join(", ")}
          </div>
        </div>
      )}

      <ControlGrid columns={2}>
        <ControlCard title="Test-Sequenzen">
          <div className="flex flex-col gap-4">
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

            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequence1x}
              disabled={isDisabled || !canStart}
              isLoading={isLoading}
              className="h-14 bg-blue-600 text-lg hover:bg-blue-700"
            >
              1x befüllen
            </TouchButton>

            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequence5x}
              disabled={isDisabled || !canStart}
              isLoading={isLoading}
              className="h-14 bg-blue-600 text-lg hover:bg-blue-700"
            >
              5x befüllen
            </TouchButton>

            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequenceMagazin}
              disabled={isDisabled || !canStart}
              isLoading={isLoading}
              className="h-14 bg-blue-600 text-lg hover:bg-blue-700"
            >
              1 Magazin (19x)
            </TouchButton>

            <TouchButton
              variant="outline"
              icon="lu:RotateCcw"
              onClick={handleReset}
              disabled={isDisabled}
              isLoading={isLoading}
              className="h-14 text-lg"
            >
              Reset
            </TouchButton>

            {autoRunning && (
              <div className="animate-pulse text-center text-lg font-semibold text-green-600">
                Sequenz läuft...
              </div>
            )}
          </div>
        </ControlCard>

        <ControlCard title="Info">
          <div className="text-muted-foreground space-y-2">
            <p>
              <strong>1x befüllen:</strong> 1 Set (3 Blöcke x 19 Zyklen)
            </p>
            <p>
              <strong>5x befüllen:</strong> 5 Sets nacheinander
            </p>
            <p>
              <strong>1 Magazin (19x):</strong> 1 Set (3 Blöcke x 19 Zyklen)
            </p>
            <p>
              <strong>Reset:</strong> Stoppt Sequenz und fährt alle Achsen in
              Referenzposition
            </p>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
