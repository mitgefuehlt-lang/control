import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS_NAMES } from "./useBbmAutomatikV2";
import { roundToDecimals } from "@/lib/decimal";
import { useEffect, useState } from "react";

interface LogEntry {
  timestamp: Date;
  message: string;
  type: "info" | "warning" | "error" | "success";
}

export function BbmAutomatikV2StatusPage() {
  const {
    state,
    liveValues,
    getAxisPositionMm,
    getAxisSpeedMmS,
    INPUT,
  } = useBbmAutomatikV2();

  // Log entries state
  const [logEntries, setLogEntries] = useState<LogEntry[]>([
    { timestamp: new Date(), message: "Status-Seite geladen", type: "info" },
  ]);

  // Add log entry helper
  const addLogEntry = (message: string, type: LogEntry["type"] = "info") => {
    setLogEntries((prev) => [
      { timestamp: new Date(), message, type },
      ...prev.slice(0, 99), // Keep last 100 entries
    ]);
  };

  // Monitor input changes and log them
  useEffect(() => {
    if (!liveValues) return;

    const inputNames = [
      "Referenz MT",
      "Referenz Schieber",
      "Referenz Drücker",
      "Tür 1",
      "Tür 2",
    ];

    // Log door sensor changes
    const tuer1 = liveValues.input_states[INPUT.TUER_1];
    const tuer2 = liveValues.input_states[INPUT.TUER_2];

    // This will be called on every render, so we need to track previous state
    // For now, just show current state in the log on mount
  }, [liveValues, INPUT]);

  // Sensor names
  const inputNames = [
    "Referenz MT",
    "Referenz Schieber",
    "Referenz Drücker",
    "Tür 1",
    "Tür 2",
    "DI 6 (frei)",
    "DI 7 (frei)",
    "DI 8 (frei)",
  ];

  // Format timestamp
  const formatTimestamp = (date: Date) => {
    return date.toLocaleString("de-DE", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  // Get log entry color based on type
  const getLogEntryColor = (type: LogEntry["type"]) => {
    switch (type) {
      case "error":
        return "text-red-600";
      case "warning":
        return "text-yellow-600";
      case "success":
        return "text-green-600";
      default:
        return "text-gray-600";
    }
  };

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Meldungs-Log */}
        <ControlCard title="Meldungen" className="col-span-2">
          <div className="h-64 overflow-y-auto bg-gray-50 rounded p-2 font-mono text-sm">
            {logEntries.length === 0 ? (
              <div className="text-gray-400 text-center py-4">
                Keine Meldungen
              </div>
            ) : (
              logEntries.map((entry, index) => (
                <div
                  key={index}
                  className={`flex gap-2 py-1 border-b border-gray-100 last:border-0 ${getLogEntryColor(entry.type)}`}
                >
                  <span className="text-gray-400 shrink-0">
                    {formatTimestamp(entry.timestamp)}
                  </span>
                  <span>{entry.message}</span>
                </div>
              ))
            )}
          </div>
        </ControlCard>

        {/* Sensoren */}
        <ControlCard title="Digitale Eingänge">
          <div className="grid grid-cols-2 gap-2">
            {inputNames.map((name, index) => {
              const isActive = liveValues?.input_states[index] ?? false;
              return (
                <div
                  key={index}
                  className={`flex items-center gap-2 px-3 py-2 rounded text-sm ${
                    isActive ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-600"
                  }`}
                >
                  <div
                    className={`w-2 h-2 rounded-full ${
                      isActive ? "bg-green-500" : "bg-gray-400"
                    }`}
                  />
                  {name}
                </div>
              );
            })}
          </div>
        </ControlCard>

        {/* Achsen-Positionen */}
        <ControlCard title="Achsen-Positionen">
          <div className="grid grid-cols-2 gap-4">
            {AXIS_NAMES.map((name, index) => {
              const position = getAxisPositionMm(index) ?? 0;
              const speed = getAxisSpeedMmS(index) ?? 0;
              const isMoving = speed !== 0;
              return (
                <div key={index} className="p-3 bg-muted rounded">
                  <div className="text-sm text-muted-foreground">{name}</div>
                  <div className="font-mono text-xl">
                    {roundToDecimals(position, 1)} mm
                  </div>
                  <div className={`text-xs ${isMoving ? "text-green-600" : "text-muted-foreground"}`}>
                    {isMoving ? `${roundToDecimals(speed, 1)} mm/s` : "Gestoppt"}
                  </div>
                </div>
              );
            })}
          </div>
        </ControlCard>

        {/* Ausgänge Status */}
        <ControlCard title="Digitale Ausgänge" className="col-span-2">
          <div className="grid grid-cols-4 gap-2">
            {[
              "Rüttelmotor",
              "Ampel Rot",
              "Ampel Gelb",
              "Ampel Grün",
              "DO 5 (frei)",
              "DO 6 (frei)",
              "DO 7 (frei)",
              "DO 8 (frei)",
            ].map((name, index) => {
              const isActive = state?.output_states[index] ?? false;
              return (
                <div
                  key={index}
                  className={`flex items-center gap-2 px-3 py-2 rounded text-sm ${
                    isActive ? "bg-blue-100 text-blue-800" : "bg-gray-100 text-gray-600"
                  }`}
                >
                  <div
                    className={`w-2 h-2 rounded-full ${
                      isActive ? "bg-blue-500" : "bg-gray-400"
                    }`}
                  />
                  {name}
                </div>
              );
            })}
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
