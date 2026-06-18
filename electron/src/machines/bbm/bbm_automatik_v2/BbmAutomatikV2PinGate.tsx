// The BBM protected pages now use the shared, app-wide service PIN gate so a
// single unlock covers both the BBM service pages and the Setup section.
// Kept as a thin re-export so existing route imports stay valid.
export { ServicePinGate as BbmAutomatikV2PinGate } from "@/components/ServicePinGate";
