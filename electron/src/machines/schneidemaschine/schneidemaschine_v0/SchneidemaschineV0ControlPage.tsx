import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useSchneidemaschineV0 } from "./useSchneidemaschineV0";
import { Button } from "@/components/ui/button";

export function SchneidemaschineV0ControlPage() {
  const { state, toggleOutput, isDisabled } = useSchneidemaschineV0();

  // Get output state for DO0
  const output0 = state?.output_states[0] ?? false;

  return (
    <Page>
      <ControlGrid columns={2}>
        <ControlCard title="Taster 1">
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
