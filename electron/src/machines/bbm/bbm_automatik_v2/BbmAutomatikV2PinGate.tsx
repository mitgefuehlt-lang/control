import React, { useEffect, useState } from "react";
import { Page } from "@/components/Page";
import { TouchNumpad } from "@/components/touch/TouchNumpad";
import { Icon } from "@/components/Icon";
import { useBbmAutomatikV2PinStore } from "./bbmAutomatikV2PinStore";

// Service-PIN, der die geschuetzten Seiten der BBM Automatik V2 entsperrt.
// Bewusst hardcoded — bei Aenderung neuen Deploy ausrollen.
const BBM_SERVICE_PIN = "1357";

type Props = {
  children: React.ReactNode;
};

export function BbmAutomatikV2PinGate({ children }: Props) {
  const [entered, setEntered] = useState("");
  const [wrong, setWrong] = useState(false);

  // Entsperr-Status liegt im geteilten Store: einmal entsperrt gilt fuer ALLE
  // geschuetzten BBM-Seiten, ein Bereichswechsel fragt die PIN nicht erneut ab.
  const unlockedUntil = useBbmAutomatikV2PinStore((s) => s.unlockedUntil);
  const unlock = useBbmAutomatikV2PinStore((s) => s.unlock);
  const touch = useBbmAutomatikV2PinStore((s) => s.touch);
  const lock = useBbmAutomatikV2PinStore((s) => s.lock);

  const unlocked = unlockedUntil !== null && Date.now() < unlockedUntil;

  // Solange entsperrt: Bedienung verlaengert das Idle-Fenster, und ein Timer
  // sperrt nach Ablauf automatisch wieder. Effekt haengt nur an "ist entsperrt"
  // (Boolean), damit das staendige touch() ihn nicht neu aufsetzt.
  const isActive = unlockedUntil !== null;
  useEffect(() => {
    if (!isActive) return;

    const onActivity = () => touch();
    window.addEventListener("pointerdown", onActivity, true);
    window.addEventListener("keydown", onActivity, true);

    const interval = setInterval(() => {
      const until = useBbmAutomatikV2PinStore.getState().unlockedUntil;
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
    if (entered.length >= BBM_SERVICE_PIN.length) return;
    const next = entered + digit;
    setWrong(false);
    if (next.length === BBM_SERVICE_PIN.length) {
      if (next === BBM_SERVICE_PIN) {
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
            PIN eingeben, um diese Seite zu oeffnen.
          </p>
        </div>

        <div className="flex gap-4">
          {Array.from({ length: BBM_SERVICE_PIN.length }).map((_, i) => {
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
