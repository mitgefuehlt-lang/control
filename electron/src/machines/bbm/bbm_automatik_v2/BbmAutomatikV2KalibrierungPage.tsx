import React, { useState } from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS, AXIS_NAMES } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { Input } from "@/components/ui/input";
import { roundToDecimals } from "@/lib/decimal";
import { TEACH_SLOT, TeachSlot } from "./bbmAutomatikV2Namespace";

// Calibration runs the axis blind toward a saved target — typically
// against an as-yet-uncalibrated soft limit. Use a slow, conservative
// default; the user can still bump it up explicitly per session.
const DEFAULT_GOTO_SPEED_MM_S = 10;

type SlotDef = {
  slot: TeachSlot;
  label: string;
};

// Welche Teach-Slots pro Achse sichtbar sind und wie sie heissen.
// Hinweis: ausgeblendete Slots (z.B. Custom2) werden NICHT geloescht —
// ihre im Backend gespeicherten Werte bleiben unangetastet, sie werden
// hier nur nicht angezeigt. Die Labels sind fest (kein Umbenennen mehr).
const AXIS_SLOTS: Record<number, SlotDef[]> = {
  [AXIS.MT]: [
    { slot: TEACH_SLOT.START, label: "Start" },
    { slot: TEACH_SLOT.ZIEL, label: "Ziel" },
  ],
  [AXIS.SCHIEBER]: [
    { slot: TEACH_SLOT.START, label: "Start" },
    { slot: TEACH_SLOT.ZIEL, label: "Ziel" },
    { slot: TEACH_SLOT.CUSTOM1, label: "Reinigung" },
  ],
  [AXIS.DRUECKER]: [
    { slot: TEACH_SLOT.START, label: "Start" },
    { slot: TEACH_SLOT.ZIEL, label: "Ziel" },
    { slot: TEACH_SLOT.CUSTOM1, label: "Wartung" },
  ],
};

type SoftLimitRowProps = {
  label: string;
  value: number | null;
  onTeach: () => void;
  onClear: () => void;
  disabled: boolean;
};

function SoftLimitRow({
  label,
  value,
  onTeach,
  onClear,
  disabled,
}: SoftLimitRowProps) {
  const hasValue = value !== null;
  return (
    <div className="flex items-center gap-2 rounded-md border px-3 py-2">
      <span className="text-sm font-medium">{label}</span>
      <span className="ml-auto font-mono text-base">
        {hasValue ? `${roundToDecimals(value, 2)} mm` : "—"}
      </span>
      <TouchButton
        variant="outline"
        icon="lu:Crosshair"
        onClick={onTeach}
        disabled={disabled}
        title="Aktuelle Position als Limit setzen"
      />
      <TouchButton
        variant="destructive"
        icon="lu:Trash2"
        onClick={onClear}
        disabled={disabled || !hasValue}
        title="Limit löschen"
      />
    </div>
  );
}

type AxisCalibrationProps = {
  axisIndex: number;
  axisName: string;
};

function AxisCalibration({ axisIndex, axisName }: AxisCalibrationProps) {
  const {
    saveTeachPosition,
    clearTeachPosition,
    goToTeachPosition,
    getTeachPositionMm,
    getAxisPositionMm,
    getAxisSoftLimitMax,
    getAxisSoftLimitMin,
    setSoftLimitMax,
    setSoftLimitMin,
    teachSoftLimitMax,
    teachSoftLimitMin,
    isDisabled,
  } = useBbmAutomatikV2();

  const [gotoSpeed, setGotoSpeed] = useState<number>(DEFAULT_GOTO_SPEED_MM_S);

  const currentPos = getAxisPositionMm(axisIndex) ?? 0;
  const softLimitMax = getAxisSoftLimitMax(axisIndex);
  const softLimitMin = getAxisSoftLimitMin(axisIndex);
  const slots = AXIS_SLOTS[axisIndex] ?? [];

  return (
    <ControlCard title={axisName}>
      <div className="flex flex-col gap-3">
        {/* Live position + Anfahr-Geschwindigkeit in einer Zeile */}
        <div className="flex items-center justify-between gap-3 rounded-md bg-muted px-3 py-2">
          <div className="flex flex-col">
            <span className="text-xs font-medium text-muted-foreground">
              Aktuelle Position
            </span>
            <span className="font-mono text-2xl font-bold">
              {roundToDecimals(currentPos, 2)} mm
            </span>
          </div>
          <div className="flex flex-col items-end">
            <span className="text-xs font-medium text-muted-foreground">
              Anfahrt
            </span>
            <div className="flex items-center gap-1">
              <Input
                type="number"
                min={1}
                max={250}
                step={1}
                value={gotoSpeed}
                onChange={(e) => {
                  const v = parseFloat(e.target.value);
                  if (!isNaN(v)) setGotoSpeed(v);
                }}
                className="w-20 text-right"
              />
              <span className="text-sm text-muted-foreground">mm/s</span>
            </div>
          </div>
        </div>

        {/* Teach-Slots: Anfahren ist die grosse Haupt-Taste, Speichern und
            Löschen sind bewusst klein und gleich gross daneben. */}
        <div className="flex flex-col gap-2">
          {slots.map(({ slot, label }) => {
            const savedMm = getTeachPositionMm(axisIndex, slot);
            const hasValue = savedMm !== null;

            return (
              <div
                key={slot}
                className="flex items-center gap-2 rounded-md border p-2"
              >
                <div className="flex w-24 shrink-0 flex-col">
                  <span className="text-sm font-semibold">{label}</span>
                  <span className="font-mono text-base">
                    {hasValue ? `${roundToDecimals(savedMm, 2)} mm` : "—"}
                  </span>
                </div>

                <TouchButton
                  variant="default"
                  icon="lu:Crosshair"
                  onClick={() => goToTeachPosition(axisIndex, slot, gotoSpeed)}
                  disabled={isDisabled || !hasValue}
                  className="h-16 flex-1 bg-blue-600 text-lg font-bold hover:bg-blue-700"
                >
                  Anfahren
                </TouchButton>

                <TouchButton
                  variant="outline"
                  icon="lu:Save"
                  onClick={() => saveTeachPosition(axisIndex, slot)}
                  disabled={isDisabled}
                  title="Aktuelle Position speichern"
                />
                <TouchButton
                  variant="destructive"
                  icon="lu:Trash2"
                  onClick={() => clearTeachPosition(axisIndex, slot)}
                  disabled={isDisabled || !hasValue}
                  title="Position löschen"
                />
              </div>
            );
          })}
        </div>

        {/* Soft-Limits: Min + Max kompakt. "Setzen" = aktuelle Position
            übernehmen, "Löschen" = kein Limit. */}
        <div className="flex flex-col gap-2">
          <SoftLimitRow
            label="Soft-Limit Min"
            value={softLimitMin}
            onTeach={() => teachSoftLimitMin(axisIndex)}
            onClear={() => setSoftLimitMin(axisIndex, null)}
            disabled={isDisabled}
          />
          <SoftLimitRow
            label="Soft-Limit Max"
            value={softLimitMax}
            onTeach={() => teachSoftLimitMax(axisIndex)}
            onClear={() => setSoftLimitMax(axisIndex, null)}
            disabled={isDisabled}
          />
        </div>
      </div>
    </ControlCard>
  );
}

export function BbmAutomatikV2KalibrierungPage() {
  return (
    <Page>
      <ControlGrid columns={2}>
        <AxisCalibration axisIndex={AXIS.MT} axisName={AXIS_NAMES[AXIS.MT]} />
        <AxisCalibration
          axisIndex={AXIS.SCHIEBER}
          axisName={AXIS_NAMES[AXIS.SCHIEBER]}
        />
        <AxisCalibration
          axisIndex={AXIS.DRUECKER}
          axisName={AXIS_NAMES[AXIS.DRUECKER]}
        />
      </ControlGrid>
    </Page>
  );
}
