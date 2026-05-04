# QiTech Control - Claude Code Projektregeln

## Kontext laden

Bei jeder neuen Conversation zu diesem Projekt IMMER zuerst diese Dateien lesen:
1. **PFLICHT:** `Kailar-Doku/ki-doku.md` - gesamter Projektverlauf, Hardware-Konfiguration, Architektur, bekannte Bugs, Lessons Learned
2. **PFLICHT:** `Kailar-Doku/learnings1.md` - Deep-Analysis der Codebase, kritische Bugs, Sicherheitsluecken, fehlende Features, EtherCAT-Bedenken
3. **Bei Hardware-Fragen:** `Kailar-Doku/bbm-automatik-v2-aufbau.md` lesen

## Projekt-Ueberblick

- **Zweck:** Industriemaschinensteuerung ueber EtherCAT (Beckhoff-Klemmen)
- **Stack:** Rust Backend (EtherCAT, Axum REST, SocketIO) + Electron/React Frontend (TypeScript, Zustand, Zod)
- **Deploy:** NixOS auf Mini-PC, GitHub Actions CI/CD ueber Tailscale SSH
- **Maschine aktuell in Arbeit:** BBM Automatik V2 (Blockbefuellmaschine)

## 5-Stufen Validierungs-Pipeline (PFLICHT vor jedem Deploy)

**WICHTIG:** Vor JEDEM `git commit` + Deploy muessen die relevanten Validierungsstufen durchlaufen werden. Fehler in der Vergangenheit (uncommitted Backend-Dateien, Index-Mismatches, Resource Leaks) haben zu stundenlangem Debugging gefuehrt.

### Stufe 1: Vollstaendigkeit ("Sind alle Dateien dabei?")
Vor `git add`: Agent pruefen lassen ob ALLE zusammengehoerenden Dateien staged sind.
- Rust mod.rs/api.rs geaendert → TypeScript Hook + Zod Schema MUSS auch dabei sein
- Array-Groessen geaendert → ALLE Dateien die diese Arrays referenzieren muessen angepasst sein
- Neue Mutation in Rust → Mutation Schema + Funktion in TypeScript MUSS existieren

### Stufe 2: Konsistenz ("Stimmen Rust und TypeScript ueberein?")
Agent vergleicht Feld-fuer-Feld:
- Rust `StateEvent` Struct ↔ TypeScript `stateEventDataSchema` Zod Schema
- Rust `outputs::*` Konstanten ↔ TypeScript `OUTPUT.*` Konstanten
- Rust `Mutation` Enum Varianten ↔ TypeScript Mutation Schemas
- Array-Groessen muessen exakt uebereinstimmen

### Stufe 3: Architektur-Sicherheit ("Keine Resource Leaks?")
Agent prueft geaenderte Dateien auf:
- `Box::leak` / `thread::spawn` in Retry-Pfaden (VERBOTEN - erzeugt Leaks)
- Channel-Sender in SharedState registriert → Receiver muss dauerhaft leben
- Retry-Loops die Machines/Channels erzeugen → MUSS process-exit + systemd-restart nutzen
- `std::process::exit()` → dnsmasq vorher starten

### Stufe 4: Post-Commit Check ("Ist der Commit vollstaendig?")
Nach `git commit`: `git diff HEAD~1 --name-only` pruefen
- Fuer jede Rust-Datei: zugehoerige TS-Datei im selben Commit?
- Keine vergessenen unstaged Dateien die zum Change gehoeren?

### Stufe 5: Post-Deploy Verifikation ("Laeuft alles?")
Nach Deploy via SSH pruefen:
```bash
ssh qitech@nixos "sudo journalctl -u qitech-control-server --no-pager -n 30"
```
Erfolgskriterien:
- "Group in OP state" (EtherCAT OK)
- "received machines[...]" (Machine registriert)
- KEINE "closed channel", "error", "panic" Meldungen
- "subscribing namespace" wenn UI verbunden

## Hardware-Zuordnung BBM Automatik V2 (Stand 2026-03-19)

### Digitale Ausgaenge (DO)
| Index | Konstante | Pin | Geraet |
|-------|-----------|-----|--------|
| 0 | AMPEL_GRUEN | DO1 | Ampel Gruen |
| 1 | AMPEL_GELB | DO2 | Ampel Gelb |
| 2 | AMPEL_ROT | DO3 | Ampel Rot |
| 3 | BUERSTENMOTOR | DO4 | Buerstenmotor |
| 4 | RUETTELMOTOR | DO5 | Ruettelmotor |
| 5 | PNEUMATIK | DO6 | Pneumatik Ventil |
| 6 | LUEFTER | DO7 | Schaltschrank-Luefter |

### Digitale Eingaenge (DI)
| Index | Konstante | Pin | Geraet |
|-------|-----------|-----|--------|
| 0 | ALARM_MT | DI1 | CL75t Alarm Transporter |
| 1 | ALARM_SCHIEBER | DI2 | CL75t Alarm Schieber |
| 2 | ALARM_DRUECKER | DI3 | CL75t Alarm Druecker |
| 3 | REF_MT | DI4 | Referenzschalter MT (NC) |
| 4 | REF_SCHIEBER | DI5 | Referenzschalter Schieber (NC) |
| 5 | REF_DRUECKER | DI6 | Referenzschalter Druecker (NC) |
| 6 | TUER | DI7 | Tuersensor |

### Achsen (3x PTO via EL2522)
| Index | Achse | EL2522 | Kanal |
|-------|-------|--------|-------|
| 0 | MT (Magazin Transporter) | #1 | CH1 |
| 1 | Schieber | #1 | CH2 |
| 2 | Druecker | #2 | CH1 |
| - | (EL2522 #2 CH2 unused) | #2 | CH2 |

## Wichtige Dateipfade

### Rust Backend (machines/src/bbm_automatik_v2/)
- `mod.rs` - Struct, Output/Input/Axis Konstanten, Helper-Funktionen
- `api.rs` - StateEvent, LiveValuesEvent, Mutation Enum, MachineApi
- `act.rs` - Control Loop (act() wird jeden Zyklus aufgerufen)
- `new.rs` - Hardware-Initialisierung (EL2522 CoE Config)

### TypeScript Frontend (electron/src/machines/bbm/bbm_automatik_v2/)
- `bbmAutomatikV2Namespace.ts` - Zod Schemas fuer State/LiveValues Events
- `useBbmAutomatikV2.ts` - React Hook mit OUTPUT/INPUT Konstanten, Mutations
- `BbmAutomatikV2MotorsPage.tsx` - Motoren-Seite UI
- `BbmAutomatikV2ActuatorsPage.tsx` - Aktoren-Seite UI

### Server
- `server/src/main.rs` - EtherCAT Discovery + Setup Aufruf
- `server/src/ethercat/setup.rs` - EtherCAT Init (setup_loop)

## Bekannte Patterns / Regeln

### EtherCAT Init Timeout nach Reboot
- Server beendet sich mit `exit(1)`, systemd (`Restart=always`) startet neu
- KEIN Retry-Loop im Code (erzeugt doppelte Machines/Channels)

### Deploy-Workflow
```bash
gh workflow run fast-deploy --ref master
```
Macht: `git pull` → `nixos-rebuild boot` → `reboot` → Service-Check

### SSH-Zugang
```bash
ssh qitech@nixos
```

### Server-Neustart
```bash
ssh qitech@nixos "sudo systemctl restart qitech-control-server"
```

### Logs pruefen
```bash
ssh qitech@nixos "sudo journalctl -u qitech-control-server --no-pager -n 30"
```
