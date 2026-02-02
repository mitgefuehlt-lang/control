import { Topbar } from "@/components/Topbar";
import { schneidemaschineV0SerialRoute } from "@/routes/routes";

export function SchneidemaschineV0Page() {
  const { serial } = schneidemaschineV0SerialRoute.useParams();

  return (
    <Topbar
      pathname={`/_sidebar/machines/schneidemaschine_v0/${serial}`}
      items={[
        {
          link: "control",
          activeLink: "control",
          title: "Control",
          icon: "lu:CirclePlay",
        },
      ]}
    />
  );
}
