import React from "react";
import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useSchneidemaschineV0 } from "./useSchneidemaschineV0";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export function SchneidemaschineV0ControlPage() {
  const { state, liveValues, toggleOutput, isDisabled } =
    useSchneidemaschineV0();

  // Get output state for DO0
  const output0 = state?.output_states[0] ?? false;

  return (
    <Page>
      <ControlGrid columns={2}>
        {/* Digital Inputs */}
        <ControlCard title="Digitale Eingänge">
          <div className="grid grid-cols-4 gap-3">
            {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => {
              const inputState = liveValues?.input_states[index] ?? false;
              return (
                <div key={index} className="flex flex-col items-center gap-1">
                  <span className="text-xs text-muted-foreground">
                    DI{index}
                  </span>
                  <Badge
                    className={
                      inputState
                        ? "bg-green-600 hover:bg-green-600"
                        : "bg-gray-400 hover:bg-gray-400"
                    }
                  >
                    {inputState ? "HIGH" : "LOW"}
                  </Badge>
                </div>
              );
            })}
          </div>
        </ControlCard>

        {/* Digital Output */}
        <ControlCard title="Digitaler Ausgang">
          <div className="flex flex-col items-center gap-4">
            <Button
              size="lg"
              variant={output0 ? "default" : "outline"}
              disabled={isDisabled}
              onClick={() => toggleOutput(0)}
              className="h-20 w-40 text-lg"
            >
              {output0 ? "AN" : "AUS"}
            </Button>
            <span className="text-sm text-muted-foreground">
              Digital Output 0
            </span>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
