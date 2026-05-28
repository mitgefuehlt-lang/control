import React, { useState } from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import {
  useBbmAutomatikV2,
  AXIS,
  AXIS_NAMES,
} from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { Input } from "@/components/ui/input";
import { roundToDecimals } from "@/lib/decimal";
import { TEACH_SLOT, TeachSlot } from "./bbmAutomatikV2Namespace";

const DEFAULT_GOTO_SPEED_MM_S = 50;

type SlotDescriptor = {
  slot: TeachSlot;
  isCustom: boolean;
};

const SLOTS: SlotDescriptor[] = [
  { slot: TEACH_SLOT.START, isCustom: false },
  { slot: TEACH_SLOT.ZIEL, isCustom: false },
  { slot: TEACH_SLOT.CUSTOM1, isCustom: true },
  { slot: TEACH_SLOT.CUSTOM2, isCustom: true },
];

function slotLabel(
  slot: TeachSlot,
  customName: string | null,
  isCustom: boolean,
): string {
  if (!isCustom) {
    return slot === TEACH_SLOT.START ? "Start" : "Ziel";
  }
  if (customName && customName.length > 0) {
    return customName;
  }
  return slot === TEACH_SLOT.CUSTOM1 ? "Position 1" : "Position 2";
}

type AxisCalibrationProps = {
  axisIndex: number;
  axisName: string;
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
    <div className="flex items-center justify-between gap-2 rounded-md border p-2">
      <div className="flex flex-1 flex-col">
        <span className="text-sm font-medium">{label}</span>
        <span className="font-mono text-lg">
          {hasValue ? `${roundToDecimals(value, 3)} mm` : "—"}
        </span>
      </div>
      <div className="flex gap-2">
        <TouchButton
          variant="default"
          icon="lu:Crosshair"
          onClick={onTeach}
          disabled={disabled}
        >
          Setzen
        </TouchButton>
        <TouchButton
          variant="destructive"
          icon="lu:Trash2"
          onClick={onClear}
          disabled={disabled || !hasValue}
        />
      </div>
    </div>
  );
}

function AxisCalibration({ axisIndex, axisName }: AxisCalibrationProps) {
  const {
    saveTeachPosition,
    clearTeachPosition,
    renameCustomPosition,
    goToTeachPosition,
    getTeachPositionMm,
    getCustomPositionName,
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
  const [renamingSlot, setRenamingSlot] = useState<TeachSlot | null>(null);
  const [renameDraft, setRenameDraft] = useState<string>("");

  const currentPos = getAxisPositionMm(axisIndex) ?? 0;
  const softLimitMax = getAxisSoftLimitMax(axisIndex);
  const softLimitMin = getAxisSoftLimitMin(axisIndex);

  const startRename = (slot: TeachSlot) => {
    setRenamingSlot(slot);
    setRenameDraft(getCustomPositionName(axisIndex, slot) ?? "");
  };

  const commitRename = () => {
    if (renamingSlot && renameDraft.trim().length > 0) {
      renameCustomPosition(axisIndex, renamingSlot, renameDraft.trim());
    }
    setRenamingSlot(null);
    setRenameDraft("");
  };

  const cancelRename = () => {
    setRenamingSlot(null);
    setRenameDraft("");
  };

  return (
    <ControlCard title={axisName}>
      <div className="flex flex-col gap-4">
        {/* Live current position */}
        <div className="flex items-center justify-between rounded-md bg-muted px-4 py-3">
          <span className="text-sm font-medium text-muted-foreground">
            Aktuelle Position
          </span>
          <span className="font-mono text-2xl font-bold">
            {roundToDecimals(currentPos, 3)} mm
          </span>
        </div>

        {/* Goto speed control (shared across slots) */}
        <div className="flex items-center justify-between gap-3">
          <label className="text-sm font-medium">
            Anfahr-Geschwindigkeit
          </label>
          <div className="flex items-center gap-2">
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
              className="w-24 text-right"
            />
            <span className="text-sm text-muted-foreground">mm/s</span>
          </div>
        </div>

        {/* Soft-Limits: Min + Max. Beide via "Setzen" (Teach-in: aktuelle
            Position übernehmen) oder "Löschen" (kein Limit). */}
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

        {/* Slot rows */}
        <div className="flex flex-col gap-3">
          {SLOTS.map(({ slot, isCustom }) => {
            const savedMm = getTeachPositionMm(axisIndex, slot);
            const customName = getCustomPositionName(axisIndex, slot);
            const label = slotLabel(slot, customName, isCustom);
            const hasValue = savedMm !== null;
            const isRenamingThis = renamingSlot === slot;

            return (
              <div
                key={slot}
                className="flex flex-col gap-2 rounded-md border p-3"
              >
                <div className="flex items-center justify-between gap-2">
                  {/* Label / rename input */}
                  {isRenamingThis ? (
                    <div className="flex flex-1 items-center gap-2">
                      <Input
                        autoFocus
                        value={renameDraft}
                        maxLength={32}
                        onChange={(e) => setRenameDraft(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") commitRename();
                          if (e.key === "Escape") cancelRename();
                        }}
                        placeholder="Slot-Name"
                        className="flex-1"
                      />
                      <TouchButton
                        size="sm"
                        onClick={commitRename}
                        disabled={renameDraft.trim().length === 0}
                      >
                        OK
                      </TouchButton>
                      <TouchButton
                        size="sm"
                        variant="outline"
                        onClick={cancelRename}
                      >
                        Abbruch
                      </TouchButton>
                    </div>
                  ) : (
                    <div className="flex flex-1 items-center gap-2">
                      <span className="font-semibold">{label}</span>
                      {isCustom && hasValue && (
                        <TouchButton
                          size="sm"
                          variant="ghost"
                          icon="lu:Pencil"
                          onClick={() => startRename(slot)}
                          disabled={isDisabled}
                        />

                      )}
                    </div>
                  )}

                  {/* Saved value */}
                  <span className="font-mono text-lg">
                    {hasValue ? `${roundToDecimals(savedMm, 3)} mm` : "—"}
                  </span>
                </div>

                {/* Action buttons */}
                {!isRenamingThis && (
                  <div className="flex gap-2">
                    <TouchButton
                      variant="default"
                      icon="lu:Save"
                      onClick={() => saveTeachPosition(axisIndex, slot)}
                      disabled={isDisabled}
                      className="flex-1"
                    >
                      Speichern
                    </TouchButton>
                    <TouchButton
                      variant="outline"
                      icon="lu:Crosshair"
                      onClick={() =>
                        goToTeachPosition(axisIndex, slot, gotoSpeed)
                      }
                      disabled={isDisabled || !hasValue}
                      className="flex-1"
                    >
                      Anfahren
                    </TouchButton>
                    <TouchButton
                      variant="destructive"
                      icon="lu:Trash2"
                      onClick={() => clearTeachPosition(axisIndex, slot)}
                      disabled={isDisabled || !hasValue}
                    />

                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </ControlCard>
  );
}

export function BbmAutomatikV2KalibrierungPage() {
  return (
    <Page>
      <ControlGrid columns={1}>
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
