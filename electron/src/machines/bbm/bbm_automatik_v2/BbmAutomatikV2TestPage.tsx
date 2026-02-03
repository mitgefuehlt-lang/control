import { ControlCard } from "@/control/ControlCard";
import { Page } from "@/components/Page";
import { ControlGrid } from "@/control/ControlGrid";
import { useBbmAutomatikV2 } from "./useBbmAutomatikV2";
import { TouchButton } from "@/components/touch/TouchButton";

export function BbmAutomatikV2TestPage() {
  const {
    isDisabled,
    isLoading,
  } = useBbmAutomatikV2();

  // TODO: Implement test sequences
  const handleSequence1x = () => {
    console.log("1x befüllen");
  };

  const handleSequence5x = () => {
    console.log("5x befüllen");
  };

  const handleSequenceMagazin = () => {
    console.log("1 Magazin (19x)");
  };

  const handleReset = () => {
    console.log("Reset");
  };

  return (
    <Page>
      <ControlGrid columns={2}>
        <ControlCard title="Test-Sequenzen">
          <div className="flex flex-col gap-4">
            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequence1x}
              disabled={isDisabled}
              isLoading={isLoading}
              className="h-14 text-lg bg-blue-600 hover:bg-blue-700"
            >
              1x befüllen
            </TouchButton>

            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequence5x}
              disabled={isDisabled}
              isLoading={isLoading}
              className="h-14 text-lg bg-blue-600 hover:bg-blue-700"
            >
              5x befüllen
            </TouchButton>

            <TouchButton
              variant="default"
              icon="lu:CirclePlay"
              onClick={handleSequenceMagazin}
              disabled={isDisabled}
              isLoading={isLoading}
              className="h-14 text-lg bg-blue-600 hover:bg-blue-700"
            >
              1 Magazin (19x)
            </TouchButton>

            <TouchButton
              variant="outline"
              icon="lu:RotateCcw"
              onClick={handleReset}
              disabled={isDisabled}
              isLoading={isLoading}
              className="h-14 text-lg"
            >
              Reset
            </TouchButton>
          </div>
        </ControlCard>

        <ControlCard title="Info">
          <div className="text-muted-foreground space-y-2">
            <p><strong>1x befüllen:</strong> Eine Filterhülse vereinzeln und in Block einfügen</p>
            <p><strong>5x befüllen:</strong> 5 Filterhülsen nacheinander befüllen</p>
            <p><strong>1 Magazin (19x):</strong> Komplettes Magazin mit 19 Zyklen befüllen</p>
            <p><strong>Reset:</strong> Alle Achsen in Ausgangsposition fahren</p>
            <div className="pt-4 border-t mt-4">
              <p className="text-xs text-yellow-600">
                Hinweis: Test-Sequenzen sind noch nicht implementiert.
              </p>
            </div>
          </div>
        </ControlCard>
      </ControlGrid>
    </Page>
  );
}
