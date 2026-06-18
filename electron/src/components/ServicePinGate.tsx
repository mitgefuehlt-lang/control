import React, { useEffect, useState } from "react";
import { create } from "zustand";
import { Page } from "@/components/Page";
import { TouchNumpad } from "@/components/touch/TouchNumpad";
import { Icon } from "@/components/Icon";

// Service-PIN, der die geschuetzten Bereiche entsperrt (BBM-Motoren/Test/
// Aktoren/Kalibrierung sowie der komplette Setup-Bereich). Bewusst hardcoded
// — bei Aenderung neuen Deploy ausrollen.
const SERVICE_PIN = "1357";

// Idle-Timeout: nach dieser Dauer ohne Bedienung sperrt sich der geschuetzte
// Bereich automatisch wieder.
export const SERVICE_PIN_IDLE_TIMEOUT_MS = 10 * 60 * 1000; // 10 Minuten

type ServicePinState = {
  // Zeitpunkt (ms epoch), bis zu dem entsperrt ist. null = gesperrt.
  unlockedUntil: number | null;
};

type ServicePinActions = {
  unlock: () => void;
  touch: () => void;
  lock: () => void;
};

// Ein einziger, app-weiter Store: einmal entsperrt gilt fuer ALLE geschuetzten
// Bereiche (BBM-Seiten + Setup), kein erneuter PIN-Prompt beim Wechseln.
export const useServicePinStore = create<ServicePinState & ServicePinActions>()(
  (set) => ({
    unlockedUntil: null,
    unlock: () => set({ unlockedUntil: Date.now() + SERVICE_PIN_IDLE_TIMEOUT_MS }),
    touch: () =>
      set((s) =>
        s.unlockedUntil !== null
          ? { unlockedUntil: Date.now() + SERVICE_PIN_IDLE_TIMEOUT_MS }
          : s,
      ),
    lock: () => set({ unlockedUntil: null }),
  }),
);

type Props = {
  children: React.ReactNode;
};

export function ServicePinGate({ children }: Props) {
  const [entered, setEntered] = useState("");
  const [wrong, setWrong] = useState(false);

  const unlockedUntil = useServicePinStore((s) => s.unlockedUntil);
  const unlock = useServicePinStore((s) => s.unlock);
  const touch = useServicePinStore((s) => s.touch);
  const lock = useServicePinStore((s) => s.lock);

  const unlocked = unlockedUntil !== null && Date.now() < unlockedUntil;

  // Solange entsperrt: Bedienung verlaengert das Idle-Fenster, ein Timer
  // sperrt nach Ablauf automatisch wieder. Effekt haengt nur an "ist entsperrt"
  // (Boolean), damit das staendige touch() ihn nicht neu aufsetzt.
  const isActive = unlockedUntil !== null;
  useEffect(() => {
    if (!isActive) return;

    const onActivity = () => touch();
    window.addEventListener("pointerdown", onActivity, true);
    window.addEventListener("keydown", onActivity, true);

    const interval = setInterval(() => {
      const until = useServicePinStore.getState().unlockedUntil;
      if (until === null || Date.now() >= until) {
        lock();
      }
    }, 5000);

    return () => {
      window.removeEventListener("pointerdown", onActivity, true);
      window.removeEventListener("keydown", onActivity, true);
      clearInterval(interval);
    };
  }, [isActive, touch, lock]);

  if (unlocked) {
    return <>{children}</>;
  }

  const handleDigit = (digit: string) => {
    if (entered.length >= SERVICE_PIN.length) return;
    const next = entered + digit;
    setWrong(false);
    if (next.length === SERVICE_PIN.length) {
      if (next === SERVICE_PIN) {
        unlock();
        setEntered("");
      } else {
        setWrong(true);
        setEntered("");
      }
      return;
    }
    setEntered(next);
  };

  const handleDelete = () => {
    setWrong(false);
    setEntered((prev) => prev.slice(0, -1));
  };

  return (
    <Page className="items-center">
      <div className="flex w-full max-w-md flex-col items-center gap-8 pt-12">
        <div className="flex flex-col items-center gap-2">
          <Icon name="lu:Lock" className="size-10 text-muted-foreground" />
          <h1 className="text-2xl font-semibold">Geschuetzter Bereich</h1>
          <p className="text-sm text-muted-foreground">
            PIN eingeben, um diesen Bereich zu oeffnen.
          </p>
        </div>

        <div className="flex gap-4">
          {Array.from({ length: SERVICE_PIN.length }).map((_, i) => {
            const filled = i < entered.length;
            return (
              <div
                key={i}
                className={[
                  "size-5 rounded-full border-2 transition-colors",
                  wrong
                    ? "border-destructive bg-destructive"
                    : filled
                      ? "border-foreground bg-foreground"
                      : "border-muted-foreground/40 bg-transparent",
                ].join(" ")}
              />
            );
          })}
        </div>

        {wrong && (
          <p className="text-sm text-destructive">Falscher PIN. Erneut versuchen.</p>
        )}

        <TouchNumpad onDigit={handleDigit} onDelete={handleDelete} />
      </div>
    </Page>
  );
}
