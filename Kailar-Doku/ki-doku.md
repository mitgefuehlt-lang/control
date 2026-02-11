# KI-Doku - QiTech Control

## Zweck
- Dieses Dokument sammelt den aktuellen Kenntnisstand ueber das Repo.
- Es ist eine lebende Datei: jede neue Erkenntnis oder Aenderung hier dokumentieren.
- Nicht als Garantie fuer 100% Vollstaendigkeit.

## Update-Regeln
- Neue Erkenntnis zuerst hier eintragen (Kurzfassung + betroffene Dateien/Module).
- Rueckverweise auf konkrete Pfade nutzen.
- Offene Fragen und Annahmen sammeln, bis sie geklaert sind.
- Keine Binaer-Inhalte kopieren; nur beschreiben.
- Vorgehen nach Dokumentation: `docs/` und `docs/developer-docs/` sind verbindliche Leitlinien (z.B. `docs/developer-docs/adding-a-machine.md` fuer neue Maschinen). Keine Abweichung ohne explizite Entscheidung.

## Repo-Ueberblick (Root)
- `control-core/`: generische Logik (EtherCAT Interface Discovery, Realtime, SocketIO-Caching, Controller).
- `ethercat-hal/`: EtherCAT HAL (PDO, CoE, IO, Device-Implementierungen).
- `server/`: Backend (Control Loop, REST, SocketIO, EtherCAT Setup, Metrics).
- `machines/`: Maschinen-Implementierungen und Registry.
- `units/`: physikalische Einheiten via `uom`.
- `utils/`: kleine Hilfen (Heap-Profiling Allocator).
- `control-core-derive/`, `ethercat-hal-derive/`: Proc-Macros.
- `ethercat-eeprom-dump/`: separates CLI-Tool (nicht im Workspace).
- `electron/`: Electron + React UI.
- `nixos/`: OS-Konfiguration und Pakete.
- `docs/`: Dokumentation.

## Architektur-Kurzfassung
- Schichten: Electron UI -> Server -> control-core + ethercat-hal.
- Kommunikation: SocketIO (Events, msgpack) fuer Streaming; REST (Axum) fuer Mutationen.
- Realtime: eigener Loop-Thread, EtherCAT TX/RX Thread, feste Affinitaeten.

## Backend Details

### server
- Einstieg: `server/src/main.rs`.
- Startet:
  - RT Loop Thread (`server/src/loop.rs`).
  - REST API (Axum, Tokio Single-Thread Runtime).
  - SocketIO Queue Worker.
  - Serial Discovery (USB) und Modbus-TCP Discovery.
  - Runtime Metrics Sampler.
- REST Endpunkte:
  - `/api/v1/write_machine_device_identification`
  - `/api/v1/machine/mutate`
  - `/api/v1/metrics/...`
- `SharedState` verwaltet SocketIO Setup, EtherCAT Meta-Daten, Machine-Mapping.
- EtherCAT Setup (`server/src/ethercat/setup.rs`):
  - Ethercrab TX/RX Thread.
  - Init Subdevices, read EEPROM Identifikation.
  - Gruppierung nach Vendor/Machine/Serial.
  - Maschinenbau ueber Registry.
  - Sonderlogik: Bypass fuer SchneideMaschineV1 bei leerem EEPROM.
  - Wago/IP20 Module werden als "Slots" nachgebaut.

### control-core
- `ethercat/interface_discovery.rs`: findet Interface, setzt nmcli managed/unmanaged.
- `realtime.rs`: set_realtime_priority, lock_memory, set_core_affinity.
- `socketio/namespace.rs`: Event Cache + Queue, Cache-Strategien.
- Controller/Converter/Transmission Module.

### ethercat-hal
- `pdo/`: Tx/Rx PDO Objektierung, Bit-Enc/Dec.
- `coe/`: CoE Konfiguration und ConfigurableDevice.
- `io/`: IO Abstraktionen (Digital/Analog/Stepper/Temperature).
- `devices/`: ELxxxx, WAGO, IP20; mapping via SubDeviceIdentityTuple.
- `shared_config/`, `debugging/`.
- `devices/mod.rs`: Dynamic Factory (Identity -> Device), `Module` Slot-Struktur, `downcast_device`, Dynamic PDO Offsets.
- Digitale IO: EL1002/1008 (DI), EL2002/2004/2008/2024/2634/2809 (DO).
- Analoge IO: EL3001 (0-10V), EL3021/3024 (4-20mA), EL3062-0030 (0-30V), EL3204 (PT100), EL4002 (0-10V).
- PTO/Encoder: EL2521/EL2522 (PTO + Encoder, CoE Config + PDO Presets), EL5152 (Encoder Period/Frequency).
- Serial: EL6021 (MDP600 22-Byte, Baudrate/Encoding Checks, Init Sequenz, Toggle Handling).
- Stepper: EL7031, EL7031-0030, EL7041-0052 (EL70x1 PDOs, Counter Wrapper, CoE Configs, Velocity Mode Checks).
- WAGO/IP20: Wago 750-354 und IP20 EcDi8Do8 lesen Module per SDO, berechnen PDO Offsets, bauen Slot-Devices (WAGO_750_501/652/1506).
- `pdo/*`: konkrete PDO Objekte (el70x1, el252x, el40xx, el5152) + `Limit`/`BoolPdoObject`/`F32PdoObject`.
- `shared_config/*`: CoE Felder fuer EL30xx/EL40xx/EL70x1 (Filter, Presentation, Motor/Pos Settings, Start Types).

### machines
- `Machine`, `MachineAct`, `MachineApi`, `MachineNewTrait`, `MachineChannel`.
- `machine_identification.rs`: Vendor/Machine/Serial/Role in EEPROM Words.
- `registry.rs`: Mapping MachineIdentification -> Constructor.
- Maschinenmodule: `winder2`, `extruder1/2`, `laser`, `buffer1`, `aquapath1`, `wago_power`, `test_machine`, `schneidemaschine_v1`, `mock`, `serial/*`.
- Serial Detection via `serialport` (USB VID/PID).
- `lib.rs`: `MachineNewHardware` (EtherCAT/Serial), role/identity Validierung, EtherCAT Device Lookup.
- `machine_identification.rs`: EEPROM Adressen pro Device Identity (Default 0x28-0x2b), read/write, Unknown -> Error.
- `registry.rs`: Registry auf MachineIdentification, baut Maschinen aus Gruppen.
- `serial/devices/*`: Laser (Modbus), Mock, Extruder/Winder Mock mit hash-basierter Seriennummer aus Pfad.

#### Maschinen-Details (Auszug)
- `winder2`: Spool/Puller/Traverse Controller, tension arm, filament tension, clamp revolution helper + Tests; viele Mutationen (Traverse, Puller, Spool, Tension).
- `extruder1` (V2) + `extruder2` (V3): Mitsubishi CS80 via EL6021, PID Temperatur, Screw Speed Controller (Pressure/RPM), Energie-Tracking, State/Live-Emit mit Hash-Cache; Mock Varianten vorhanden.
- `aquapath1`: PID fuer Temperatur (Heizen/Kuehlen), PWM Ausgaenge, Flow aus Encoder; EL5152 Konfig.
- `buffer1`: Stepper/BLDC Konfig, Standby/Filling/Emptying; Buffer Tower Controller fuer Speed.
- `laser`: Serial Modbus Laser; tolerance/roundness Logik; Drop disconnects.
- `wago_power`: Modbus TCP, 24V On/Off via Holding Register.
- `test_machine`/`schneidemaschine_v1`: EL1008/EL2008/EL2522; Schneidemaschine ohne EL2522 CoE Konfig (SDO Fehler).
- `analog_input_test_machine`/`ip20_test_machine`: einfache Ein-/Ausgaenge + State/Live Emit.

### units
- `uom`-System, ISQ + Einheiten.

### utils
- Heap Profiling Allocator (Feature `heap-profile`).

### derive crates
- `control-core-derive`: `BuildEvent`, `Machine`.
- `ethercat-hal-derive`: `RxPdo`, `TxPdo`, `PdoObject`, `EthercatDevice`.

## Frontend (electron)
- Main Process: `electron/src/main.ts` (BrowserWindow, preload, devtools, single instance).
- Preload: `electron/src/preload.ts` -> contextBridge exposes IPC contexts.
- Renderer: `electron/src/App.tsx` mit TanStack Router, i18n, global logging.
- Routing: `electron/src/routes/routes.tsx` (Memory History, initial /_sidebar/setup/ethercat).
- Sidebar: `electron/src/components/SidebarLayout.tsx` listet Maschinen; Connection Guard.
- SocketIO: `electron/src/client/socketioStore.ts` (msgpack, zod, throttled updates ~30 FPS).
- Main namespace events: `electron/src/client/mainNamespace.ts`.
- REST Client: `electron/src/client/useClient.tsx`.
- Machine Meta: `electron/src/machines/properties.ts` (Identifikation + erlaubte EtherCAT Devices).
- Styling: `electron/src/styles/global.css` (Tailwind v4, Fonts Sora/Consequences/Geist Mono).
- Update Pipeline: `electron/src/helpers/ipc/update/update-listeners.ts`:
  - Clone Repo, run `nixos-install.sh`, progress parsing, cancel via tree-kill.

## NixOS / OS
- Module: `nixos/modules/qitech.nix` (systemd service, capabilities, realtime limits, udev, firewall).
- Packages: `nixos/packages/server.nix` (features `tracing-journald,io-uring`), `nixos/packages/electron.nix` (wrapper, /var/lib/qitech).
- OS Config: `nixos/os/configuration.nix`:
  - preempt=full, isolcpus, nohz_full, rcu_nocbs.
  - GNOME kiosk, autologin, power mgmt off.
  - QITECH_OS env vars und gitInfo.
- Update: `nixos-install.sh` sammelt Git Info, `nixos-rebuild boot`, reboot.

## CI/CD und Deploy (GitHub Actions)
- ` .github/workflows/deploy.yml`: manuell (workflow_dispatch); Tailscale (OAuth), SSH auf `konrad@nixos`, `nixos-rebuild switch --flake .` im Runner-Checkout (`/run/github-runner/...`), danach `systemctl is-active` Check.
- ` .github/workflows/fast-deploy.yml`: automatisch bei Push auf `master` mit Rust/Electron Aenderungen; baut `server` release + Electron UI, scp nach `/var/lib/qitech`, patchelf via nix-shell (Interpreter/RPATH), restart `qitech-control-server`, Health Check via `systemctl is-active`/journal.
- ` .github/workflows/nix.yml`: Nix CI (build Electron + Server + System Config), `nix flake check`, Nix formatting (`nixfmt-classic`).
- ` .github/workflows/rust.yml`: Cargo build/test/fmt + mock build (features `development-build,mock-machine`).
- ` .github/workflows/electron.yml`: UI build/test/lint/format, aber auf Branch `main` (abweichend von `master` im Repo).
- Doku-Referenz: `docs/developer-docs/getting-started.md` beschreibt Contribution-Flow explizit auf `master` (rebase/push/merge). Kein Hinweis in den Docs auf `main`.

## Scripts
- `cargo_run_linux.sh`: build + setcap + /dev/ttyUSB* perms, start server.
- `compile_nix_pkgs.sh`: build + cache + sign.
- `compile_metrics.sh`: build metrics -> CSV.
- `generate-installinfo.sh`: schreibt /tmp/installInfo.nix.
- `docker-nix.sh`: Nix container.

## Change Impact Map (kritisch)
- EtherCAT Device/CoE/PDO Aenderung -> device mapping, IO layer, machine validation, UI device roles.
- Machine Identification Adressen/Role Mapping -> EEPROM write/read + UI assignment.
- RT Loop Timing/Affinity -> determinism, jitter, metrics.
- SocketIO Event Schema -> Zod Validation im UI + Caching.
- Update Pipeline (rmSync/clone/nixos-install) -> On-device stability und Datenverlust.
- NixOS Module/Service -> capabilities, realtime, udev, firewall.

## Offene Punkte / Nicht gelesen
- Binaerdateien (PDF/PNG/DRAWIO) nur gelistet, nicht analysiert.
- Docs zu Electrical Diagrams und Maschinen-Handbuechern nicht ausgewertet.
- Tests nicht ausgefuehrt.

## Docs (weitere Inhalte)
- `docs/threading.md`: Thread-Modell (Main, Api, Ethercat Interface Tests, Loop, TxRx), TODO Realtime fuer Threads.
- `docs/troubleshooting.md`: EtherCAT-Fehlerbilder (keine Terminals), Firmware-Reflash, Inverter-Settings.
- `docs/devices.md`: Checkliste Device Implementierung, PDO/CoE/Identity Schritte.
- `docs/ethercat-basics.md`: State Machine, SDO/PDO/EEPROM, Adressierung, Topologien.
- `docs/mitsubishi_inverter.md` + `docs/wiring_mitsubishi.md`: Modbus Settings + Verdrahtung EL6021 <-> Inverter.
- `docs/machines/laser-DRE.md`: Laser DRE Modbus RTU, 38400/8N1, Polling 16ms.
- `docs/developer-docs/*`: Minimal-Examples EL2004/EL3021, Code Style, Machine-Setup, Presets (Zod + Migration), Performance/Stability, Memtest, XTREM Protocol.
- `docs/nixos/quick-start.md` + `docs/nixos/details.md`: Setup, Update, Service-Management, Nix Flake/Module Details.
- Binaer-Assets: `docs/assets/*` (png/jpg/jpeg), `docs/drawio/*.drawio`, `docs/machines/*.pdf`, `docs/electrical-diagrams/*/*.pdf` nur gelistet.
  - Verbindlich fuer Maschinen: `docs/developer-docs/adding-a-machine.md` (Struktur: `mod.rs`, `new.rs`, `act.rs`, `api.rs`, Registry/ID).

### Drawio Zusammenfassungen
- `docs/drawio/architecture-overview.drawio`: Architekturfluss Electron -> Server -> control-core -> ethercat-hal -> devices/pdo/io; Winder2 Beispiel (Pages/Components/Client Cache/Namespace/Events/Mutations); Actor-Layer (Digital Output Setter, Stepper Pulse Train, Analog Input Getter) mapped auf IO/Devices/PDO.
- `docs/drawio/control-loop.drawio`: Threads: EthercatInterfaceThread (Discover Interfaces -> Test Interface -> Create machines) und LoopThread (Setup -> TX/RX -> Read Inputs -> act() -> Write Inputs); Daten: machines + ethercat_devices als Zylinder; EtherCAT cloud.
- `docs/drawio/io-example.drawio`: IO Abstraktion (Digital Output) und zwei Moeglichkeiten der Zuordnung zu Devices (EL2002 vs EL2004), mit "Functionality XY".
- `docs/drawio/pdo.drawio`: Beispiel EL2521/EL3001/EL2024 PDO Assignment, Tx/Rx Pdo, PDO Objects und Content; zeigt Predefined PDO Assignment (Standard/Compact) auf konkrete PDO Objects.
- `docs/drawio/serial_device.drawio`: Serial Device Detection/Recognition Flow (Detect -> Compare Added/Removed -> Delete Removed -> Connect -> Add New Devices -> Global HashMap).

### Binaer-Inventar (nicht inhaltlich analysiert)
- Count: 4 PDF, 19 PNG, 7 JPG, 8 JPEG, 5 DRAWIO (alle Drawio gelesen).
- PDFs:
  - `docs/electrical-diagrams/extruder/Nozzle.pdf` (126144 bytes)
  - `docs/electrical-diagrams/extruder/QiTech_Pro_Extruder_Electrical_Diagram_2025.pdf` (2891342 bytes)
  - `docs/electrical-diagrams/winder/QiTech_Pro_Winder_Electrical_Diagram.pdf` (1669240 bytes)
  - `docs/machines/Usermanual Winder picture.pdf` (2804191 bytes)
- Images liegen unter `docs/assets/*` (png/jpg/jpeg); nicht inhaltlich geprueft.

## Fortschrittslog
- 2026-01-22: Initiale Bestandsaufnahme (Root, Backend Kernmodule, Frontend Kernmodule, NixOS + Scripts, zentrale Docs).
- 2026-01-22: Maschinen-Crate vollstaendig gelesen (lib/ident/registry + alle Module inkl. Serial Mocks).
- 2026-01-22: Ethercat-HAL devices/io/pdo/shared_config im Detail gelesen.
- 2026-01-22: Alle Markdown-Dokus gelesen (Rest in Binaer-Assets verbleibend).
- 2026-01-22: Drawio Diagramme gelesen und zusammengefasst.
- 2026-01-22: `docs/` Ordner einzeln erneut durchgegangen (alle .md + .drawio); PDFs/Images weiterhin nur Inventar.
- 2026-01-22: GitHub Actions Workflows gelesen und Deploy-Pfade dokumentiert.
- 2026-01-22: Mini-PC (alter Build) auf Fork umgestellt: `~/control` remote auf `https://github.com/mitgefuehlt-lang/control.git`, lokale Aenderungen verworfen (`reset --hard`, `clean -fd`), `git pull` erfolgreich. `nixos-install.sh` gestartet; musste Repo-Eigentum auf root setzen (`/home/konrad/control`) wegen Nix-Fehler "repo not owned by current user". Build startete, danach SSH auf `192.168.178.106` verweigert, `192.168.178.100` aktuell Timeout (Status unklar).
- 2026-01-22 11:53: Reinstall/Rebuild Verlauf (Mini-PC, Fork): Rebuild zuerst auf altem Build versucht (SSH via `konrad@192.168.178.106`), `git pull` blockiert wegen lokaler Aenderungen; auf Wunsch verworfen (`git reset --hard`, `git clean -fd`). `nixos-install.sh` scheiterte mit "repository path not owned by current user" fuer `git+file:///home/konrad/control`; Loesung: Repo-Eigentum auf root gesetzt, Build erneut gestartet. Danach war SSH auf `192.168.178.106` zeitweise "refused" und `192.168.178.100` timeout, Status unklar. Spaeter auf `qitech` gewechselt (neuer Build aktiv), Fork per HTTPS neu geklont (`/home/qitech/control`). `nixos-rebuild switch --flake .#nixos` scheiterte wegen `builtins.currentSystem` in `flake.nix`; Fix: System fest auf `x86_64-linux` gesetzt, Commit/Push, anschliessend Rebuild erfolgreich. SSH ist jetzt aktiv; Zugriff via `qitech@192.168.178.106` bestaetigt.
- 2026-01-22 11:54: Smoke-Check Mini-PC: `qitech-control-server` aktiv, `sshd` aktiv, Ports 3001/22 offen (IPv4/IPv6). Repo-Remote auf Fork (`https://github.com/mitgefuehlt-lang/control.git`) bestaetigt, Hostname `nixos`.
- 2026-01-22 12:10: Rebuild nach Reboot: `dnsmasq` schlug fehl mit "unknown interface enp1s0". Tatsaechelliche Interfaces: `enp2s0` (Ethernet) und `wlo1` (WLAN). Fix vorbereitet: `nixos/os/configuration.nix` auf `enp2s0` umgestellt (statische IPv4, dnsmasq `interface`, Firewall trustedInterfaces).
- 2026-01-22 12:12: Fix verifiziert: `dnsmasq`, `sshd` und `qitech-control-server` alle `active` nach Rebuild; `dnsmasq` bindet an `enp2s0`.
- 2026-01-22 12:27: Boot-Fehler dokumentiert: Stage-1 meldet `stage 2 init script (...) not found` beim Booten neuer Generationen (z.B. 21). Das weist darauf hin, dass der Boot-Eintrag/Initrd auf einen Systempfad im Nix Store zeigt, der beim Boot nicht verfuegbar ist (inkonsistenter Boot-Eintrag oder veralteter Store-Path). QiTech-Doku (`docs/nixos/*`) beschreibt Build/Update-Flows, aber keinen Stage-1 Bootfehler oder Bootloader-Reparatur. Korrekturmassnahme: `nixos-rebuild switch --install-bootloader` ausgefuehrt, um systemd-boot und Eintraege zu erneuern; Reboot-Test steht noch aus.
- 2026-01-22 12:39: Root Cause gefunden: Rebuilds ohne `--impure` verwenden das im Repo liegende `nixos/os/ci-hardware-configuration.nix` (root auf `/dev/null`, `tmpfs`). Das erzeugt ein Initrd mit `initrd-fsinfo` fuer `/dev/null` und fuehrt beim Boot zu `stage 2 init script ... not found` (Root wird nicht gemountet). Ursache: Flake-Evaluierung ist "pure", daher wird `/etc/nixos/hardware-configuration.nix` nicht eingelesen, obwohl die Config per `builtins.pathExists` darauf verweist. Fix: Rebuild mit `--impure` oder `nixos-install.sh` (enthaelt `--impure`) ausfuehren, damit die echte Hardware-Config eingebunden wird. Doku weist `--impure` nicht explizit aus; daher als lokaler Hinweis in `ki-doku.md`.
- 2026-01-22 13:14: Reboot-Test nach `--impure` Rebuild erfolgreich: `sshd`, `qitech-control-server`, `dnsmasq` alle `active` nach Neustart.
- 2026-01-22 14:21: Zweiter Reboot-Test bestaetigt: `sshd`, `qitech-control-server`, `dnsmasq` erneut alle `active`.
- 2026-01-22 15:03: GitHub Actions Deploy via Tailscale vorbereitet: `services.tailscale.enable = true` und `tailscale0` als trustedInterface gesetzt (fuer CI-Deploy aus GitHub Cloud). Erfordert Auth-Login via `tailscale up` nach Rebuild.
- 2026-01-22 16:02: Fuer heute pausiert; Fortsetzung am Montag. Resume-Link dokumentiert: `codex resume 019be522-a6c5-7643-8696-30357813465a`.
- 2026-01-27 ~10:00 [Claude Opus 4.5]: Fortsetzung nach Neuaufsetzung Mini-PC. Ziel: GitHub Actions Deploy ueber Tailscale einrichten.
- 2026-01-27 ~10:05 [Claude Opus 4.5]: **Fehler** bei GitHub Actions Workflow `fast-deploy.yml`: `oauth authkeys require --advertise-tags`. Ursache: Workflow verwendete `authkey` Parameter mit OAuth Client Secret, aber OAuth erfordert zwingend `--advertise-tags`.
- 2026-01-27 ~10:10 [Claude Opus 4.5]: **Loesung**: Workflow auf korrekte OAuth-Syntax umgestellt. Aenderung in `.github/workflows/fast-deploy.yml`:
  - Alt: `authkey: ${{ secrets.TAILSCALE_AUTHKEY }}`
  - Neu: `oauth-client-id: ${{ secrets.TS_OAUTH_CLIENT_ID }}`, `oauth-secret: ${{ secrets.TS_OAUTH_SECRET }}`, `tags: tag:ci`
- 2026-01-27 ~10:10 [Claude Opus 4.5]: **Offene Schritte** fuer Benutzer:
  1. Tailscale ACL: `"tag:ci": ["autogroup:admin"]` unter `tagOwners` hinzufuegen
  2. OAuth Client erstellen (https://login.tailscale.com/admin/settings/oauth) mit Scope `devices:write` und Tag `tag:ci`
  3. GitHub Secrets anlegen: `TS_OAUTH_CLIENT_ID`, `TS_OAUTH_SECRET`
  4. Workflow-Aenderung committen und pushen
- 2026-01-27 [Claude Opus 4.5]: **Neue Regel etabliert**: Jeder Schritt, Fehler und Loesung wird mit Datum, Uhrzeit und KI-Modell in ki-doku.md dokumentiert.
- 2026-01-27 ~10:20 [Claude Opus 4.5]: Workflow-Aenderung committed und gepusht (Commit d513af5e).
- 2026-01-27 ~10:20 [Claude Opus 4.5]: **Fehler** bei Workflow-Run: `OAuth identity empty`. Ursache: GitHub Secrets `TS_OAUTH_CLIENT_ID` und `TS_OAUTH_SECRET` wurden noch nicht angelegt. Benutzer muss diese in GitHub unter Settings -> Secrets -> Actions erstellen.
- 2026-01-27 ~10:31 [Claude Opus 4.5]: **Fehler** bei Workflow-Run: `tailscale: failed to evaluate SSH policy`. Ursache: Tailscale SSH Policy in ACLs fehlte.
- 2026-01-27 ~11:47 [Claude Opus 4.5]: **Fehler** bei Workflow-Run: `Connection timed out`. Ursache: DEPLOY_HOST war auf lokale IP statt Tailscale IP gesetzt.
- 2026-01-27 ~16:00 [Claude Opus 4.5]: Tailscale auf Mini-PC war ausgeloggt nach Tag-Aenderung. Neu authentifiziert mit `sudo tailscale up --advertise-tags=tag:server --ssh --accept-routes`.
- 2026-01-27 16:09 [Claude Opus 4.5]: **ERFOLG** - GitHub Actions Deploy ueber Tailscale funktioniert! Workflow `fast-deploy.yml` erfolgreich durchgelaufen (Run ID: 21404628891, Dauer: 51s).
- 2026-01-27 16:09 [Claude Opus 4.5]: **Finale Konfiguration fuer Tailscale CI/CD:**
  - GitHub Secrets: `TS_OAUTH_CLIENT_ID`, `TS_OAUTH_SECRET`, `DEPLOY_HOST` (=100.120.73.16), `DEPLOY_USER` (=qitech), `DEPLOY_SSH_KEY`
  - Tailscale ACLs: `tag:ci` und `tag:server` in tagOwners; SSH-Regel src=tag:ci, dst=tag:server, users=[qitech,root]
  - Mini-PC: `tailscale up --advertise-tags=tag:server --ssh --accept-routes`
- 2026-01-27 ~17:00 [Claude Opus 4.5]: Dokumentation gelesen fuer das Anlegen einer neuen Maschine. Relevante Docs:
  - `docs/developer-docs/adding-a-machine.md` - Hauptanleitung (4 Dateien: mod.rs, new.rs, act.rs, api.rs)
  - `docs/devices.md` - Device-Implementierung (PDO, CoE, Identity)
  - `docs/identification.md` - Machine Identification (EEPROM Words: Vendor/Machine/Serial/Role)
  - `docs/architecture-overview.md` - Architektur (Electron -> Server -> control-core -> ethercat-hal)
  - MockMachine als einfachstes Beispiel unter `machines/src/mock/`
- 2026-01-27 ~17:30 [Claude Opus 4.5]: **SchneidemaschineV0 angelegt** nach QiTech-Struktur:
  - Hardware: EK1100, 2x EL1008, 1x EL2008, 5x EL2522
  - Machine ID: `SCHNEIDEMASCHINE_V0 = 0x0037`
  - Dateien erstellt:
    - `machines/src/schneidemaschine_v0/mod.rs` - Struct mit 16 DI, 8 DO, 10 PTO-Achsen
    - `machines/src/schneidemaschine_v0/new.rs` - Device-Zusammenbau mit Rollen 1-8
    - `machines/src/schneidemaschine_v0/act.rs` - Control Loop (30 Hz emit)
    - `machines/src/schneidemaschine_v0/api.rs` - Mutations (SetOutput, SetAxisSpeed, StopAllAxes) + Events
  - Registriert in `machines/src/lib.rs` und `machines/src/registry.rs`
  - Device-Rollen:
    - Role 1: EL1008 #1 (DI 1-8)
    - Role 2: EL1008 #2 (DI 9-16)
    - Role 3: EL2008 (DO 1-8)
    - Role 4-8: EL2522 #1-5 (Achsen 1-10)
- 2026-01-27 ~17:32 [Claude Opus 4.5]: **Kompilierung erfolgreich** - SchneidemaschineV0 Code kompiliert und deployed auf Mini-PC. Server laeuft (`qitech-control-server: active`). dnsmasq war beim Check noch nicht aktiv (Timing-Problem).
- 2026-01-27 ~17:36 [Claude Opus 4.5]: **Workflow-Fix** - Race Condition behoben: `sleep 5` vor systemctl-Checks eingefuegt, damit Services Zeit haben zu starten. Deploy jetzt erfolgreich (Run ID: 21405616278).
- 2026-01-27 ~18:00 [Claude Opus 4.5]: **Problem** - SchneidemaschineV0 nicht im UI Assign-Dropdown sichtbar. Ursache: Maschine war nur im Backend registriert, nicht im Frontend (`electron/src/machines/properties.ts`).
- 2026-01-27 ~18:05 [Claude Opus 4.5]: **Loesung** - Frontend-Registration hinzugefuegt:
  - `schneidemaschineV0` in `electron/src/machines/properties.ts` erstellt
  - Device-IDs aus Backend uebernommen:
    - EK1100: vendor=0x2, product=0x44c2c52, rev=0x120000
    - EL1008: vendor=0x2, product=0x03f03052, rev=0x120000
    - EL2008: vendor=0x2, product=0x07d83052, rev=0x110000/0x120000
    - EL2522: vendor=0x2, product=0x09da3052, rev=0x160000
  - 9 Rollen definiert (Role 0-8): Bus Coupler, 2x DI, 1x DO, 5x PTO
  - Zum `machineProperties` Array hinzugefuegt
- 2026-01-27 ~18:15 [Claude Opus 4.5]: **Problem** - Nach Deploy war SchneidemaschineV0 immer noch nicht im UI sichtbar.
- 2026-01-27 ~18:20 [Claude Opus 4.5]: **Ursache** - Nix hatte den Electron-Build gecacht. `nixos-rebuild switch` hat den gecachten Build verwendet statt neu zu bauen.
- 2026-01-27 ~18:25 [Claude Opus 4.5]: **Loesung** - `nixos-install.sh` Script ausgefuehrt um kompletten Rebuild zu erzwingen. Vorher `git safe.directory` fuer root konfiguriert. SchneidemaschineV0 jetzt in `/run/current-system/sw/share/qitech-control-electron/assets/index-*.js` vorhanden. Electron App muss neu gestartet werden.
- 2026-01-27 ~18:35 [Claude Opus 4.5]: **Bug gefunden** - Device Role Dropdown reagiert nicht auf Klicks. Ursache: In `electron/src/setup/DeviceEepromDialog.tsx` Zeile 436 fehlte `onValueChange={field.onChange}` beim Device Role Select. Der Machine Select (Zeile 366) hatte es korrekt, Device Role nicht.
- 2026-01-27 ~18:40 [Claude Opus 4.5]: **Bug behoben** - `onValueChange={field.onChange}` hinzugefuegt. Commit 3d4cab24. Deployed via `nixos-install.sh` auf Mini-PC.
- 2026-01-27 ~18:50 [Claude Opus 4.5]: **Hardware-Anpassung** - SchneidemaschineV0 auf aktuelle Hardware reduziert (Commit 9376918e):
  - Vorher: 2x EL1008 (16 DI), 1x EL2008 (8 DO), 5x EL2522 (10 Achsen)
  - Nachher: 1x EL1008 (8 DI), 1x EL2008 (8 DO), 1x EL2522 (2 Achsen)
  - Neue Rollen: 1=Digital Input, 2=Digital Output, 3=PTO
- 2026-01-27 ~19:00 [Claude Opus 4.5]: **ERFOLG** - SchneidemaschineV0 laeuft!
  - Serial: 21 (2. Prozessschritt, 1. Maschine)
  - Hardware: EK1100 + EL1008 + EL2008 + EL2522
  - Rollen: 0=Bus Coupler, 1=DI, 2=DO, 3=PTO
- 2026-01-28 ~18:00 [Claude Opus 4.5]: **Tailscale ACLs neu konfiguriert** nach Reset durch Benutzer:
  - ACLs komplett geloescht und neu eingetragen
  - tagOwners: `tag:ci` und `tag:server` fuer `autogroup:admin`
  - SSH-Regeln: `tag:ci` -> `tag:server` (users: qitech, root) + `autogroup:admin` -> `tag:server`
  - Mini-PC: `sudo tailscale up --advertise-tags=tag:server --ssh --accept-routes`
  - `sudo tailscale set --ssh` explizit ausgefuehrt
- 2026-01-28 ~18:10 [Claude Opus 4.5]: **ERFOLG** - GitHub Actions Deploy ueber Tailscale funktioniert wieder!
  - Workflow Run ID: 21447835275
  - Tailscale-Verbindung: Runner sieht Mini-PC im Status
  - Ping: 0% Paketverlust (Latenz initial hoch wegen NAT-Traversal)
  - SSH + git pull + nixos-rebuild: Erfolgreich

### Tailscale CI/CD Komplettanleitung (Stand 2026-01-28)

**Problem**: Tailscale ACLs waren komplett geloescht, GitHub Actions Deploy funktionierte nicht mehr.

**Loesung in 4 Schritten:**

#### Schritt 1: ACLs auf login.tailscale.com eintragen
```json
{
  "tagOwners": {
    "tag:ci": ["autogroup:admin"],
    "tag:server": ["autogroup:admin"]
  },
  "acls": [
    {
      "action": "accept",
      "src": ["*"],
      "dst": ["*:*"]
    }
  ],
  "ssh": [
    {
      "action": "accept",
      "src": ["tag:ci"],
      "dst": ["tag:server"],
      "users": ["qitech", "root"]
    },
    {
      "action": "accept",
      "src": ["autogroup:admin"],
      "dst": ["tag:server"],
      "users": ["autogroup:nonroot", "root"]
    }
  ]
}
```
**Wichtig**: `dst: ["*"]` funktioniert NICHT bei SSH-Regeln, muss explizit `tag:server` sein.

#### Schritt 2: Mini-PC mit Tailscale verbinden
```bash
sudo tailscale up --advertise-tags=tag:server --ssh --accept-routes
```

#### Schritt 3: Tailscale SSH explizit aktivieren
```bash
sudo tailscale set --ssh
```
**Wichtig**: Ohne diesen Befehl zeigt `tailscale status` eine Warnung:
> "Tailscale SSH enabled, but access controls don't allow anyone to access this device"
Nach `tailscale set --ssh` verschwindet die Warnung und SSH funktioniert.

#### Schritt 4: GitHub Secrets pruefen
- `TS_OAUTH_CLIENT_ID` - OAuth Client ID von login.tailscale.com
- `TS_OAUTH_SECRET` - OAuth Client Secret
- `DEPLOY_HOST` - Tailscale IP des Mini-PCs (z.B. `100.120.73.16`)
- `DEPLOY_USER` - `qitech`
- `DEPLOY_SSH_KEY` - SSH Private Key

**OAuth Client erstellen**: https://login.tailscale.com/admin/settings/oauth
- Scope: `devices:write`
- Tag: `tag:ci`

#### Debugging bei Problemen
Debug-Step im Workflow (temporaer hinzufuegen):
```yaml
- name: Debug Tailscale
  run: |
    tailscale status
    tailscale ip -4
    ping -c 3 ${{ secrets.DEPLOY_HOST }} || echo "Ping failed"
```

### SchneidemaschineV0 DI1 -> DO1 Logik (Stand 2026-01-28)

**Anforderung**: Eingang 1 (DI1) aktiv durch Taster -> Ausgang 1 (DO1) aktiv

**Implementierung bereits vorhanden** in `machines/src/schneidemaschine_v0/act.rs`:
```rust
// Simple IO logic: DI1 -> DO1 (press = output on)
let input_pressed = self.digital_inputs[0].get_value().unwrap_or(false);
if input_pressed != self.output_states[0] {
    self.set_output(0, input_pressed);
}
```

**Wie QiTech Beckhoff-Klemmen ansteuert (Architektur):**

1. **Control Loop** (`server/src/loop.rs`):
   - `copy_ethercat_inputs()` - Liest Inputs von EtherCAT-Devices in PDO-Objekte
   - `execute_machines()` - Ruft `machine.act(now)` fuer jede Maschine auf
   - `copy_ethercat_outputs()` - Schreibt Outputs von PDO-Objekten zu EtherCAT-Devices

2. **Device Layer** (`ethercat-hal/src/devices/`):
   - Jedes Device (EL1008, EL2008, etc.) hat TxPDO (Inputs) und/oder RxPDO (Outputs)
   - `is_used` Flag: Devices muessen mit `set_used(true)` markiert werden, sonst werden I/O-Daten nicht kopiert
   - `get_ethercat_device()` in `machines/src/lib.rs` ruft automatisch `set_used(true)` auf

3. **IO Layer** (`ethercat-hal/src/io/`):
   - `DigitalInput::get_value()` - Liest aus Device TxPDO
   - `DigitalOutput::set(bool)` - Schreibt in Device RxPDO

4. **Machine Layer** (`machines/src/schneidemaschine_v0/`):
   - `new.rs` - Erstellt Machine, holt Devices mit `get_ethercat_device`
   - `act.rs` - Control-Logik (wird jeden Zyklus aufgerufen, ~300us)
   - `mod.rs` - Struct Definition, Helper-Funktionen
   - `api.rs` - Events fuer UI, Mutations fuer API

**Wichtige Dateien fuer neue Maschinen:**
- `docs/developer-docs/adding-a-machine.md` - Hauptanleitung
- `docs/devices.md` - Device-Implementierung
- `machines/src/mock/` - Einfachstes Beispiel

**Server-Neustart bei EtherCAT-Problemen:**
```bash
sudo systemctl restart qitech-control-server
sudo journalctl -u qitech-control-server --no-pager -n 30
```
Erfolgreich wenn: "Group in Safe-OP state" und "Group in OP state" erscheinen.

- 2026-01-28 ~18:15 [Claude Opus 4.5]: **SchneidemaschineV0 DI1->DO1 verifiziert**
  - Code-Review: Logik war bereits korrekt implementiert in `act.rs`
  - Problem war EtherCAT-Timeout beim Server-Start (intermittierend)
  - Nach Server-Neustart: "Group in OP state" - EtherCAT funktioniert
  - Hardware: EK1100 (Role 0) + EL1008 (Role 1, DI) + EL2008 (Role 2, DO) + EL2522 (Role 3, PTO)
  - Test durch Benutzer: **ERFOLG** - DI1 -> DO1 funktioniert wie erwartet
- 2026-01-28 ~19:00 [Claude Opus 4.5]: **EL2522 PTO Stepper-Motor-Ansteuerung implementiert** (bisher schwerste Challenge, jetzt geloest!)
  - Hardware-Setup:
    - CL57T Stepper-Treiber von StepperOnline
    - 200 Pulse/Umdrehung (Treiber-Einstellung)
    - Kugelumlaufspindel: Durchmesser 16mm, Steigung 10mm
    - Berechnung: 20 Pulse/mm (200/10)
    - Anschluss an EL2522 Channel 2: PUL+/- an A2+/-, DIR+/- an B2+/-
  - Implementierte Aenderungen:
    1. **new.rs**: CoE-Konfiguration fuer EL2522 Channel 2
       - `PulseDirectionSpecification` Mode
       - `ramp_function_active: true` fuer sanfte Beschleunigung
       - `direct_input_mode: true` (Hz-Wert direkt als Frequenz)
       - `base_frequency_1: 5000` Hz (fuer Ramp-Berechnung)
       - `ramp_time_constant_rising/falling: 2500` ms
       - `frequency_factor: 100` (1:1 Verhaeltnis)
       - `watchdog_timer_deactive: true` (fuer Tests)
    2. **mod.rs**: Mechanik-Modul + Debug-Funktionen
       - `mechanics::PULSES_PER_MM = 20.0`
       - `mm_per_s_to_hz()`, `hz_to_mm_per_s()`, `pulses_to_mm()`
       - `set_axis_speed_mm_s()` - Geschwindigkeit in mm/s setzen
       - `get_debug_pto()` - Vollstaendige EtherCAT-Status-Abfrage
       - `emit_debug_pto()`, `log_debug_all()` - Debug-Ausgabe
    3. **api.rs**: Neue Events und Mutations
       - `DebugPtoEvent` mit allen EtherCAT-Statusinformationen
       - `SetAxisSpeedMmS { index, speed_mm_s }` Mutation
       - `DebugPto { index }` und `DebugLogAll` Mutations
    4. **act.rs**: Periodische Debug-Ausgabe wenn Achse laeuft
       - Log alle 1s: Frequenz, Position, Ramp-Status, Fehler
    5. **pulse_train_output.rs**: Neue oeffentliche Methoden
       - `get_input()`, `get_output()`, `set_output()`
  - Server-Log nach Deploy:
    `[SchneidemaschineV0] EL2522 configured: Channel 2 = PulseDirection mode, base_freq=5000Hz, ramp=2500ms`
  - Geschwindigkeits-Umrechnung:
    - 50 mm/s = 1000 Hz
    - 230 mm/s (Max) = 4600 Hz
  - **Status**: Deploy erfolgreich, EtherCAT in OP-State, Hardware-Test steht aus

### EL2522 PTO Stepper-Motor-Ansteuerung (Stand 2026-01-28)

**Hardware-Konfiguration:**
- Motor-Treiber: CL57T (StepperOnline)
- Pulse/Umdrehung: 200 (Treiber-DIP-Schalter)
- Kugelumlaufspindel: Lead 10mm
- Pulses/mm: 20 (200/10)
- Max. Geschwindigkeit: 230 mm/s = 4600 Hz
- Max. Beschleunigung: 500 mm/s² (100 mm/s² verwendet)

**Verkabelung EL2522 Channel 2:**
```
EL2522          CL57T
A2+ (Pin 3) --> PUL+
A2- (Pin 4) --> PUL-
B2+ (Pin 5) --> DIR+
B2- (Pin 6) --> DIR-
```

**CoE-Konfiguration (new.rs):**
```rust
EL2522ChannelConfiguration {
    operating_mode: EL2522OperatingMode::PulseDirectionSpecification,
    ramp_function_active: true,
    direct_input_mode: true,
    base_frequency_1: 5000,  // Max Hz fuer Ramp
    ramp_time_constant_rising: 2500,   // ms von 0 auf base_freq
    ramp_time_constant_falling: 2500,
    frequency_factor: 100,   // 1:1 (100 = 100%)
    watchdog_timer_deactive: true,
    ..Default::default()
}
```

**API-Verwendung:**
```json
// Achse 2 auf 50 mm/s setzen (= 1000 Hz)
{ "action": "SetAxisSpeedMmS", "value": { "index": 1, "speed_mm_s": 50.0 } }

// Achse stoppen
{ "action": "SetAxisSpeed", "value": { "index": 1, "speed": 0 } }

// Alle Achsen stoppen
{ "action": "StopAllAxes" }

// Debug-Info fuer Channel 2 abrufen
{ "action": "DebugPto", "value": { "index": 1 } }

// Alle Debug-Infos in Server-Konsole loggen
{ "action": "DebugLogAll" }
```

**Debug-Event Felder (DebugPtoEvent):**
- `channel` - Kanal-Nummer (0 oder 1)
- `frequency_setpoint_hz` - Gesendete Frequenz
- `frequency_setpoint_mm_s` - Umgerechnet in mm/s
- `actual_position_pulses` - Aktuelle Position (Zaehler)
- `actual_position_mm` - Position in mm
- `ramp_active` - Rampe gerade aktiv
- `error` - Fehler-Flag vom Device
- `sync_error` - Sync-Fehler
- `counter_overflow/underflow` - Zaehler-Ueberlauf

**Troubleshooting:**
1. Motor dreht nicht:
   - Server-Log pruefen: Sollte "EL2522 configured" zeigen
   - `DebugLogAll` aufrufen und `error`-Flag pruefen
   - Verkabelung pruefen (PUL/DIR richtig angeschlossen?)
2. Motor dreht falsch herum:
   - DIR+/DIR- tauschen oder negative Frequenz senden
3. Motor ruckelt:
   - `ramp_function_active` pruefen
   - `ramp_time_constant` erhoehen

- 2026-01-28 ~17:45 [Claude Opus 4.5]: **Motor-Steuerung via Taster implementiert**
  - Anforderung: DI1 (Taster) soll Motor starten/stoppen statt DO1 zu schalten
  - Aenderung in `act.rs`: DI1 steuert jetzt Achse 2 (Channel 2)
  - Logik: Taster gedrueckt = 1000 Hz (50 mm/s), losgelassen = 0 Hz
  - Commit: fccbdcd4 "SchneidemaschineV0: DI1 controls motor instead of DO1"
  - **ERFOLG** - Hardware-Test bestanden, Motor laeuft bei Tastendruck

### Motor-Steuerung via Taster (Stand 2026-01-28)

**Funktionsweise:**
- **Taster druecken (DI1 = HIGH)** → Motor laeuft mit 50 mm/s (1000 Hz)
- **Taster loslassen (DI1 = LOW)** → Motor stoppt (0 Hz)

**Code in `act.rs`:**
```rust
// DI1 controls motor on Channel 2: press = run at 50 mm/s, release = stop
let input_pressed = self.digital_inputs[0].get_value().unwrap_or(false);
let target_speed = if input_pressed { 1000 } else { 0 }; // 1000 Hz = 50 mm/s
if self.axis_speeds[1] != target_speed {
    self.set_axis_speed(1, target_speed);
    tracing::info!(
        "[SchneidemaschineV0] DI1={} -> Motor speed set to {} Hz ({} mm/s)",
        input_pressed,
        target_speed,
        target_speed as f32 / 20.0
    );
}
```

**Live-Monitoring:**
```bash
ssh qitech@192.168.178.106 "sudo journalctl -u qitech-control-server -f"
```

**Erwartete Log-Ausgaben:**
```
# Taster gedrueckt:
[SchneidemaschineV0] DI1=true -> Motor speed set to 1000 Hz (50 mm/s)

# Taster losgelassen:
[SchneidemaschineV0] DI1=false -> Motor speed set to 0 Hz (0 mm/s)

# Waehrend Motor laeuft (alle 1s):
[PTO2] freq=1000Hz pos=12345p (617.2mm) ramp=false err=false
```

**Geschwindigkeit aendern:**
In `act.rs` Zeile mit `target_speed` anpassen:
- 500 Hz = 25 mm/s
- 1000 Hz = 50 mm/s (aktuell)
- 2000 Hz = 100 mm/s
- 4600 Hz = 230 mm/s (Maximum)

Formel: `Hz = mm/s * 20` (weil 20 Pulse/mm)

### Zusammenfassung Session 2026-01-28

**Erledigte Aufgaben:**
1. ✅ Tailscale ACLs nach Reset neu konfiguriert
2. ✅ GitHub Actions CI/CD Deploy wiederhergestellt
3. ✅ EL2522 PTO Stepper-Motor-Ansteuerung implementiert (CoE-Konfiguration)
4. ✅ Debug-API fuer PTO-Status hinzugefuegt
5. ✅ Motor-Steuerung via Taster (DI1) implementiert
6. ✅ Hardware-Test erfolgreich - Motor laeuft bei Tastendruck

**Technische Meilensteine:**
- Erste erfolgreiche Ansteuerung eines Stepper-Motors via EL2522 in diesem Projekt
- Vollstaendige CoE-Konfiguration fuer Pulse+Direction Mode
- Rampen-Funktion fuer sanftes Anfahren/Bremsen aktiviert
- Umfassende Debug-Moeglichkeiten implementiert

**Gelernte Lektionen:**
- EL2522 benoetigt CoE-Konfiguration VOR dem Wechsel in OP-State
- `direct_input_mode: true` ermoeglicht direkte Hz-Werte statt Prozentwerte
- Tailscale SSH erfordert explizites `tailscale set --ssh` nach Neuverbindung
- `dst: ["*"]` funktioniert NICHT in Tailscale SSH-Regeln

**Naechste moegliche Schritte:**
- Geschwindigkeit ueber UI einstellbar machen
- Positionierung implementieren (Zielposition anfahren)
- Endschalter/Referenzfahrt hinzufuegen
- Zweite Achse (Channel 1) aktivieren

---

## Session 2026-01-30: SchneidemaschineV0 Frontend UI

### Kontext
- Arbeiten aus dem Homeoffice via Parsec (Zugriff auf Tower)
- Mini-PC (192.168.178.106) laeuft mit Backend und Hardware
- Ziel: UI fuer SchneidemaschineV0 erstellen, beginnend mit einem einfachen Taster-Button

### Erreichte Meilensteine

#### 1. Mini-PC Status geprueft
- Mini-PC online auf 192.168.178.106
- `qitech-control-server` aktiv (Uptime: 7 Minuten nach Boot)
- SchneidemaschineV0 geladen (Serial: 21)

#### 2. Frontend-Analyse durchgefuehrt
- Komplette UI-Architektur analysiert (React 19, Tailwind CSS 4, Zustand, SocketIO)
- Bestehende Maschinen-Patterns untersucht (Mock1, Extruder2, Laser1)
- Backend-API der SchneidemaschineV0 mit Frontend-Anforderungen abgeglichen

#### 3. SchneidemaschineV0 Frontend erstellt
Neue Dateien in `electron/src/machines/schneidemaschine/schneidemaschine_v0/`:

| Datei | Zweck |
|-------|-------|
| `schneidemaschineV0Namespace.ts` | Zod-Schemas, Zustand Store, WebSocket Event Handler |
| `useSchneidemaschineV0.ts` | React Hook mit `setOutput()` und `toggleOutput()` |
| `SchneidemaschineV0Page.tsx` | Navigation/Topbar mit Control-Tab |
| `SchneidemaschineV0ControlPage.tsx` | UI mit "Taster 1" Button fuer DO0 |

Routes registriert in `electron/src/routes/routes.tsx`.

#### 4. Remote-Zugriff eingerichtet
Problem: UI auf Tower, Backend auf Mini-PC - wie verbinden?

**Loesung: SSH-Tunnel**
```bash
ssh -N -L 3001:localhost:3001 qitech@192.168.178.106
```
- Tower localhost:3001 wird auf Mini-PC localhost:3001 weitergeleitet
- NixOS Firewall blockiert Port 3001 extern, aber SSH (Port 22) ist offen
- Tunnel laeuft im Hintergrund, UI verbindet sich ueber localhost

### Fehler und Loesungen

| Fehler | Ursache | Loesung |
|--------|---------|---------|
| Mutation-Format falsch | Backend nutzt `#[serde(tag="action", content="value")]` | JSON geaendert: `{ action: "SetOutput", value: { index, on } }` |
| `is_default_state` nicht gefunden | Backend sendet dieses Feld nicht | Aus Zod-Schema entfernt, defaultState-Logik angepasst |
| `startsWith` undefined | `activeLink` Property fehlte in Topbar items | `activeLink: "control"` hinzugefuegt |
| Port 3001 nicht erreichbar | NixOS Firewall blockiert Port | SSH-Tunnel statt direkter Verbindung |
| `npm run dev` nicht gefunden | Falsches Script | `npm start` verwendet (Vite) |
| Vite nicht gefunden | Dependencies fehlten | `npm install` ausgefuehrt |

### Technische Details

**Mutation-Format (korrekt):**
```typescript
// Frontend sendet:
{
  action: "SetOutput",
  value: { index: 0, on: true }
}

// Backend (Rust) erwartet:
#[serde(tag = "action", content = "value")]
enum Mutation {
    SetOutput { index: usize, on: bool },
    // ...
}
```

**StateEvent-Schema (angepasst):**
```typescript
// Backend sendet KEIN is_default_state fuer diese Maschine
export const stateEventDataSchema = z.object({
  output_states: z.tuple([z.boolean(), ...]),  // 8x
  axis_speeds: z.tuple([z.number(), z.number()]),
});
```

**SSH-Tunnel Architektur:**
```
[Homeoffice]          [Tower (Windows)]              [Mini-PC (NixOS)]
     |                       |                              |
  Parsec ─────────────► Electron UI                         |
                             |                              |
                        localhost:3001                      |
                             |                              |
                        SSH-Tunnel ──────────────────► localhost:3001
                                                            |
                                                      Backend Server
                                                            |
                                                      EtherCAT Hardware
```

### Aktueller Stand
- UI laeuft auf Tower (Vite dev server + Electron)
- SSH-Tunnel verbindet zu Mini-PC Backend
- SchneidemaschineV0 ist in der Maschinen-Liste sichtbar
- "Taster 1" Button fuer DO0 implementiert und **erfolgreich getestet**

### Test-Ergebnis (2026-01-30 ~11:35)
- SchneidemaschineV0 Control Page laedt korrekt
- Button "Taster 1" wird angezeigt
- Kommunikation Frontend <-> Backend ueber SSH-Tunnel funktioniert

### Weitere Fixes (2026-01-30 ~11:50)

**Problem:** "Unhandled Event - Namespace can't handle" Fehlermeldungen im UI

**Ursache:**
- Backend sendet `DebugPtoEvent` das nicht behandelt wurde
- `mainNamespace` wirft Fehler bei unbekannten Events

**Loesung:**
1. `schneidemaschineV0Namespace.ts`: DebugPtoEvent ignorieren, unbekannte Events nur loggen
2. `mainNamespace.ts`: Unbekannte Events ignorieren statt Fehler werfen

```typescript
// Statt:
handleUnhandledEventError(eventName);

// Jetzt:
console.warn(`Unknown event "${eventName}" ignored`);
```

### Hardware-Test (2026-01-30 ~11:55)

**Test:** Button "Taster 1" mehrfach gedrueckt (AN/AUS)

**Server-Logs bestaetigen Empfang:**
```
10:39:43 Mutating machine=1/55/21 {"action": "SetOutput", "value": {"index": 0, "on": true}}
10:39:45 Mutating machine=1/55/21 {"action": "SetOutput", "value": {"index": 0, "on": false}}
10:39:46 Mutating machine=1/55/21 {"action": "SetOutput", "value": {"index": 0, "on": true}}
10:39:47 Mutating machine=1/55/21 {"action": "SetOutput", "value": {"index": 0, "on": false}}
```

**Ergebnis:**
- Frontend -> Backend Kommunikation: **FUNKTIONIERT**
- Mutation-Format korrekt (action/value)
- Backend empfaengt und verarbeitet Befehle
- EL2008 DO0 sollte schalten (LED leuchtet bei "AN") - nicht live verifizierbar (Homeoffice)

### Update-Quelle geaendert (2026-01-30 ~12:10)

**Aenderung:** Default GitHub-Quelle fuer Software-Updates auf eigenes Repo umgestellt.

**Datei:** `electron/src/setup/GithubSourceDialog.tsx`

**Vorher:**
```typescript
export const defaultGithubSource: GithubSource = {
  githubRepoOwner: "qitechgmbh",
  githubRepoName: "control",
  githubToken: "github_pat_...",  // QiTech PAT
};
```

**Nachher:**
```typescript
export const defaultGithubSource: GithubSource = {
  githubRepoOwner: "mitgefuehlt-lang",
  githubRepoName: "control",
  githubToken: undefined,  // Kein Token noetig (public repo)
};
```

**Hinweise:**
- Das Repo `mitgefuehlt-lang/control` ist ein Fork von `qitechgmbh/control`
- Da das Repo public ist, wird kein GitHub-Token benoetigt
- Falls localStorage noch den alten Wert cached hat:
  - Option 1: Im UI auf "Edit Source" klicken und manuell aendern
  - Option 2: DevTools -> Application -> Local Storage -> `github-source-storage` loeschen

### Naechste Schritte
- [x] Achsen-Steuerung UI (axis_speeds) - erledigt 2026-02-02
- [x] Live-Werte Anzeige (input_states, axis_positions) - erledigt 2026-02-02
- [ ] Weitere Outputs hinzufuegen (DO1-DO7)
- [ ] Vor-Ort-Test: LED am EL2008 pruefen

---

## Session 2026-02-02: Motor Control UI mit Beschleunigung und Position

### Uebersicht
Komplette Motor-Steuerung fuer SchneidemaschineV0 implementiert:
- Geschwindigkeitsregelung mit Software-Ramping
- Dynamische Beschleunigungseinstellung
- Positionsfahrt mit Auto-Stop

### 1. Motor Control UI Grundgeruest

**Dateien:**
- `electron/src/machines/schneidemaschine/schneidemaschine_v0/SchneidemaschineV0MotorsPage.tsx` (neu)
- `electron/src/machines/schneidemaschine/schneidemaschine_v0/useSchneidemaschineV0.ts`
- `electron/src/routes/routes.tsx`

**Implementierung:**
- Neue "Motors" Seite neben "Control" Seite
- TouchButton fuer START (gruen) und STOP (rot)
- EditValue fuer Geschwindigkeitseingabe

**Problem 1: Motor reagierte nicht auf UI-Befehle**
- Ursache: `act.rs` hatte DI1-Override der bei jedem Zyklus die Geschwindigkeit zuruecksetzte
- Loesung: DI1-Override entfernt, UI hat volle Kontrolle

**Problem 2: Falscher Achsen-Index**
- Ursache: UI nutzte Index 0, Motor ist aber an Channel 2 (Index 1)
- Loesung: `MOTOR_AXIS_INDEX = 1` konstante eingefuehrt

### 2. Software-Ramping fuer Beschleunigung

**Problem:** EL2522 Hardware-Rampe wird nur bei Initialisierung via CoE konfiguriert, nicht zur Laufzeit aenderbar.

**Loesung:** Software-Ramping implementiert

**Backend-Aenderungen (machines/src/schneidemaschine_v0/):**

```rust
// mod.rs - Neue Felder
pub axis_speeds: [i32; 2],           // Aktuelle Geschwindigkeit (Hz)
pub axis_target_speeds: [i32; 2],    // Ziel-Geschwindigkeit (Hz)
pub axis_accelerations: [f32; 2],    // Beschleunigung (mm/s²)
pub last_ramp_update: Instant,

// Software-Ramp Funktion
pub fn update_software_ramp(&mut self, dt_secs: f32) -> bool {
    // Berechnet delta_hz basierend auf Beschleunigung
    // Bewegt axis_speeds Richtung axis_target_speeds
    // Gibt true zurueck wenn sich Geschwindigkeit geaendert hat
}
```

```rust
// act.rs - Ramp-Update im Loop
let dt = now.duration_since(self.last_ramp_update).as_secs_f32();
if dt > 0.001 {
    let speed_changed = self.update_software_ramp(dt);
    self.last_ramp_update = now;
    if speed_changed {
        self.emit_state();  // UI bekommt Live-Updates
    }
}
```

```rust
// new.rs - Hardware-Rampe deaktiviert
channel2_configuration: EL2522ChannelConfiguration {
    ramp_function_active: false,  // Software-Ramping stattdessen
    // ...
}
```

**Frontend-Aenderungen:**
- `SetAxisAcceleration` Mutation hinzugefuegt
- EditValue fuer Beschleunigung (1-500 mm/s², Step 10)
- Beschleunigung wird vor START angewendet

### 3. Positionsfahrt

**Ziel:** Motor faehrt auf eingestellte Position und stoppt automatisch

**Backend-Aenderungen:**

```rust
// mod.rs - Neue Felder
pub axis_target_positions: [u32; 2],  // Ziel-Position (Pulse)
pub axis_position_mode: [bool; 2],    // Position-Modus aktiv?

// Neue Funktion
pub fn move_to_position_mm(&mut self, index: usize, position_mm: f32, speed_mm_s: f32) {
    // Berechnet Richtung basierend auf aktueller vs Ziel-Position
    // Setzt target_counter_value in Hardware
    // Aktiviert position_mode
}
```

```rust
// update_software_ramp - Position-Check
if self.axis_position_mode[i] {
    let current_pos = self.axes[i].get_position();
    let target_pos = self.axis_target_positions[i];
    // Prueft ob Ziel erreicht -> stoppt automatisch
}
```

```rust
// new.rs - Travel Distance Control aktiviert
channel2_configuration: EL2522ChannelConfiguration {
    travel_distance_control: true,  // Auto-Stop bei Ziel-Position
    // ...
}
```

**Frontend:**
- `MoveToPosition` Mutation
- EditValue fuer Ziel-Position (0-10000 mm)
- "ZUR POSITION" Button (blau)

### 4. UI Layout

**Finale Struktur der Motors-Seite:**
```
+------------------------------------------+
|           Achse 1 - Motor                |
+------------------------------------------+
| [Geschwindigkeit] [Beschleunigung] [Position] |
|    50 mm/s          100 mm/s²        0 mm    |
+------------------------------------------+
| [START]      [ZUR POSITION]      [STOP]  |
| (gruen)         (blau)           (rot)   |
+------------------------------------------+
| Aktuelle Geschw: 0 mm/s | Position: 0 mm |
+------------------------------------------+
| Motor laeuft (pulsierend, wenn aktiv)    |
+------------------------------------------+
```

### 5. Fehler und Loesungen

| Fehler | Ursache | Loesung |
|--------|---------|---------|
| Motor reagiert nicht | DI1-Override in act.rs | Override entfernt |
| Falscher Motor angesteuert | Index 0 statt 1 | MOTOR_AXIS_INDEX = 1 |
| Import-Fehler `@/lib/roundTo` | Falscher Pfad | Geaendert zu `@/lib/decimal` |
| START/STOP disabled falsch | Pruefung auf currentSpeed statt targetSpeed | Prueft jetzt serverTargetSpeedHz |
| Geschwindigkeit zeigt 0 waehrend Fahrt | State nur bei Mutation emittiert | emit_state() bei Ramp-Aenderung |
| Rust Syntax Error | `as i32.max(1)` | `(... as i32).max(1)` |
| Deployment dauert 28+ min | Unbekannte Ursache bei GitHub Actions | Abgebrochen, neuer Versuch ~3-7 min |

### 6. Deployment

**GitHub Actions Workflow:** `fast-deploy.yml`
- Trigger: `gh workflow run fast-deploy.yml --ref master`
- Status: `gh run list --workflow=fast-deploy.yml --limit=1`
- Watch: `gh run watch <run-id> --exit-status`
- Dauer: Normal 2-7 min, selten bis 28 min (dann abbrechen)

**Direkte Verbindung fehlgeschlagen:**
- SSH zu 192.168.178.106 mit allen Keys verweigert
- Tailscale-Verbindung nur via GitHub Actions funktioniert

### 7. Technische Details

**Mechanik-Konstanten:**
```rust
pub const PULSES_PER_REV: u32 = 200;  // CL57T Stepper
pub const LEAD_MM: f32 = 10.0;         // Kugelgewindespindel
pub const PULSES_PER_MM: f32 = 20.0;   // 200/10
```

**Geschwindigkeits-Grenzen:**
- Max: 230 mm/s (= 4600 Hz)
- EL2522 base_frequency_1: 5000 Hz

**Beschleunigungs-Grenzen:**
- Min: 1 mm/s²
- Max: 500 mm/s²
- Default: 100 mm/s²

### 8. Commits

1. `83322fa6` - Add acceleration control for motor UI with software ramping
2. `103ffecc` - Fix motor UI: button logic and layout
3. `8cf13864` - Fix: emit state during software ramp so UI shows current speed
4. `178af19f` - Fix syntax: add parentheses around cast before method call
5. `9f942972` - Add position control and fix current speed display

### 9. Offene Punkte

- [ ] Hardware-Test: Positionsfahrt verifizieren
- [ ] Hardware-Test: Beschleunigung fuehlt sich korrekt an?
- [ ] Position-Reset Button (Nullpunkt setzen)
- [ ] Negative Positionen / Richtungsumkehr
- [ ] Endschalter-Integration

---

## Session 2026-02-03: BBM Automatik V2 Hardware-Anpassung & EtherCAT Troubleshooting

### Uebersicht
- BBM Automatik V2 Maschine fuer 1x EL2522 angepasst (statt 2x)
- Electron UI Preload-Pfad gefixt
- EtherCAT Timeout-Problem dokumentiert und geloest

### 1. BBM Automatik V2 - Zweite EL2522 optional gemacht

**Problem:** Maschine startete nicht - "role 4 not found" Fehler
- BBM Automatik V2 war fuer 2x EL2522 (4 Achsen) implementiert
- Aktuell nur 1x EL2522 angeschlossen (2 Achsen: MT + Schieber)

**Loesung:** Zweite EL2522 auskommentiert, Placeholder fuer Achsen 2+3

**Geaenderte Dateien:**
- `machines/src/bbm_automatik_v2/mod.rs` - Kommentar geaendert (1x statt 2x EL2522)
- `machines/src/bbm_automatik_v2/new.rs` - PTO_2 Block auskommentiert

**Code-Aenderung (new.rs):**
```rust
// Vorher: Versuchte roles::PTO_2 zu laden (Role 4)
let (el2522_2, subdevice_2) = get_ethercat_device::<EL2522>(..., roles::PTO_2, ...)?;

// Nachher: Auskommentiert, Placeholder verwendet
tracing::info!("[BbmAutomatikV2] Using only EL2522 #1 (2 axes). Second EL2522 not connected.");
let axes = [
    PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO1), // MT
    PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO2), // Schieber
    PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO1), // Druecker (placeholder)
    PulseTrainOutput::new(el2522_1.clone(), EL2522Port::PTO2), // Buerste (placeholder)
];
```

**Hinweis:** Achsen 2+3 zeigen auf dieselbe Hardware wie 0+1 - nur als Placeholder bis zweite EL2522 da ist.

### 2. Electron UI Preload-Pfad Fix

**Problem:** Electron crashte mit "preload script must have absolute path"

**Ursache:** `path.join(DIR, "preload.js")` erzeugte relativen Pfad

**Loesung:** `path.resolve()` statt `path.join()`

**Datei:** `electron/src/main.ts`
```typescript
// Vorher:
const preload = path.join(DIR, "preload.js");

// Nachher:
const preload = path.resolve(DIR, "preload.js");
```

### 3. EtherCAT Timeout Troubleshooting

**Problem:** Nach Deployment kein EtherCAT - UI zeigt keine Klemmen

**Server-Log zeigte:**
```
[server::main] Failed to initialize EtherCAT network
[server::ethercat::setup::setup_loop] Failed to initialize subdevices: Timeout
```

**Ursache:** Intermittierender Timeout bei EtherCAT-Initialisierung nach Service-Restart

**Loesung:** Service nochmal neustarten
```bash
ssh qitech@nixos "sudo systemctl restart qitech-control-server"
```

**Nach Neustart:**
```
Initialized 4 subdevices
[BbmAutomatikV2] EL2522 #1 configured: Ch1=MT, Ch2=Schieber
[BbmAutomatikV2] Using only EL2522 #1 (2 axes). Second EL2522 not connected.
Group in Safe-OP state
Group in OP state
Successfully initialized EtherCAT devices
```

### 4. Bekanntes Problem: EtherCAT Timeout nach Deployment

**Symptom:**
- Deployment via `fast-deploy.yml` erfolgreich
- Server startet, aber EtherCAT Initialisierung schlaegt mit "Timeout" fehl
- UI zeigt keine EtherCAT-Geraete

**Haeufigkeit:** Passiert oefters nach nixos-rebuild/Service-Restart

**Standard-Loesung:**
```bash
# Verbindung via SSH
ssh qitech@nixos

# Service neustarten
sudo systemctl restart qitech-control-server

# Logs pruefen (sollte "Group in OP state" zeigen)
sudo journalctl -u qitech-control-server --no-pager -n 30
```

**Erfolgskriterien im Log:**
- "Initialized X subdevices"
- "Group in Safe-OP state"
- "Group in OP state"
- "Successfully initialized EtherCAT devices"

**Falls weiterhin fehlschlaegt:**
1. Mehrfach Service neustarten (bis zu 3x)
2. Hardware-Power-Cycle der EtherCAT-Klemmen
3. Mini-PC komplett neustarten

### 5. SSH-Zugang

**Direkter SSH funktioniert jetzt:**
```bash
ssh qitech@nixos
```

**Hinweis:** Hostname "nixos" wird via Tailscale/lokales Netzwerk aufgeloest.

### 6. Commits

1. `b7be5ba7` - Make second EL2522 optional and fix Electron preload path
   - `machines/src/bbm_automatik_v2/mod.rs` - Kommentar-Update
   - `machines/src/bbm_automatik_v2/new.rs` - PTO_2 auskommentiert
   - `electron/src/main.ts` - Preload path.resolve() fix

### 7. Aktuelle Hardware-Konfiguration BBM Automatik V2

**EtherCAT-Klemmen (4 Subdevices):**
| Role | Subdevice | Klemme | Funktion |
|------|-----------|--------|----------|
| 0 | 0 | EK1100 | Bus-Koppler |
| 1 | 1 | EL1008 | 8x Digital Input (Referenzschalter, Tuersensoren) |
| 2 | 2 | EL2008 | 8x Digital Output (Ruettelmotor, Ampel) |
| 3 | 3 | EL2522 | 2x PTO (MT, Schieber) |

**Geplante Erweiterung (TODO):**
| Role | Klemme | Funktion |
|------|--------|----------|
| 4 | EL2522 #2 | 2x PTO (Druecker, Buerste) |

### 8. Naechste Schritte

- [ ] Zweite EL2522 anschliessen und Role 4 aktivieren
- [ ] Homing-Sequenz implementieren (Referenzfahrt)
- [ ] Automatik-Zyklus (State Machine)
- [ ] Sicherheitslogik (Tuersensoren, Endschalter)

---

## Session 2026-02-05: BBM Automatik V2 - Homing & Arduino-Analyse

### Arduino-Code Analyse (BBMx22_Automatik_Code.ino v3.2)

Der Arduino-Code fuer die gleiche Maschine wurde analysiert, um nuetzliche Parameter und Patterns zu extrahieren.
**Wichtig:** Wir bauen alles selbst, aber die Parameter dienen als Referenz.

#### Parameter-Vergleich

| Parameter | Arduino | Unsere Implementierung | Anmerkung |
|-----------|---------|------------------------|-----------|
| Steps/mm | 20 | 20 | Identisch ✓ |
| Homing Speed | 10 mm/s | 15 mm/s | Unsere etwas schneller |
| Max Speed | 200 mm/s | 250 mm/s | Mehr Reserve |
| Acceleration | 500 mm/s² | 100 mm/s² (default) | Konfigurierbar |
| Homing Backoff | 2 mm | 2 mm | Identisch ✓ |

#### Position-Parameter aus Arduino

**Achse 1 (Transporter):**
- Start: 5 mm, Auto-Run: 34.5 mm, Soft Limit: 230 mm
- Advance pro Zyklus: 10 mm

**Achse 2 (Schieber):**
- Start: 7 mm, Target: 51 mm, Soft Limit: 53 mm
- Wobble: 1.5 mm, 1 Zyklus

**Achse 3 (Druecker):**
- Start: 60 mm, Target: 105 mm, Soft Limit: 107 mm

#### Auto-Sequenz (19 Zyklen pro Magazin)

```
1. Achse 1 → Run Position (34.5mm)
2. Loop (19x):
   a. Wobble Achse 2 (±1.5mm)
   b. Achse 2 → Target (51mm)
   c. Achse 3 → Target (105mm)
   d. Achse 3 → Start (60mm)
   e. Achse 2 → Start + Achse 1 -10mm (parallel)
3. Achse 1 → Load Position (5mm)
```

#### Safety Features im Arduino-Code

| Feature | Implementierung | Status bei uns |
|---------|-----------------|----------------|
| Tuersensoren (2x NC) | Emergency Stop wenn offen | Inputs vorhanden, Logik TODO |
| Soft Limits | Max-Positionen pro Achse | TODO |
| Driver Alarms | 3 Alarm-Pins (Active LOW) | Nicht relevant (EtherCAT) |
| Watchdog | 2s Timeout, Auto-Reset | EtherCAT hat eigene Watchdogs |
| Homing Timeout | 20s (Achse1: 60s) | TODO |
| Auto Move Timeout | 30s pro Bewegung | TODO |
| Input Debounce | 10ms | Nicht noetig (EtherCAT ist digital) |

#### Signal Tower States (Arduino)

```
STARTUP → WAIT_HOMING → HOMING → AUTO_RUNNING ↔ LOAD_WAIT
                ↓           ↓          ↓
              ERROR ←←←←←←←←←←←←←←←←←←←
```

#### Homing-Sequenz (Arduino-Pattern)

**Reihenfolge:** Achse 3 zuerst, dann Achse 2 + Achse 1 parallel

**3-Phasen-Homing pro Achse:**
1. Negative Richtung fahren bis Sensor schaltet
2. 2mm zurueckfahren vom Sensor
3. Position auf 0 setzen
4. (Optional) Zur Startposition fahren

### Homing-Implementierung (Rust/EtherCAT)

Homing wurde in `machines/src/bbm_automatik_v2/` implementiert:

**mod.rs - HomingPhase Enum:**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HomingPhase {
    Idle,
    SearchingSensor,   // Phase 1: Fahr zum Sensor
    Retracting,        // Phase 2: 2mm zurueck
    SettingZero,       // Phase 3: Position nullen
}
```

**Homing-Konstanten:**
```rust
pub mod homing {
    pub const HOMING_SPEED_MM_S: f32 = 15.0;
    pub const RETRACT_DISTANCE_MM: f32 = 2.0;
}
```

**Input-Mapping (Referenzschalter):**
```rust
pub mod inputs {
    pub const REF_MT: usize = 1;        // DI1: Transporter
    pub const REF_SCHIEBER: usize = 2;  // DI2: Schieber
    pub const REF_DRUECKER: usize = 3;  // DI3: Druecker
    pub const TUER_1: usize = 4;        // DI4
    pub const TUER_2: usize = 5;        // DI5
}
```

**API-Erweiterungen (api.rs):**
- `StartHoming { index: usize }` - Startet Homing fuer eine Achse
- `CancelHoming { index: usize }` - Bricht Homing ab
- `axis_homing_active: [bool; 4]` in StateEvent

**PulseTrainOutput-Erweiterungen (pulse_train_output.rs):**
```rust
pub fn reset_position(&self) {
    let mut output = (self.get_output)();
    output.set_counter = true;
    output.set_counter_value = 0;
    (self.set_output)(output);
}

pub fn clear_set_counter(&self) {
    let mut output = (self.get_output)();
    output.set_counter = false;
    (self.set_output)(output);
}
```

### UI-Verbesserungen (Motors Page)

- 2x2 Grid fuer Inputs (Geschw/Beschl/Sollpos/Schritt)
- Button-Layout: START/STOP in Reihe 1, JOG-/FAHRE/JOG+/HOME in Reihe 2
- HOME-Button zeigt "STOP" waehrend Homing (pulsierend)
- Buerste mit CW/CCW Richtungswahl
- Zustand-Store fuer persistente Input-Werte (Zustand bleibt nach Tab-Wechsel)

### Naechste Schritte

1. [ ] Homing testen auf echter Hardware (Sensor angeschlossen an DI2)
2. [ ] Soft Limits implementieren (Achsen-Grenzen)
3. [ ] Tuer-Safety implementieren (Emergency Stop wenn Tuer offen)
4. [ ] Zweite EL2522 anschliessen (Druecker + Buerste)
5. [ ] Auto-Sequenz State Machine (19 Zyklen)
6. [ ] Wobble-Funktion fuer Schieber

### Commits (2026-02-05)

- `c733936c` - Implement proper homing sequence for BBM axes
- (weitere Commits aus Codex-Session siehe unten)

---

## Session 2026-02-05 (Codex): BBM UI Layout + Tower UI Workflow

### UI-Aenderungen (BBM Automatik V2)
- `electron/src/control/EditValue.tsx`: Compact-Layout fuer kleine Eingabefelder weiter justiert (Padding/Abstaende), Separator-Sichtbarkeit erzwungen, und neues `resetPlacement` eingefuehrt (Position des Reset-Pfeils im Popover).
- `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2MotorsPage.tsx`:
  - 4 Eingabefelder in einer Reihe (kompakt).
  - Einheiten wieder als Teil der Anzeige (bessere Lesbarkeit).
  - Reset-Pfeil jetzt **im Popover-Header** (oben rechts) statt auf der Card.
  - Buerste (Rotation) Eingabefeld auf gleiche Groesse wie die anderen Achsen (`compact`).
- `electron/src/machines/bbm/bbm_automatik_v2/useBbmAutomatikV2.ts`: Achsenname `MT` -> `Transporter`.

### Bugs/Probleme + Loesungen
- **JSX-Fehler**: `Expected corresponding JSX closing tag for <div>` in `BbmAutomatikV2MotorsPage.tsx` durch fehlendes `</div>` nach dem 4er-Grid.  
  **Fix:** fehlendes Closing-Tag eingefuegt.
- **Tower-UI zeigt "no data"**: Backend nicht erreichbar auf `localhost:3001` (SSH-Tunnel weg).  
  **Fix:** Tunnel neu starten:
  ```bash
  ssh -N -L 3001:localhost:3001 qitech@192.168.178.106
  ```
  und sicherstellen, dass der Server laeuft (`sudo systemctl restart qitech-control-server` falls noetig).
- **Tower-UI nicht gestartet**: Vite/Electron Dev-Server lief nicht.  
  **Fix:** `cd electron && npm run start`.

### Deploys (Commits)
1. `6b1828ec` - Make BBM motor inputs compact
2. `a15d276e` - Fix BBM motors layout closing div
3. `bd0ef67f` - Move BBM reset into edit popover
4. `56345158` - Add header reset option to EditValue
5. `947b0c1c` - Adjust BBM axis label and rotation input size

---

## Session 2026-02-06 [Claude Opus 4.6]: EL2522 Hardware Travel Distance Control

### Uebersicht

Implementierung des EL2522 Hardware Travel Distance Control Plans - Ersetzung der unzuverlaessigen Software-Bremslogik durch die eingebaute Hardware-Rampe der EL2522 fuer schrittverlustreies Bremsen.

### 1. HAL-Erweiterung (ethercat-hal)

~09:00 [Claude Opus 4.6]

- `ethercat-hal/src/io/pulse_train_output.rs`: `go_counter: bool` zu `PulseTrainOutputOutput` hinzugefuegt
- `ethercat-hal/src/devices/el2522.rs`: `go_counter` in `set_output()` und `get_output()` durchgereicht
- `select_end_counter` war bereits in `PulseTrainOutputInput` vorhanden

### 2. SDO-Write Infrastruktur (server)

~09:30 [Claude Opus 4.6]

- `server/src/app_state.rs`: `SdoWriteU16` Variante zu `HotThreadMessage` hinzugefuegt
- `server/src/loop.rs`: Handler fuer `SdoWriteU16` implementiert (SDO-Write via EtherCrab)
- `machines/src/lib.rs`: `SdoWriteU16Fn` Callback-Typ (`Arc<dyn Fn(usize, u16, u8, u16) + Send + Sync>`) und `sdo_write_u16` Feld in `MachineNewParams`
- `server/src/ethercat/setup.rs`: Callback-Konstruktion die `HotThreadMessage::SdoWriteU16` sendet
- `server/src/main.rs`: `sdo_write_u16: None` fuer Serial-Maschinen

### 3. get_ethercat_device erweitert

~10:00 [Claude Opus 4.6]

- `machines/src/lib.rs`: Return-Typ von 2-Tuple auf 3-Tuple erweitert: `(Arc<RwLock<T>>, &SubDeviceRef, usize)` - der `usize` ist der `subdevice_index`
- Alle Aufrufer aktualisiert: `bbm_automatik_v2/new.rs`, `schneidemaschine_v0/new.rs`, `buffer1/new.rs`

### 4. CoE-Konfiguration geaendert

~10:30 [Claude Opus 4.6]

- `machines/src/bbm_automatik_v2/new.rs` und `machines/src/schneidemaschine_v0/new.rs`:
  - `ramp_function_active: true` (vorher false)
  - `ramp_time_constant_rising: 2500` (100 mm/s² default)
  - `ramp_time_constant_falling: 2250` (10% steiler per Beckhoff-Vorgabe)

### 5. Maschinen-Structs erweitert

~11:00 [Claude Opus 4.6]

- **BbmAutomatikV2:** `last_ramp_update` entfernt, `sdo_write_u16` und `pto_subdevice_indices: [usize; 2]` hinzugefuegt
- **SchneidemaschineV0:** Gleiche Aenderungen, plus `axis_target_positions` von `[u32; 2]` auf `[i32; 2]` geaendert (signed fuer negative Positionen)

### 6. Software-Rampe durch Hardware-Monitor ersetzt

~11:30 [Claude Opus 4.6]

- `update_software_ramp()` komplett entfernt und durch `update_hardware_monitor()` ersetzt
- Position Mode: Prueft `input.select_end_counter` -> stoppt automatisch wenn Hardware Ziel meldet
- JOG Mode: Setzt nur Zielfrequenz, Hardware rampt selbststaendig
- `act.rs` in beiden Maschinen: Software-Ramp-Timing entfernt, nur noch Hardware-Monitor Aufruf

### 7. Position Mode umgeschrieben (move_to_position_mm)

~12:00 [Claude Opus 4.6]

- `go_counter = true` aktiviert Travel Distance Control
- `disble_ramp = false` nutzt Hardware-Rampe
- `target_counter_value` auf Zielposition gesetzt
- Richtung wird aus aktuelle vs. Zielposition berechnet

### 8. Stop-Funktionen angepasst

~12:30 [Claude Opus 4.6]

- `stop_axis` und `stop_all_axes`: `disble_ramp = true` fuer Sofort-Stop, `go_counter = false`

### 9. Dynamische Beschleunigung per SDO

~13:00 [Claude Opus 4.6]

- `set_axis_acceleration`: Berechnet Ramp-Zeiten und sendet SDO-Write an richtige EL2522
- Formel: `rising_ms = (base_freq / (accel_mm_s2 * PULSES_PER_MM)) * 1000`
- `falling_ms = rising_ms * 0.9` (10% steiler)

### Bugs und Loesungen

#### Bug 1: Build-Fehler - el2521.rs fehlte go_counter

~14:00 [Claude Opus 4.6]

- **Symptom:** GitHub Workflow "Rust" schlug fehl: `error[E0063]: missing field 'go_counter' in initializer of 'PulseTrainOutputOutput'`
- **Ursache:** `ethercat-hal/src/devices/el2521.rs` implementiert auch `PulseTrainOutputDevice` und hatte das neue `go_counter` Feld nicht
- **Loesung:** `go_counter` in `set_output()` und `get_output()` von el2521.rs hinzugefuegt
- **Betroffene Datei:** `ethercat-hal/src/devices/el2521.rs`

#### Bug 2: cargo nicht lokal verfuegbar

- **Symptom:** `cargo check` schlug fehl auf Windows
- **Ursache:** cargo nicht im PATH
- **Loesung:** Builds laufen immer ueber GitHub Workflows (`.github/workflows/rust.yml`), nicht lokal

### Konzept-Erklaerung: Hardware vs Software Rampe

| Modus | `disble_ramp` | `go_counter` | Verhalten |
|-------|--------------|--------------|-----------|
| JOG | `false` | `false` | Hardware rampt zur Zielfrequenz, kein Positionsziel |
| Position | `false` | `true` | Hardware rampt + bremst + stoppt exakt am Ziel |
| Stop | `true` | `false` | Sofort-Stop (Notfall/E-Stop) |

### Geaenderte Dateien (komplett)

| Datei | Aenderung |
|-------|-----------|
| `ethercat-hal/src/io/pulse_train_output.rs` | `go_counter` zu Output hinzugefuegt |
| `ethercat-hal/src/devices/el2522.rs` | `go_counter` durchgereicht |
| `ethercat-hal/src/devices/el2521.rs` | `go_counter` durchgereicht (Bug-Fix) |
| `server/src/app_state.rs` | `SdoWriteU16` Variante |
| `server/src/loop.rs` | SDO-Write Handler |
| `machines/src/lib.rs` | `SdoWriteU16Fn`, `sdo_write_u16` in Params, 3-Tuple `get_ethercat_device` |
| `server/src/ethercat/setup.rs` | SDO-Write Callback Konstruktion |
| `server/src/main.rs` | `sdo_write_u16: None` |
| `machines/src/bbm_automatik_v2/mod.rs` | Hardware-Monitor, Position Mode, Stop, SDO-Acceleration |
| `machines/src/bbm_automatik_v2/new.rs` | CoE-Config, neue Felder |
| `machines/src/bbm_automatik_v2/act.rs` | Software-Ramp entfernt |
| `machines/src/schneidemaschine_v0/mod.rs` | Gleiche Aenderungen |
| `machines/src/schneidemaschine_v0/new.rs` | CoE-Config, neue Felder |
| `machines/src/schneidemaschine_v0/act.rs` | Software-Ramp entfernt |
| `machines/src/schneidemaschine_v0/api.rs` | `axis_target_positions` Typ i32 |
| `machines/src/buffer1/new.rs` | 3-Tuple Destructuring |

### Status

- Build laeuft auf GitHub Actions (nach el2521.rs Fix)
- Hardware-Test steht aus

### Naechste Schritte

- [x] Build-Ergebnis pruefen (GitHub Workflow) - erledigt
- [x] Hardware-Test: Position Mode mit `select_end_counter` - erledigt 2026-02-09
- [x] Hardware-Test: JOG Mode mit Hardware-Rampe - erledigt 2026-02-09
- [ ] Hardware-Test: Verschiedene Beschleunigungen per SDO

---

## Git Branch Workflow (ab 2026-02-09)

### Warum Branches?

**Vorher:** Alle Aenderungen direkt auf `master` -> bei Fehlern ist sofort die Produktion betroffen.

**Jetzt:** Feature-Branches fuer neue Entwicklungen -> `master` bleibt immer stabil.

### Konzept

```
master (stabil, getestet)
  |
  +-- feature/hardware-travel-distance-control  (aktuelle Entwicklung)
  |
  +-- feature/naechstes-feature                 (zukuenftig)
```

- **`master`**: Nur getesteter, funktionierender Code. Kann jederzeit deployed werden.
- **`feature/*`**: Neue Entwicklungen. Koennen kaputt sein, ohne master zu beeinflussen.
- **Merge**: Wenn ein Feature fertig und getestet ist, wird es in `master` gemerged.

### Workflow

1. **Neues Feature starten:**
   ```bash
   git checkout master
   git checkout -b feature/mein-neues-feature
   ```

2. **Entwickeln und committen** (auf dem Feature-Branch):
   ```bash
   git add <dateien>
   git commit -m "Beschreibung"
   git push origin feature/mein-neues-feature
   ```

3. **Feature-Branch deployen** (zum Testen auf Hardware):
   ```bash
   gh workflow run fast-deploy.yml --ref feature/mein-neues-feature
   ```

4. **Feature fertig -> in master mergen:**
   ```bash
   git checkout master
   git merge feature/mein-neues-feature
   git push origin master
   ```

5. **Master deployen** (stabile Version):
   ```bash
   gh workflow run fast-deploy.yml --ref master
   ```

### Deploy-Workflow (fast-deploy.yml)

Der Workflow unterstuetzt beliebige Branches:
- Holt den neuesten Stand mit `git fetch origin`
- Wechselt zum Branch mit `git checkout "$BRANCH"`
- Setzt auf den neuesten Remote-Stand mit `git reset --hard "origin/$BRANCH"`
- Baut mit `nixos-rebuild switch`

**Wichtig:** Der Server laeuft immer auf dem zuletzt deployte Branch. Nach einem Deploy den aktuellen Branch pruefen:
```bash
ssh qitech@nixos "cd /home/qitech/control && git branch --show-current"
```

### UI nach Deploy neu starten

Das Electron UI ist eine Desktop-App (kein systemd Service). Nach einem Deploy muss es manuell neu gestartet werden:

```bash
# Via SSH auf dem Server:
ssh qitech@nixos "pkill -f qitech-control-electron"
# GNOME Autostart startet die App automatisch neu
```

Oder im Deploy-Workflow: Der `pkill` Befehl wird nach dem `nixos-rebuild switch` ausgefuehrt.

---

## Session 2026-02-09: Step Loss Fix & Branch Workflow

### Uebersicht

Kritischer Bug-Fix fuer Schrittverluste bei Hardware Travel Distance Control. Drei separate Probleme identifiziert und behoben durch Multi-Agent-Analyse.

### 1. txpdo_toggle Bug (KRITISCH)

~10:00 [Claude Opus 4.6]

**Problem:** `PtoStatus::read()` und `EncStatus::read()` in `ethercat-hal/src/pdo/el252x.rs` hatten einen Early-Return wenn `txpdo_toggle == false`. Das bedeutete: **50% aller Hardware-Status-Reads wurden uebersprungen.**

**Auswirkung:** `select_end_counter` (Ziel erreicht), `ramp_active`, `error` etc. wurden nur bei jedem zweiten EtherCAT-Zyklus gelesen.

**Fix:**
```rust
// VORHER (fehlerhaft):
fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
    self.txpdo_toggle = bits[8 + 7];
    if !self.txpdo_toggle {
        return;  // 50% der Daten ignoriert!
    }
    self.select_end_counter = bits[0];
    // ...
}

// NACHHER (korrekt):
fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
    self.select_end_counter = bits[0];
    self.ramp_active = bits[1];
    // ... alle Felder lesen
    self.txpdo_toggle = bits[8 + 7];
}
```

**Datei:** `ethercat-hal/src/pdo/el252x.rs`
**Tests:** 2 neue Tests hinzugefuegt (`test_pto_status_toggle_false`, `test_enc_status_toggle_false`)

### 2. Aggressive Bremsrampe

~10:30 [Claude Opus 4.6]

**Problem:** `falling_ms = rising_ms * 0.9` (10% steilere Bremsrampe) verursachte Schrittverluste beim Bremsen.

**Fix:** `falling_ms = rising_ms` (gleiche Rampe fuer Beschleunigen und Bremsen)

**Dateien:**
- `machines/src/bbm_automatik_v2/mod.rs`
- `machines/src/schneidemaschine_v0/mod.rs`

### 3. JOG-Regression durch txpdo_toggle Fix

~14:00 [Claude Opus 4.6]

**Problem:** Nach dem txpdo_toggle Fix funktionierte JOG nicht mehr. Ursache: Stale `select_end_counter` von vorherigen Positionsfahrten wurde jetzt sofort erkannt (vorher durch den 50%-Skip maskiert) und brach neue Bewegungen sofort ab.

**Fix:** Grace Period von 5 EtherCAT-Zyklen nach Start einer neuen Positionsfahrt:
```rust
// Bei move_to_position_mm:
self.axis_position_ignore_cycles[index] = 5;

// In update_hardware_monitor:
if self.axis_position_mode[i] {
    if self.axis_position_ignore_cycles[i] > 0 {
        self.axis_position_ignore_cycles[i] -= 1;
    } else if input.select_end_counter {
        // Ziel erreicht
    }
}
```

**Dateien:**
- `machines/src/bbm_automatik_v2/mod.rs` + `new.rs`
- `machines/src/schneidemaschine_v0/mod.rs` + `new.rs`

### 4. Position Verification (Schrittverlusst-Erkennung)

~10:30 [Claude Opus 4.6]

Nach Erreichen der Zielposition wird die Abweichung geprueft:
```rust
let deviation = (actual_pos - target_pos).abs();
if deviation > 2 {
    tracing::warn!("[Axis {}] STEP LOSS DETECTED: target={} actual={} deviation={}", ...);
}
```

### 5. Deploy-Workflow fuer Feature-Branches

~12:00 [Claude Opus 4.6]

**Problem:** `fast-deploy.yml` nutzte `git pull --ff-only` was fehlschlug wenn der Server auf `master` war aber ein Feature-Branch deployed werden sollte.

**Fix:** Workflow geaendert auf `git fetch` + `git checkout` + `git reset --hard` (siehe Branch Workflow Sektion oben).

### Commits

1. `febcdcb1` - Fix step loss: txpdo_toggle bug, aggressive braking, add position verification
2. `47523e05` - Fix fast-deploy: support deploying any branch, not just current
3. `f2cad828` - Fix JOG after position move: add grace period for select_end_counter
4. `8ff1ca1a` - Fix direction: frequency_value must be positive in TDC mode
5. `cf614729` - Fix deploy workflow pkill scope
6. `691ada45` - Deploy workflow fix continued
7. `da2acb29` - Deploy workflow finalized

### Geaenderte Dateien

| Datei | Aenderung |
|-------|-----------|
| `ethercat-hal/src/pdo/el252x.rs` | txpdo_toggle Early-Return entfernt, 2 Tests |
| `machines/src/bbm_automatik_v2/mod.rs` | Bremsrampe, Position-Verify, Ignore-Cycles |
| `machines/src/bbm_automatik_v2/new.rs` | `axis_position_ignore_cycles` Init |
| `machines/src/schneidemaschine_v0/mod.rs` | Gleiche Fixes |
| `machines/src/schneidemaschine_v0/new.rs` | `axis_position_ignore_cycles` Init |
| `.github/workflows/fast-deploy.yml` | Branch-Support, pkill-Scope Fix, Server-Restart separiert |

### 6. Direction Fix (frequency_value muss positiv sein)

~15:00 [Claude Opus 4.6]

**Problem:** Motor fuhr nur in eine Richtung (vorwaerts). Rueckwaertsfahrt funktionierte nicht - Motor blieb stehen oder fuhr weiter vorwaerts.

**Ursache:** In `move_to_position_mm()` wurde die Frequenz mit der Richtung multipliziert:
```rust
// VORHER (fehlerhaft):
output.frequency_value = speed_hz * direction;  // direction = -1 bei Rueckwaerts
```

Im Travel Distance Control Modus (`go_counter = true`) bestimmt die EL2522-Hardware die Fahrtrichtung **automatisch** durch Vergleich von `target_counter_value` mit der aktuellen Position. Ein negativer `frequency_value` kollidierte mit dieser automatischen Richtungssteuerung - die Hardware interpretierte den negativen Wert nicht als "rueckwaerts", sondern als ungueltigen/widerspruchlichen Befehl.

**Fix:** `frequency_value` immer positiv (nur Betrag/Magnitude):
```rust
// NACHHER (korrekt):
output.frequency_value = speed_hz;  // Immer positiv, Hardware bestimmt Richtung
```

**Dateien:**
- `machines/src/bbm_automatik_v2/mod.rs`
- `machines/src/schneidemaschine_v0/mod.rs`

**Commit:** `8ff1ca1a` - Fix direction: frequency_value must be positive in TDC mode

### 7. Deploy-Workflow Fix (pkill zu breit)

~15:30 [Claude Opus 4.6]

**Problem:** `pkill qitech-control-` im Deploy-Workflow war zu breit gefasst und killte nicht nur die Electron-App sondern auch den `qitech-control-server` (systemd Service). Das fuehrte dazu, dass nach einem Deploy der Server kurz offline war.

**Fix:** Zwei separate Schritte statt einem breiten `pkill`:
1. `pkill -x qitech-control-e` - killt nur die Electron-App (exakter Match)
2. Separater Server-Restart-Schritt: `sudo systemctl restart qitech-control-server`

**Commits:**
- `cf614729` - Fix deploy workflow pkill scope
- `691ada45` - Weiterer Deploy-Workflow Fix
- `da2acb29` - Deploy-Workflow Finalisierung

### Hardware-Test-Ergebnis

- [x] Position Mode: Funktioniert nach txpdo_toggle Fix
- [x] JOG Mode: Funktioniert nach Grace-Period Fix
- [x] Richtung: Vorwaerts und Rueckwaerts funktioniert nach frequency_value Fix
- [ ] Schrittverlust-Log pruefen (STEP LOSS DETECTED Warnung im Journal)
- [ ] Verschiedene Beschleunigungen testen

### 8. UI-Verbesserungen (Nachmittag)

~16:00 [Claude Opus 4.6]

Diverse UI-Verbesserungen fuer BBM Automatik V2 auf Basis von Bediener-Feedback.

#### 8.1 JOG-Button Beschriftung

**Aenderung:** Symbole vor den Text verschoben fuer bessere Lesbarkeit auf Touchscreen.

| Vorher | Nachher |
|--------|---------|
| `JOG+` | `+ JOG` |
| `JOG-` | `- JOG` |

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2MotorsPage.tsx`

#### 8.2 Input-Limits angepasst

Eingabefelder auf sinnvolle Bereiche begrenzt (Soft Limits passend zur Mechanik):

| Feld | Vorher | Nachher |
|------|--------|---------|
| Schrittweite (Step Size) | 1-1000 mm | 0-200 mm |
| Sollposition (Target Position) | 0-10000 mm | 0-500 mm |

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2MotorsPage.tsx`

#### 8.3 Endlage-Labels auf Status-Seite

**Aenderung:** Beschriftung der Referenzschalter-Anzeigen von "Referenz" auf "Endlage" geaendert (entspricht der tatsaechlichen Funktion - es sind Endlagenschalter, keine Referenzschalter).

| Vorher | Nachher |
|--------|---------|
| Referenz MT | Endlage MT |
| Referenz Schieber | Endlage Schieber |
| Referenz Druecker | Endlage Druecker |

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2StatusPage.tsx`

#### 8.4 Geschwindigkeits-Preset Farben (Ampel-System)

Speed-Preset-Buttons auf der Auto-Seite mit Ampelfarben fuer intuitive Bedienung:

| Preset | Farbe | Bedeutung |
|--------|-------|-----------|
| Langsam | Gruen | Sicher, Einrichten |
| Mittel | Gelb | Normal |
| Schnell | Rot | Volle Geschwindigkeit |

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2AutoPage.tsx`

#### 8.5 Speed-Presets auf Testsequenz-Seite

Gleiche 3 Geschwindigkeits-Buttons (Langsam/Mittel/Schnell mit Gruen/Gelb/Rot) auch auf der Test-Seite hinzugefuegt. Vorher war dort keine Geschwindigkeitsauswahl moeglich.

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2TestPage.tsx`

#### 8.6 Auto STOP-Button Zustandsanzeige

STOP-Button auf der Auto-Seite zeigt jetzt den Zustand visuell an:
- **Grau** wenn Automatik inaktiv (nichts zum Stoppen)
- **Rot** wenn Automatik laeuft (aktive Stopp-Moeglichkeit)

Entspricht dem gleichen Pattern wie auf der Motors-Seite.

**Datei:** `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2AutoPage.tsx`

#### 8.7 Default-Geschwindigkeit = Langsam

Beim Laden der Auto- und Test-Seite ist jetzt "Langsam" (gruener Button) vorausgewaehlt statt "Mittel". Sicherheitsmassnahme: Maschine startet immer mit der langsamsten Geschwindigkeit.

**Dateien:**
- `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2AutoPage.tsx`
- `electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2TestPage.tsx`

#### 8.8 Sidebar-Routing Fix (BBM Automatik -> Auto Tab)

**Problem:** Klick auf "BBM Automatik" in der Sidebar fuehrte zu einer "not found" Seite, weil kein Default-Tab definiert war.

**Loesung:** Neues `defaultTab` Feld in der Maschinen-Konfiguration eingefuehrt. BBM Automatik V2 setzt `defaultTab: "auto"`, so dass der Sidebar-Link direkt zum Auto-Tab navigiert.

**Geaenderte Dateien:**
- `electron/src/machines/types.ts` - `defaultTab` Feld zu `MachineProperties` Typ hinzugefuegt
- `electron/src/machines/properties.ts` - `defaultTab: "auto"` fuer BBM Automatik V2 gesetzt
- `electron/src/machines/useMachines.tsx` - `defaultTab` aus Properties durchgereicht
- `electron/src/sidebar/SidebarLayout.tsx` - Navigation nutzt `defaultTab` wenn vorhanden

### Commits (UI-Verbesserungen)

8. `93da9016` - UI improvements: JOG labels, input limits, Endlage labels, speed colors
9. `df804fae` - Add speed presets to Testsequenz page, fix Auto stop button
10. `6f4fa6ae` - Fix default speed to Langsam, fix sidebar routing to Auto tab

### Tages-Zusammenfassung (2026-02-09)

Alle Commits dieser Session im Ueberblick:

| # | Commit | Beschreibung | Bereich |
|---|--------|-------------|---------|
| 1 | `febcdcb1` | Fix step loss: txpdo_toggle bug, aggressive braking, add position verification | Backend/HAL |
| 2 | `47523e05` | Fix fast-deploy: support deploying any branch | DevOps |
| 3 | `f2cad828` | Fix JOG after position move: add grace period for select_end_counter | Backend |
| 4 | `8ff1ca1a` | Fix direction: frequency_value must be positive in TDC mode | Backend |
| 5 | `cf614729` | Add UI restart to deploy workflow, document branch workflow | DevOps |
| 6 | `691ada45` | Fix deploy: separate UI restart into own SSH step | DevOps |
| 7 | `da2acb29` | Fix deploy: pkill pattern was too broad, killed server too | DevOps |
| 8 | `93da9016` | UI improvements: JOG labels, input limits, Endlage labels, speed colors | Frontend |
| 9 | `df804fae` | Add speed presets to Testsequenz page, fix Auto stop button | Frontend |
| 10 | `6f4fa6ae` | Fix default speed to Langsam, fix sidebar routing to Auto tab | Frontend |

**Schwerpunkte:** Hardware Travel Distance Control Bugfixes (1-4), Deploy-Workflow Fixes (5-7), UI-Verbesserungen (8-10)

---

## Session 2026-02-09 (Nachtrag): Soft Limits

### Soft Limits fuer BBM Automatik V2

~17:00 [Claude Opus 4.6]

Soft Limits aus dem Arduino-Code (v3.2) uebernommen. Verhindert dass Achsen ueber mechanische Grenzen hinausfahren.

**Neues Modul in `machines/src/bbm_automatik_v2/mod.rs`:**
```rust
pub mod soft_limits {
    pub const MIN_MM: f32 = 0.0;
    pub const MT_MAX_MM: f32 = 230.0;
    pub const SCHIEBER_MAX_MM: f32 = 53.0;
    pub const DRUECKER_MAX_MM: f32 = 107.0;

    pub fn max_position_mm(axis: usize) -> Option<f32> {
        match axis {
            0 => Some(MT_MAX_MM),
            1 => Some(SCHIEBER_MAX_MM),
            2 => Some(DRUECKER_MAX_MM),
            _ => None, // Buerste hat keine Soft Limits
        }
    }
}
```

**Enforcement:**
- In `move_to_position_mm()`: Zielposition wird auf Soft Limits geclampt
- In `update_hardware_monitor()` (JOG): Speed wird auf 0 gesetzt wenn Limit erreicht
- Warnung im Log: `[BbmAutomatikV2] Axis X soft limit reached at Y.Y mm - stopping`

**Commit:** `072f2088` - Add soft limits for BBM Automatik V2 axes (from Arduino v3.2)

---

## Session 2026-02-11: Motor-Alarm-Monitoring, kritische Bug-Fixes, CI-Pipeline

### Uebersicht

Umfangreiche Session mit drei Schwerpunkten:
1. **Motor-Alarm-Monitoring** fuer BBM Automatik V2 (CL75t Treiber-Alarme)
2. **Kritische Bug-Fixes** in 20+ Dateien (unsafe code, panics, etc.)
3. **CI-Pipeline** vollstaendig grueen bekommen (Rust + Electron + Nix)

Branch: `feature/motor-alarm-monitoring`
PR: https://github.com/mitgefuehlt-lang/control/pull/1

### 1. Motor-Alarm-Monitoring (Plan implementiert)

~09:00 [Claude Opus 4.6]

**Hintergrund:** Die BBM Automatik V2 nutzt CL75t Schrittmotor-Treiber. Diese haben einen Alarm-Pin (AL) der bei Ueberstrom, Ueberhitzung etc. ausloest. Im Arduino-Code (`BBMx22_Automatik_Code.ino` v3.2, Zeile 717-738) wurde `checkDriverAlarms()` jeden Zyklus aufgerufen - bei Alarm sofort `emergencyStopAll()`.

**Neue DI-Belegung (EL1008):**

| DI | Index | Funktion | Vorher |
|----|-------|----------|--------|
| DI1 | 0 | REF_MT | unveraendert |
| DI2 | 1 | REF_SCHIEBER | unveraendert |
| DI3 | 2 | REF_DRUECKER | unveraendert |
| DI4 | 3 | ALARM_MT | war TUER_1 |
| DI5 | 4 | ALARM_SCHIEBER | war TUER_2 |
| DI6 | 5 | ALARM_DRUECKER | neu |
| DI7 | 6 | TUER (eine Tuer) | neu |
| DI8 | 7 | frei | unveraendert |

**Alarm-Polaritaet:** Active LOW (Open-Collector, `false` = Alarm) - wie Arduino `ALARM_ACTIVE_HIGH = false`.

#### 1.1 Backend-Aenderungen

**`machines/src/bbm_automatik_v2/mod.rs`:**
- `inputs` Modul aktualisiert: `TUER_1`/`TUER_2` entfernt, `ALARM_MT`/`ALARM_SCHIEBER`/`ALARM_DRUECKER`/`TUER` hinzugefuegt
- `ALARM_ACTIVE_LOW: bool = true` Konstante
- `axis_alarm_active: [bool; 4]` Feld im Struct
- `check_driver_alarms()` - prueft Alarm-Pins, stoppt alle Achsen bei Alarm
- `reset_alarms()` - mit Hardware-Pin-Validierung (prueft ob physischer Alarm noch aktiv)
- `are_doors_closed()` auf einen Sensor umgestellt

**`machines/src/bbm_automatik_v2/act.rs`:**
- `check_driver_alarms()` wird VOR `update_hardware_monitor()` aufgerufen (hoechste Prioritaet)

**`machines/src/bbm_automatik_v2/api.rs`:**
- `axis_alarm_active: [bool; 4]` in `StateEvent`
- `ResetAlarms` Mutation hinzugefuegt

**`machines/src/bbm_automatik_v2/new.rs`:**
- `axis_alarm_active: [false; 4]` initialisiert

#### 1.2 Frontend-Aenderungen

**`electron/src/machines/bbm/bbm_automatik_v2/bbmAutomatikV2Namespace.ts`:**
- `axis_alarm_active: z.tuple([z.boolean(), z.boolean(), z.boolean(), z.boolean()])` zum Schema

**`electron/src/machines/bbm/bbm_automatik_v2/useBbmAutomatikV2.ts`:**
- `INPUT` Konstanten aktualisiert (ALARM_MT/SCHIEBER/DRUECKER + TUER)
- `resetAlarms` Mutation, `getAxisAlarmActive`, `isAnyAlarmActive` Helper
- `areDoorsClosedFn` auf einen Sensor

**`electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2MotorsPage.tsx`:**
- Pro Achse: rotes "TREIBER ALARM" Banner wenn `axis_alarm_active[i]`
- Global: "ALARM RESET" Button sichtbar wenn irgendein Alarm aktiv

**`electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2AutoPage.tsx`:**
- Von 2-Tuer auf 1-Tuer Anzeige umgestellt

**`electron/src/machines/bbm/bbm_automatik_v2/BbmAutomatikV2StatusPage.tsx`:**
- Input-Namen aktualisiert (Alarm-Pins statt Tuersensoren)

### 2. Kritische Bug-Fixes (learnings1.md Analyse)

~09:30 [Claude Opus 4.6]

Ein Agent-Swarm analysierte die gesamte Codebase und fand 7 kritische Bug-Kategorien. Alle wurden behoben.

#### 2.1 Unsafe Static Mut (KRITISCH)

**Problem:** `unsafe static mut LAST_DEBUG: Option<Instant>` in `act.rs` - undefined behavior in Rust, Thread-Safety-Verletzung.

**Auswirkung:** Mehrere Maschinen-Instanzen teilen sich denselben statischen Speicher. Bei gleichzeitigem Zugriff: Data Race, potentieller Crash oder korrupte Daten.

**Fix:** In Struct-Feld `last_debug_log: Option<Instant>` verschoben.

**Dateien:**
- `machines/src/bbm_automatik_v2/act.rs` + `mod.rs` + `new.rs`
- `machines/src/schneidemaschine_v0/act.rs` + `mod.rs` + `new.rs`

#### 2.2 expect() Panics (KRITISCH)

**Problem:** `expect()` in `RequestValues` Handler aller 16 Maschinen-`act.rs` Dateien. Ein Serialisierungs-Fehler haette den gesamten Maschinen-Thread gekillt.

**Auswirkung:** Thread-Tod = Maschine reagiert nicht mehr, alle Achsen laufen mit letztem Befehl weiter.

**Fix:** `unwrap_or_else(|e| { tracing::error!(...); serde_json::Value::Null })` - loggt Fehler, gibt Null zurueck.

**Betroffene Dateien (16 Stueck):**
- `bbm_automatik_v2/act.rs`, `schneidemaschine_v0/act.rs`, `lib.rs`
- `winder2/act.rs`, `winder2/mock/act.rs`
- `extruder1/act.rs`, `extruder1/mock/act.rs`
- `extruder2/act.rs`, `extruder2/mock/act.rs`
- `laser/act.rs`, `buffer1/act.rs`, `aquapath1/act.rs`
- `wago_power/act.rs`, `test_machine/act.rs`
- `analog_input_test_machine/act.rs`, `ip20_test_machine/act.rs`

#### 2.3 todo!() Panics (HOCH)

**Problem:** `todo!()` Makros = sofortiger Panic wenn der Code-Pfad erreicht wird.

**Auswirkung:** Thread-Tod bei bestimmten Messages.

**Fix:** Durch `tracing::warn!()` + No-Op ersetzt.

**Dateien:**
- `machines/src/laser/act.rs` - `None => todo!()` bei fehlender Seriennummer
- `machines/src/buffer1/mod.rs` - `fill_buffer()` und `empty_buffer()`
- `machines/src/winder2/api.rs` - `SetPullerTargetDiameter`
- `machines/src/winder2/mock/api.rs` - gleich
- `machines/src/lib.rs` - `ConnectToMachine`/`DisconnectMachine`

#### 2.4 Alarm-Reset ohne Hardware-Validierung (MITTEL)

**Problem:** `reset_alarms()` setzte `axis_alarm_active` auf `false` ohne zu pruefen ob der physische Alarm-Pin noch aktiv ist.

**Fix:** Prueft jetzt die physischen Alarm-Pins bevor Reset erlaubt wird:
```rust
pub fn reset_alarms(&mut self) {
    for &(axis, input_idx) in &alarm_inputs {
        let raw = self.digital_inputs[input_idx].get_value().unwrap_or(!ALARM_ACTIVE_LOW);
        let still_alarm = if ALARM_ACTIVE_LOW { !raw } else { raw };
        if still_alarm {
            tracing::warn!("[BbmAutomatikV2] Cannot reset axis {} - alarm still active", axis);
            continue;
        }
        self.axis_alarm_active[axis] = false;
    }
}
```

#### 2.5 SDO-Write Debug-Logging (NIEDRIG)

**Problem:** `SdoWriteU16Fn` gibt `()` zurueck, Fehler werden verschluckt.

**Fix:** Debug-Logging vor jedem SDO-Write + Warnung wenn kein Writer verfuegbar.

**Dateien:** `bbm_automatik_v2/mod.rs`, `schneidemaschine_v0/mod.rs`

#### 2.6 Laser Emission Rate Kommentar (NIEDRIG)

**Problem:** Kommentar sagte "60 FPS" aber Code emittiert bei 30 Hz.

**Fix:** Kommentar auf "~30 Hz" korrigiert.

**Datei:** `machines/src/laser/act.rs`

### 3. CI-Pipeline - Alle Workflows gruen

#### 3.1 React Import Fix (Electron CI - TypeScript)

**Problem:** `tsconfig.json` nutzt `"jsx": "react"` was expliziten `import React from "react"` erfordert.

**Betroffene Dateien (8 Stueck):**

SchneidemaschineV0 (3 Dateien):
- `SchneidemaschineV0Page.tsx`
- `SchneidemaschineV0MotorsPage.tsx`
- `SchneidemaschineV0ControlPage.tsx`

BBM Automatik V2 (5 Dateien):
- `BbmAutomatikV2Page.tsx`
- `BbmAutomatikV2MotorsPage.tsx`
- `BbmAutomatikV2StatusPage.tsx`
- `BbmAutomatikV2TestPage.tsx`
- `BbmAutomatikV2AutoPage.tsx`

**Commits:**
- `d3765d48` - Fix missing React import in SchneidemaschineV0 TSX files
- `91a4c590` - Fix missing React import in BBM Automatik V2 TSX files

#### 3.2 Rustfmt Fix (Rust CI - cargo fmt)

**Problem:** CI nutzt `rustfmt --edition 2024` mit strengeren Regeln als der lokale Linter:
- `unwrap_or_else` Bloecke: `let state = serde_json::to_value(...)` darf auf eine Zeile wenn kurz genug, muss umbrechen wenn zu lang
- `tracing::` Makro-Argumente: jedes Argument auf eigene Zeile
- Kommentar-Alignment: keine Extra-Spaces vor Inline-Kommentaren

**Betroffene Dateien (6 Stueck):**
- `bbm_automatik_v2/act.rs`, `bbm_automatik_v2/mod.rs`
- `extruder1/mock/act.rs`, `extruder2/act.rs`, `extruder2/mock/act.rs`
- `winder2/act.rs`, `winder2/mock/act.rs`
- `ip20_test_machine/act.rs`
- `lib.rs`
- `schneidemaschine_v0/mod.rs`

**Commits:**
- `b7940cee` - Fix rustfmt formatting in act.rs files and mod.rs
- `5c27e1c2` - Fix rustfmt edition 2024 formatting across all modified files

#### 3.3 Prettier Fix (Electron CI - Code Style)

**Problem:** `prettier --check .` fand Formatting-Issues in 11 Dateien.

**Fix:** `npx prettier --write` auf alle 11 Dateien ausgefuehrt.

**Commit:** `84061853` - Fix Prettier formatting in all modified frontend files

#### 3.4 Finaler CI-Status

| Workflow | Status | Dauer |
|----------|--------|-------|
| Rust (fmt + build + test + mock) | PASSED | ~1m50s |
| Electron (format + compile + lint + test) | PASSED | ~1m30s |
| Nix CI (build + flake check) | PASSED | ~4m48s |

**Lint-Warnings (nicht blockierend, pre-existing):**
- `addLogEntry` unused in BbmAutomatikV2StatusPage.tsx
- `areDoorsClosed` unused in BbmAutomatikV2AutoPage.tsx
- Diverse andere unused variables in bestehenden Dateien

### 4. PR #1 erstellt

**URL:** https://github.com/mitgefuehlt-lang/control/pull/1
**Branch:** `feature/motor-alarm-monitoring` -> `master`
**Alle Checks:** Gruen

### 5. Codebase-Analyse (4 Agents)

Vier parallel laufende Analyse-Agents untersuchten die gesamte Codebase. Ergebnisse als Referenz fuer zukuenftige Implementierungen:

#### 5.1 UI Features (Agent ae63323)

**Fehlende UI-Features (Prioritaet):**
- Kein Machine Overview Dashboard (alle Maschinen auf einen Blick)
- Keine Breadcrumbs-Navigation
- Keine Tooltips/Inline-Hilfe
- BBM Automatik V2 hat keine Graphen (Winder/Extruder haben welche)
- Kein responsives Layout fuer Tablets (feste Spaltenbreite)
- Keine Custom Dashboards
- Kein PDF-Export (nur XLSX)
- Keine Parametervergleichs-UI (vorher/nachher)

#### 5.2 Machine Features (Agent a4f9c91)

**Fehlende Backend-Features (Prioritaet):**
- Zyklusautomatik (19-Zyklen Sequenz) nur als UI-Stub
- Keine State Persistence (Zustand geht bei Neustart verloren)
- Kein Data Logging / Produktionszaehler
- Keine Kalibrierung / Wartungstracking
- Keine Fehlercodes (nur generische Alarm-Flags)
- Wobble-Funktion fuer Schieber nicht implementiert
- Keine Rezept-/Programmverwaltung

#### 5.3 Safety & Monitoring (Agent a14f83c)

**Kritische Sicherheitsluecken:**

| Bereich | Status | Risiko |
|---------|--------|--------|
| EtherCAT Watchdog | Deaktiviert (fuer TDC benoetigt) | KRITISCH |
| EtherCAT Link-Loss Detection | Nicht implementiert | KRITISCH |
| Globaler E-Stop | Fehlt (nur per-Maschine) | HOCH |
| Tuer-Interlock Enforcement | Nur angezeigt, nicht erzwungen | MITTEL |
| Kollisionserkennung zwischen Achsen | Nicht implementiert | MITTEL |
| Motor-Gesundheit (Strom, Temperatur) | Nur Alarm-Pin | MITTEL |
| Alarm-Historie | Nicht persistent | MITTEL |

**Was funktioniert:**
- Loop Jitter Messung
- Driver Alarm Detection (BBM nur)
- Soft Limit Enforcement
- Position Feedback via PTO Counter
- Homing State Machine
- System Resource Metrics (REST API)

#### 5.4 Display & Data (Agent aaaf795)

**Visualisierung:**
- uPlot (v1.6.32) aktiv fuer Graphen (Winder, Extruder, Aquapath, Laser)
- SVG-basierte 2D-Visualisierungen (Spool, TensionArm, TraverseBar)
- Keine 3D-Modelle, keine interaktiven Maschinendiagramme

**Daten-Features:**
- Excel-Export vorhanden (xlsx Library)
- Kein CSV/PDF-Export
- Preset-System vorhanden (JSON Import/Export)
- Keine Produktions-Reports, keine Schicht-/Tagesberichte

**Responsive Design:**
- Touch-optimierte Button-Groessen (>44px)
- Keine Tailwind Breakpoints (md:/lg:/xl:)
- Feste Sidebar-Breite (192px), kein Hamburger-Menu

### 6. learnings1.md erstellt

**Datei:** `Kailar-Doku/learnings1.md`
**Inhalt:** Vollstaendige Deep-Analysis der Codebase mit konkreten Bugs, Schwachstellen und Verbesserungsvorschlaegen. Kategorien: Kritische Bugs, Sicherheitsluecken, fehlende Features, Architektur-Empfehlungen.

### Commits (2026-02-11)

| # | Commit | Beschreibung | Bereich |
|---|--------|-------------|---------|
| 1 | `c547dd8a` | Add motor alarm monitoring, fix critical bugs across all machines | Backend+Frontend |
| 2 | `d3765d48` | Fix missing React import in SchneidemaschineV0 TSX files | Frontend |
| 3 | `b7940cee` | Fix rustfmt formatting in act.rs files and mod.rs | Backend |
| 4 | `91a4c590` | Fix missing React import in BBM Automatik V2 TSX files | Frontend |
| 5 | `5c27e1c2` | Fix rustfmt edition 2024 formatting across all modified files | Backend |
| 6 | `84061853` | Fix Prettier formatting in all modified frontend files | Frontend |

### Wichtige Erkenntnisse

1. **CI-Workflows muessen manuell gestartet werden:** `gh workflow run <name> --ref <branch>` - sie laufen nicht automatisch bei Push
2. **Nix CI laeuft immer gruen** - muss nicht jedes Mal gestartet werden
3. **rustfmt --edition 2024** hat strengere Regeln als lokaler Linter:
   - `tracing::` Makro-Argumente: jedes auf eigene Zeile wenn Format-String lang
   - `unwrap_or_else` Closure: Einrueckung abhaengig von Zeilenlaenge
   - Keine Extra-Spaces fuer Kommentar-Alignment
4. **`import React from "react"`** muss in allen TSX-Dateien stehen wegen `tsconfig.json` `"jsx": "react"`
5. **Prettier** muss vor dem Commit auf allen geaenderten Frontend-Dateien laufen
6. **Alarm-Polaritaet CL75t:** Active LOW (Open-Collector) - `false` auf DI = Alarm aktiv

### Naechste Schritte

Basierend auf den Analyse-Ergebnissen, priorisiert:

1. [ ] PR #1 mergen (alle Checks gruen)
2. [x] Zyklusautomatik implementieren (19-Zyklen Sequenz - Kern-Feature) → Session 2026-02-11 #2
3. [x] Tuer-Interlock enforcen (Motor-Stop wenn Tuer offen) → Session 2026-02-11 #2
4. [ ] EtherCAT Link-Loss Detection
5. [ ] BBM Graphen-Seite erstellen
6. [ ] Produktionszaehler (Zyklen, Sets)
7. [ ] Alarm-Historie persistent machen
8. [ ] Machine Overview Dashboard

---

## Session 2026-02-11 #2: Door Interlock + 19-Cycle Automation

**Branch:** `feature/door-interlock-and-cycle-automation`
**Commit:** `2ccfa7a5` - Add door interlock and 19-cycle auto-sequence state machine
**Deployed:** Ja, live auf Mini-PC (nixos-rebuild switch)

### Was wurde implementiert

#### 1. Door Interlock Enforcement

Tuersensor-Ueberwachung waehrend Betrieb. Wenn Tuer offen geht:
- Sofort alle Achsen stoppen (Emergency Stop)
- Auto-Sequenz abbrechen falls aktiv
- Ruettelmotor aus, Ampel rot
- `door_interlock_active = true` → rotes Banner auf allen UI-Seiten
- Auto-Reset wenn Tuer wieder geschlossen wird

**Dateien:**
- `mod.rs`: `check_door_interlock()` Methode, `door_interlock_active` Feld
- `act.rs`: Interlock-Check nach Driver-Alarm-Check (zweithöchste Prioritaet)
- `api.rs`: `door_interlock_active` in StateEvent
- Frontend: Rotes Banner "TUER OFFEN - NOTFALL-STOPP AKTIV" auf Auto/Test/Motors-Seiten

#### 2. 19-Cycle Auto-Sequence State Machine

Komplette Befuell-Sequenz aus Arduino v3.2 als Rust State Machine:

**Positionen (mm):**
- MT: Start=5, Run=34.5, Advance=-10/Zyklus
- Schieber: Start=7, Target=51, Wobble=±1.5
- Druecker: Start=60, Target=105

**Geschwindigkeitspresets:**
| Preset | MT | Schieber | Druecker | Buerste |
|--------|-----|----------|----------|---------|
| Slow   | 30  | 40       | 40       | 30 RPM  |
| Medium | 60  | 80       | 80       | 50 RPM  |
| Fast   | 100 | 150      | 150      | 70 RPM  |

**Zyklusablauf (AutoCycleStep enum):**
1. WobbleOut → Schieber +1.5mm (Filter loesen)
2. WobbleBack → Schieber -1.5mm
3. SchieberToTarget → 51mm (Filter fallen ins Magazin)
4. DrueckerToTarget → 105mm (haengende Filter nachruecken)
5. ParallelReturn → Druecker + Schieber zurueck + MT -10mm (parallel)
6. WaitParallelComplete → Warten bis alle 3 fertig

**Hierarchie:** Set → Block (3 pro Set) → Zyklus (19 pro Block)

**State Machine Steuerung:**
- `update_auto_sequence()` in act()-Loop, prueft `axis_position_mode` fuer Move-Completion
- `advance_auto_sequence()` zaehlt Zyklen/Bloecke/Sets hoch
- `start_auto_sequence(preset, sets)` mit Safety-Checks (Tuer, Alarm)
- `stop_auto_sequence()` stoppt alles sofort

**Dateien Backend (9 Aenderungen, +513 Zeilen):**
- `mod.rs`: speed_presets, auto_positions, AutoCycleStep, AutoSequenceState, 6 neue Methoden
- `api.rs`: StateEvent +6 Felder, Mutation +2 Varianten (StartAutoSequence, StopAutoSequence)
- `act.rs`: door_interlock + auto_sequence Updates in act()-Loop
- `new.rs`: Feld-Initialisierung

**Dateien Frontend (5 Aenderungen):**
- `bbmAutomatikV2Namespace.ts`: Schema +6 Felder
- `useBbmAutomatikV2.ts`: startAutoSequence/stopAutoSequence Mutations, isDoorInterlockActive/isAutoRunning Helpers
- `BbmAutomatikV2AutoPage.tsx`: Echte Mutations statt Console.log, Live-Fortschritt (Set/Block/Zyklus), Interlock-Banner
- `BbmAutomatikV2TestPage.tsx`: Buttons verdrahtet (1x/5x/Magazin → StartAutoSequence, Reset → Stop+Home)
- `BbmAutomatikV2MotorsPage.tsx`: Interlock-Banner hinzugefuegt

### Verifikation

- [x] `cargo check` auf Mini-PC (NixOS, nix develop) → fehlerfrei
- [x] `npx tsc --noEmit` → fehlerfrei
- [x] `npx prettier --check` → formatiert
- [x] `nixos-rebuild switch` → deployed, Server active
- [x] EtherCAT: OP state, alle Subdevices initialisiert
- [ ] Hardware-Test ausstehend (Achsen fahren, Tuer-Interlock, Sequenz-Ablauf)

### Offene Punkte

1. **Buerste:** Wird in Auto-Sequenz noch nicht gestartet (nur Ruettler). Klaeren ob Buerste mitlaufen soll
2. **Test-Seite:** "1x befuellen" und "1 Magazin" machen aktuell das gleiche (1 Set). Fuer Einzel-Zyklus-Test braeuchte es eigene Mutation
3. **Pause/Resume:** Aktuell nur Start/Stop, kein Pausieren moeglich
4. **Hardware-Test:** Sequenz auf echter Maschine validieren (Positionen, Timing, Wobble-Effekt)
