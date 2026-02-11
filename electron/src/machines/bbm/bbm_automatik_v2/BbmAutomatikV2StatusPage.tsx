import React from "react";
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
  const { state, liveValues, getAxisPositionMm, getAxisSpeedMmS, INPUT } =
    useBbmAutomatikV2();

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
      "Endlage MT",
      "Endlage Schieber",
      "Endlage Drücker",
      "Alarm MT",
      "Alarm Schieber",
      "Alarm Drücker",
      "Tür",
    ];

    // Log door sensor changes
    const tuer = liveValues.input_states[INPUT.TUER];

    // This will be called on every render, so we need to track previous state
    // For now, just show current state in the log on mount
  }, [liveValues, INPUT]);

  // Sensor names
  const inputNames = [
    "Endlage MT",
    "Endlage Schieber",
    "Endlage Drücker",
    "Alarm MT",
    "Alarm Schieber",
    "Alarm Drücker",
    "Tür",
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
          <div className="h-64 overflow-y-auto rounded bg-gray-50 p-2 font-mono text-sm">
            {logEntries.length === 0 ? (
              <div className="py-4 text-center text-gray-400">
                Keine Meldungen
              </div>
            ) : (
              logEntries.map((entry, index) => (
                <div
                  key={index}
                  className={`flex gap-2 border-b border-gray-100 py-1 last:border-0 ${getLogEntryColor(entry.type)}`}
                >
                  <span className="shrink-0 text-gray-400">
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
                  className={`flex items-center gap-2 rounded px-3 py-2 text-sm ${
                    isActive
                      ? "bg-green-100 text-green-800"
                      : "bg-gray-100 text-gray-600"
                  }`}
                >
                  <div
                    className={`h-2 w-2 rounded-full ${
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
                <div key={index} className="bg-muted rounded p-3">
                  <div className="text-muted-foreground text-sm">{name}</div>
                  <div className="font-mono text-xl">
                    {roundToDecimals(position, 1)} mm
                  </div>
                  <div
                    className={`text-xs ${isMoving ? "text-green-600" : "text-muted-foreground"}`}
                  >
                    {isMoving
                      ? `${roundToDecimals(speed, 1)} mm/s`
                      : "Gestoppt"}
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
                  className={`flex items-center gap-2 rounded px-3 py-2 text-sm ${
                    isActive
                      ? "bg-blue-100 text-blue-800"
                      : "bg-gray-100 text-gray-600"
                  }`}
                >
                  <div
                    className={`h-2 w-2 rounded-full ${
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
