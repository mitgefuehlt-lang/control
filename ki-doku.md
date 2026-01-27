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
