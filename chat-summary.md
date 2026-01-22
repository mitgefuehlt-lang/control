# Chat Summary (Session)

## Ziel und Leitlinien
- Wunsch des Users: Repo komplett verstehen, keine "random" Aenderungen; Docs strikt beachten (insb. `docs/developer-docs/adding-a-machine.md`).
- Neuer, dauerhafter Wissensspeicher: `ki-doku.md` im Repo-Root.
- Doku auf Deutsch, auf Root-Ebene.
- Priorisierung explizit abgelehnt: alle Bereiche gleich wichtig.

## Repo-Analyse (lokal)
- Vollanalyse der Rust-Workspace-Struktur und Module (control-core, ethercat-hal, server, machines, units, utils, derive-crates, electron, nixos, docs).
- Tiefanalyse von `machines/` und `ethercat-hal/` inkl. Devices, PDOs, CoE, WAGO/IP20, Serial, Stepper, Analoge IO.
- Auswertung der Diagramme in `docs/drawio/*.drawio`.
- Docs-Ordner vollstaendig gelesen (alle .md, drawio), PDF/Images nur inventarisiert.
- Neues Dokument `ki-doku.md` erstellt und fortlaufend aktualisiert.

## `ki-doku.md` Inhalt (Auszug)
- Architektur, Module, Echtzeit-Loop, EtherCAT-Setup, REST/SocketIO, UI, NixOS, Scripts.
- Maschinen-Details, Identifikation, Registry, Serial-Devices.
- Drawio-Zusammenfassungen.
- Binaer-Inventar (PDF/PNG/JPG/JPEG).
- Verbindlichkeit der Docs festgehalten.
- CI/CD/Deploy Workflows dokumentiert.

## GitHub Actions (gelesen)
- `deploy.yml`: manueller NixOS-Rebuild via SSH + Tailscale.
- `fast-deploy.yml`: Push auf `master` baut Server + Electron, scp nach `/var/lib/qitech`, patchelf via nix-shell, Service restart.
- `nix.yml`, `rust.yml`, `electron.yml` (Auffaellig: electron.yml nur auf `main`, rest auf `master`).
- Doku (`docs/developer-docs/getting-started.md`) spricht von `master` (rebase/push/merge). Kein `main` in Docs.

## Mini-PC Check (NixOS)
- SSH auf Mini-PC (IP initial 192.168.178.106), NixOS 26.05 (Yarara).
- `qitech-control-server` aktiv, EtherCAT OP state; SchneideMaschineV1 initialisiert.
- `QITECH_OS=true`, aber `QITECH_OS_GIT_*` leer.
- Deploy-Stand in `/var/lib/qitech`: `server` (qitech-service), Electron Assets (konrad).
- Ursache: Fast-Deploy setzt Ownership so.
- Zeitzone: Mini-PC laeuft in UTC; Deploy-Zeiten in UTC mit CET abgeglichen.

## Doku-Checkliste (NixOS)
- `nixos-install.sh` ist Standard (setzt Git-Info Env Vars, rebuild + reboot).
- Autologin/HMI via `qitech` User (Home Manager), GNOME Kiosk.
- Service via systemd; Logs via journald.
- Update-Flow ueber `nix flake lock` + `nixos-rebuild`.

## Neuaufsetzen (Entscheidung)
- User will clean reinstall, nur `qitech`, keine Daten behalten.
- Repo von `qitechgmbh/control` nach `/tmp/control` geklont.
- `nixos-install.sh` geprueft (sammelt Git-Info, nixos-rebuild boot, reboot).
- Build lief, reboot erfolgte.
- Danach IP geaendert: `192.168.178.100` (wlo1).
- SSH nicht erreichbar, da `openssh` nicht aktiviert (Doku erwaehnt SSH nicht).
- Empfehlung: `services.openssh.enable = true;` in `nixos/os/configuration.nix` und rebuild.

## Benutzerstatus (vor Clean Reinstall)
- Benutzer: `konrad`, `qitech` (Kailar HMI), `qitech-service`.
- `realtime` Gruppe enthielt `konrad`, `qitech`, `qitech-service`.
- Nach Reinstall sollte `konrad` nicht mehr existieren (noch nicht bestaetigt mangels SSH).

## Fork + Clean Repo lokal
- Fork erstellt: `https://github.com/mitgefuehlt-lang/control.git`.
- Neuer lokaler Clone: `C:\Users\Admin\Desktop\Rust_Beckhoff_Projekt\Qitech_Github\control-master-clean`.
- `ki-doku.md` wurde in den neuen Clone kopiert.

## Offene Punkte / To-Do
- SSH auf Mini-PC wieder aktivieren (OpenSSH in NixOS config aktivieren, rebuild).
- Abgleich Autologin/HMI-User (`qitech`) nach Reinstall.
- Entscheidung: Electron workflow Branch (`main` vs `master`) angleichen.
- Falls Fast-Deploy weiter genutzt: Doku-Notiz als "Dev-Modus".
- QITECH_OS_GIT_* auf NixOS neu verifizieren (sollte nach Reinstall gesetzt sein).
- Mini-PC Reinstall-Status pruefen: Nach `nixos-install.sh` vom Fork (origin auf `mitgefuehlt-lang/control` gesetzt) ist SSH auf `192.168.178.106` verweigert, `192.168.178.100` Timeout. IP/SSH-Status am Geraet pruefen, ob Rebuild/Reboot fertig.

## Wichtige Pfade/Dateien
- `ki-doku.md` (Root)
- `.github/workflows/*.yml`
- `docs/nixos/README.md`, `docs/nixos/details.md`, `docs/nixos/quick-start.md`
- `docs/developer-docs/getting-started.md`
- `nixos/os/configuration.nix`
- `nixos-install.sh`

## Hinweis zu Chat-History
- Chat-History kann nicht exportiert werden; diese Datei ist der Ersatz.
