import { Topbar } from "@/components/Topbar";
import { bbmAutomatikV2SerialRoute } from "@/routes/routes";

export function BbmAutomatikV2Page() {
  const { serial } = bbmAutomatikV2SerialRoute.useParams();

  return (
    <Topbar
      pathname={`/_sidebar/machines/bbm_automatik_v2/${serial}`}
      items={[
        {
          link: "motors",
          activeLink: "motors",
          title: "Motoren",
          icon: "lu:Cog",
        },
        {
          link: "test",
          activeLink: "test",
          title: "Test",
          icon: "lu:FlaskConical",
        },
        {
          link: "auto",
          activeLink: "auto",
          title: "Automatik",
          icon: "lu:Play",
        },
        {
          link: "status",
          activeLink: "status",
          title: "Status",
          icon: "lu:Activity",
        },
      ]}
    />
  );
}
