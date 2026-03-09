import React from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2 } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";

export function BbmAutomatikV2ActuatorsPage() {
  const {
    state,
    setRuettelmotor,
    setPneumatik,
    setAmpel,
    isDisabled,
    isLoading,
    OUTPUT,
  } = useBbmAutomatikV2();

  const ruettelmotorOn = state?.output_states[OUTPUT.RUETTELMOTOR] ?? false;
  const pneumatikOn = state?.output_states[OUTPUT.PNEUMATIK] ?? false;
  const ampelRot = state?.output_states[OUTPUT.AMPEL_ROT] ?? false;
  const ampelGelb = state?.output_states[OUTPUT.AMPEL_GELB] ?? false;
  const ampelGruen = state?.output_states[OUTPUT.AMPEL_GRUEN] ?? false;

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Pneumatik Ventil */}
        <ControlCard title="Pneumatik Ventil">
          <div className="flex flex-col gap-4">
            <TouchButton
              variant={pneumatikOn ? "destructive" : "default"}
              icon={pneumatikOn ? "lu:Square" : "lu:Play"}
              onClick={() => setPneumatik(!pneumatikOn)}
              disabled={isDisabled}
              isLoading={isLoading}
              className={`h-14 text-lg ${pneumatikOn ? "" : "bg-green-600 hover:bg-green-700"}`}
            >
              {pneumatikOn ? "ZU" : "AUF"}
            </TouchButton>

            {pneumatikOn && (
              <div className="animate-pulse text-center font-semibold text-green-600">
                Ventil offen
              </div>
            )}
          </div>
        </ControlCard>

        {/* Rüttelmotor */}
        <ControlCard title="Rüttelmotor">
          <div className="flex flex-col gap-4">
            <TouchButton
              variant={ruettelmotorOn ? "destructive" : "default"}
              icon={ruettelmotorOn ? "lu:Square" : "lu:Play"}
              onClick={() => setRuettelmotor(!ruettelmotorOn)}
              disabled={isDisabled}
              isLoading={isLoading}
              className={`h-14 text-lg ${ruettelmotorOn ? "" : "bg-green-600 hover:bg-green-700"}`}
            >
              {ruettelmotorOn ? "AUS" : "AN"}
            </TouchButton>

            {ruettelmotorOn && (
              <div className="animate-pulse text-center font-semibold text-green-600">
                Rüttelmotor aktiv
              </div>
            )}
          </div>
        </ControlCard>

        {/* Ampel */}
        <ControlCard title="Ampel">
          <div className="flex flex-col gap-4">
            <div className="flex gap-2">
              <TouchButton
                variant={ampelRot ? "default" : "outline"}
                onClick={() => setAmpel(!ampelRot, ampelGelb, ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`h-14 flex-1 text-lg ${ampelRot ? "bg-red-600 hover:bg-red-700" : ""}`}
              >
                Rot
              </TouchButton>

              <TouchButton
                variant={ampelGelb ? "default" : "outline"}
                onClick={() => setAmpel(ampelRot, !ampelGelb, ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`h-14 flex-1 text-lg ${ampelGelb ? "bg-yellow-500 text-black hover:bg-yellow-600" : ""}`}
              >
                Gelb
              </TouchButton>

              <TouchButton
                variant={ampelGruen ? "default" : "outline"}
                onClick={() => setAmpel(ampelRot, ampelGelb, !ampelGruen)}
                disabled={isDisabled}
                isLoading={isLoading}
                className={`h-14 flex-1 text-lg ${ampelGruen ? "bg-green-600 hover:bg-green-700" : ""}`}
              >
                Grün
              </TouchButton>
            </div>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
