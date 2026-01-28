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
