import { create } from "zustand";

// Idle-Timeout: nach dieser Dauer ohne Bedienung sperrt sich der geschuetzte
// Bereich automatisch wieder. Bewusst kurz genug fuer Sicherheit, lang genug
// dass normales Arbeiten nicht staendig die PIN-Abfrage triggert.
export const BBM_PIN_IDLE_TIMEOUT_MS = 10 * 60 * 1000; // 10 Minuten

export type BbmAutomatikV2PinState = {
  // Zeitpunkt (ms epoch), bis zu dem der Bereich entsperrt ist. null = gesperrt.
  unlockedUntil: number | null;
};

export type BbmAutomatikV2PinActions = {
  // PIN korrekt eingegeben: Bereich fuer das Idle-Fenster oeffnen.
  unlock: () => void;
  // Bedienung erkannt: Idle-Fenster verlaengern (nur wenn bereits entsperrt).
  touch: () => void;
  // Manuell oder per Timeout wieder sperren.
  lock: () => void;
};

export type BbmAutomatikV2PinStore = BbmAutomatikV2PinState &
  BbmAutomatikV2PinActions;

export const useBbmAutomatikV2PinStore = create<BbmAutomatikV2PinStore>()(
  (set) => ({
    unlockedUntil: null,

    unlock: () => set({ unlockedUntil: Date.now() + BBM_PIN_IDLE_TIMEOUT_MS }),

    touch: () =>
      set((s) =>
        s.unlockedUntil !== null
          ? { unlockedUntil: Date.now() + BBM_PIN_IDLE_TIMEOUT_MS }
          : s,
      ),

    lock: () => set({ unlockedUntil: null }),
  }),
);
