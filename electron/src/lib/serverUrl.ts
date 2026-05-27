// Resolve the QiTech server base URL.
//
// Electron (production) loads index.html via `file://`, so we cannot derive
// the origin from `window.location` and fall back to localhost:3001 where
// the Rust server listens on the mini-PC.
//
// When the same React bundle is served by the Rust server itself
// (browser/tablet via `http://<host>:3001`), use the page origin so the
// client always talks to the host that served it.
export function getServerBaseUrl(): string {
  if (
    typeof window !== "undefined" &&
    (window.location.protocol === "http:" ||
      window.location.protocol === "https:")
  ) {
    return window.location.origin;
  }
  return "http://localhost:3001";
}
