import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2, AXIS_NAMES } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";
import { roundToDecimals } from "@/lib/decimal";

export function BbmAutomatikV2StatusPage() {
  const {
    state,
    liveValues,
    setAmpel,
    getAxisPositionMm,
    getAxisSpeedMmS,
    isDisabled,
    isLoading,
    INPUT,
    OUTPUT,
  } = useBbmAutomatikV2();

  // Ampel state
  const ampelRot = state?.output_states[OUTPUT.AMPEL_ROT] ?? false;
  const ampelGelb = state?.output_states[OUTPUT.AMPEL_GELB] ?? false;
  const ampelGruen = state?.output_states[OUTPUT.AMPEL_GRUEN] ?? false;

  // Rüttelmotor
  const ruettelmotorOn = state?.output_states[OUTPUT.RUETTELMOTOR] ?? false;

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

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Ampel Control */}
        <ControlCard title="Ampel">
          <div className="flex flex-col gap-4">
            <div className="flex gap-4 justify-center">
              {/* Ampel visualization */}
              <div className="flex flex-col gap-2 p-4 bg-gray-800 rounded-lg">
                <div
                  className={`w-12 h-12 rounded-full ${
                    ampelRot ? "bg-red-500 shadow-lg shadow-red-500/50" : "bg-red-900"
                  }`}
                />
                <div
                  className={`w-12 h-12 rounded-full ${
                    ampelGelb ? "bg-yellow-500 shadow-lg shadow-yellow-500/50" : "bg-yellow-900"
                  }`}
                />
                <div
                  className={`w-12 h-12 rounded-full ${
                    ampelGruen ? "bg-green-500 shadow-lg shadow-green-500/50" : "bg-green-900"
                  }`}
                />
              </div>
            </div>

            {/* Ampel buttons */}
            <div className="flex gap-2">
              <TouchButton
                variant={ampelRot ? "default" : "outline"}
                onClick={() => setAmpel(!ampelRot, ampelGelb, ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`flex-1 h-10 ${ampelRot ? "bg-red-600 hover:bg-red-700" : ""}`}
              >
                Rot
              </TouchButton>
              <TouchButton
                variant={ampelGelb ? "default" : "outline"}
                onClick={() => setAmpel(ampelRot, !ampelGelb, ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`flex-1 h-10 ${ampelGelb ? "bg-yellow-600 hover:bg-yellow-700" : ""}`}
              >
                Gelb
              </TouchButton>
              <TouchButton
                variant={ampelGruen ? "default" : "outline"}
                onClick={() => setAmpel(ampelRot, ampelGelb, !ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`flex-1 h-10 ${ampelGruen ? "bg-green-600 hover:bg-green-700" : ""}`}
              >
                Grün
              </TouchButton>
            </div>

            {/* Quick presets */}
            <div className="flex gap-2 pt-2 border-t">
              <TouchButton
                variant="outline"
                onClick={() => setAmpel(false, false, true)}
                disabled={isDisabled}
                className="flex-1 h-8 text-sm"
              >
                Bereit
              </TouchButton>
              <TouchButton
                variant="outline"
                onClick={() => setAmpel(false, true, false)}
                disabled={isDisabled}
                className="flex-1 h-8 text-sm"
              >
                Läuft
              </TouchButton>
              <TouchButton
                variant="outline"
                onClick={() => setAmpel(true, false, false)}
                disabled={isDisabled}
                className="flex-1 h-8 text-sm"
              >
                Fehler
              </TouchButton>
              <TouchButton
                variant="outline"
                onClick={() => setAmpel(false, false, false)}
                disabled={isDisabled}
                className="flex-1 h-8 text-sm"
              >
                Aus
              </TouchButton>
            </div>
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
        <ControlCard title="Digitale Ausgänge">
          <div className="grid grid-cols-2 gap-2">
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
