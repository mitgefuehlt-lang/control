# Learnings 1: Deep-Analysis control-master-clean

**Datum:** 2026-02-11
**Scope:** Vollstaendige Codebase-Analyse (Rust Backend, Electron Frontend, EtherCAT HAL, Doku)

---

## 1. Architektur-Ueberblick

### Kommunikationsfluss
```
Electron (React/Zustand) --REST--> Axum Server --Channel--> Machine act() Loop
                         <--SocketIO-- Namespace emit (StateEvent, LiveValuesEvent)
```

### Alle Maschinen im System
| Maschine | Status | Hardware | Bemerkung |
|----------|--------|----------|-----------|
| BbmAutomatikV2 | ~70% | 2x EL2522, EL1008, EL2008 | Homing, Alarm, Soft Limits fertig; Auto-Sequenz fehlt |
| Winder2 | ~80% | 3x EL70x1, TensionArm, DI/DO | Tension/Traverse Control funktional |
| Extruder1/2/3 | ~75% | Mitsubishi CS80, EL3xxx, DI/DO | Temperatur/Druck-Regelung; Pumpen-Interlocks fehlen |
| Laser | ~90% | USB Serial | Durchmesser-Messung funktional |
| Buffer1 | ~60% | Diverse | Basis-Steuerung; adaptive Buffer-Logik fehlt |
| AquaPath1 | ~50% | Diverse | Initialisierung vorhanden, Steuerlogik minimal |
| SchneidemaschineV0 | ~40% | EL2522, EL1008, EL2008 | Motor-Steuerung basic; kein Position-Holding |
| WagoPower | ~30% | WAGO Module | Initiale Unterstuetzung |
| TestMachines | 100% | Diverse | Beispiel-Implementierungen |

---

## 1.5 Hardware-Fallen (Stand 2026-05-27)

### EL2522 Travel Distance Control + u32 Counter Wraparound
**Symptom:** Negative Zielposition wird als positive Riesenzahl interpretiert; Motor faehrt in falsche Richtung oder kehrt um sobald Counter unter 0 wandert.

**Setup:** EL2522 in `PulseDirectionSpecification` + `travel_distance_control: true`. Per PDO setzt der Host `target_counter_value: u32` und `go_counter: bool`. Die Hardware vergleicht `target vs counter` UNSIGNED, um die Richtung zu bestimmen. `frequency_value` wird als POSITIVE Magnitude interpretiert; das Vorzeichen wird ignoriert.

**Beweis aus Logs (BbmAutomatikV2):**
- `target=-120 pulses` (signed) -> as u32 = 4_294_967_176
- `current=80, freq=-200Hz` commanded
- Position steigt: 146 -> 346 -> 546 -> 746 (Motor faehrt POSITIV, Vorzeichen ignoriert)

**Loesung in `machines/src/bbm_automatik_v2/`:**
- TDC nur fuer Moves nutzen, wo `target >= 0 && current >= 0`.
- Sonst Speed-Mode (`go_counter=false`, signed `frequency_value`) + Software-Stop via `wrapping_sub` Delta-Tracking. Vorzeichen-Compare auf u32-Subtraktion liefert i32 mit korrektem Vorzeichen, solange `|delta| < 2^31` (in der Praxis immer erfuellt bei realistischen Bewegungen).
- Hardware-Homing-Pfad nutzt seit jeher `frequency_value=-X` im Speed-Mode (`go_counter=false`) und das funktioniert -> Speed-Mode mit negativem freq ist OK.

**Generalisierbar fuer andere Maschinen?** Vermutlich ja. Andere Beckhoff-PTO-Module (EL2521) duerften aehnlich liegen. Wenn jemand spaeter eine Maschine schreibt die negative Absolutpositionen braucht: gleiche Logik anwenden.

### Soft-Limits: kein MIN, nur MAX
Auf Wunsch des Users (Kalibrierung, Lower-End-Tests) wird in `BbmAutomatikV2` kein `MIN_MM=0` mehr enforced. Nur die MAX-Grenze pro Achse (MT=230, Schieber=53, Druecker=107) wird gehalten - und auch nur wenn `axis_homed && idle`. Vor Homing keine Limits.

---

## 2. Kritische Bugs

### Bug 1: `unsafe static mut` Race Condition
**Datei:** `machines/src/bbm_automatik_v2/act.rs`, Zeile 33-43
```rust
static mut LAST_DEBUG: Option<Instant> = None;
let should_log = unsafe { ... };
```
**Problem:** Mutable static ohne Synchronisation = undefiniertes Verhalten bei mehreren Threads. Wird im Hot-Loop (~1kHz) aufgerufen.
**Fix:** `std::sync::OnceLock<Instant>` oder `AtomicU64` verwenden.
**Schwere:** HOCH

### Bug 2: `expect()` bei Serialisierung = Panic
**Dateien:** Alle `act.rs` Dateien (bbm_automatik_v2, extruder1, laser, etc.)
```rust
state: serde_json::to_value(self.get_state())
    .expect("Failed to serialize state"),  // PANIC bei Fehler
```
**Problem:** Wenn StateEvent-Struct inkompatibel geaendert wird, crasht die gesamte Maschine.
**Fix:** `unwrap_or_default()` oder explizites Error-Handling.
**Schwere:** MITTEL (selten, aber nicht wiederherstellbar)

### Bug 3: Laser `UnsubscribeNamespace` Panic
**Datei:** `machines/src/laser/act.rs`
```rust
MachineMessage::UnsubscribeNamespace => match &mut self.namespace.namespace {
    Some(namespace) => { /* ... */ },
    None => todo!(),  // PANIC bei doppeltem Unsubscribe
}
```
**Fix:** `None => {}` statt `todo!()`
**Schwere:** MITTEL

### Bug 4: `todo!()` in Machine Cross-Connection
**Datei:** `machines/src/lib.rs`, Zeile 538-542
```rust
MachineMessage::ConnectToMachine(_) => { todo!(); }
MachineMessage::DisconnectMachine(_) => { todo!(); }
```
**Problem:** Feature definiert aber nicht implementiert. Panic wenn Message gesendet wird.
**Schwere:** MITTEL (Feature aktuell nicht genutzt)

---

## 3. Sicherheitsluecken (Safety)

### 3.1 Kein EtherCAT Link-Loss Watchdog
**Problem:** Wenn die EtherCAT-Verbindung unterbrochen wird:
- Kein Watchdog erkennt den Ausfall
- State-Updates laufen mit veralteten Daten weiter
- Benutzer sieht letzte bekannte Werte unbegrenzt
- Motoren laufen potentiell weiter (EL2522 Watchdog deaktiviert!)

**Empfehlung:**
- Heartbeat/Watchdog im Control-Loop implementieren
- Bei Link-Loss: sofort alle Motoren stoppen, UI-Warnung anzeigen
- EL2522 Watchdog-Timer evaluieren (aktuell `watchdog_timer_deactive: true`)

### 3.2 Soft Limits basieren nur auf Pulse-Counting
**Problem:** Position-Tracking nutzt `get_position()` (Pulse-Counter im EL2522). Bei Step-Loss wird die Position falsch und Soft Limits greifen nicht mehr.

**Aktueller Stand:** System loggt "STEP LOSS DETECTED" Warnung (mod.rs Zeile 475) aber laeuft weiter.

**Empfehlung:**
- Periodisch Position gegen Referenzschalter validieren
- Bei Step-Loss > Schwellwert: automatische Re-Referenzierung erzwingen
- Step-Loss-Counter im UI anzeigen

### 3.3 Alarm-Persistenz ohne Hardware-Bestaetigung
**Problem:** `reset_alarms()` setzt `axis_alarm_active` zurueck, prueft aber nicht ob der physische Alarm (CL75t AL-Pin) noch aktiv ist. Benutzer koennte Reset druecken waehrend der Treiber noch im Fehler ist.

**Empfehlung:** Beim Reset pruefen ob der Alarm-Pin tatsaechlich inaktiv ist:
```rust
pub fn reset_alarms(&mut self) {
    // Erst pruefen ob physische Alarme noch aktiv
    for &(axis, input_idx) in &alarm_inputs {
        let raw = self.digital_inputs[input_idx].get_value().unwrap_or(!ALARM_ACTIVE_LOW);
        let still_alarm = if ALARM_ACTIVE_LOW { !raw } else { raw };
        if still_alarm {
            tracing::warn!("Cannot reset - Axis {} alarm still active!", axis);
            return;
        }
    }
    self.axis_alarm_active = [false; 4];
}
```

### 3.4 SDO-Write Fehler werden ignoriert
**Datei:** `machines/src/bbm_automatik_v2/mod.rs`, Zeile 366
```rust
sdo_write(subdevice_index, pto_base, 0x14, rising_ms);  // Kein Fehler-Return
```
**Problem:** Wenn der SDO-Write fehlschlaegt, wird die Beschleunigung nicht gesetzt, aber das System denkt sie waere aktiv. Motor koennte mit falscher Rampe fahren.

### 3.5 Keine API-Authentifizierung
**Problem:** REST-API und SocketIO-Namespaces haben keine Authentifizierung. Jeder Client im Netzwerk kann Maschinen steuern.
**Aktueller Schutz:** Netzwerk-Isolation (nur lokales EtherCAT-Netz)
**Empfehlung:** API-Key Header wenn jemals in nicht-vertrauenswuerdigem Netz deployed wird.

---

## 4. Fehlende Features

### 4.1 BbmAutomatikV2: Automatik-Sequenz (KRITISCH)
**Datei:** `BbmAutomatikV2AutoPage.tsx`, Zeile 36-47
```typescript
const handleStart = () => {
    console.log("Automatik Start", { speedPreset, magazinSets });
    // NUR LOGGING - keine echte Implementierung!
};
```
Die gesamte Auto-Seite ist ein UI-Stub. Im Arduino-Code (v3.2) ist die komplette Zykluslogik implementiert. Backend hat keinerlei Sequenz-Steuerung.

### 4.2 Maschinen-Querverbindung (Machine-to-Machine)
`MachineMessage::ConnectToMachine` / `DisconnectMachine` sind definiert aber `todo!()`. Verhindert z.B. Winder->Buffer Workflows.

### 4.3 Homing fuer andere Maschinen
Nur BbmAutomatikV2 hat eine Homing-Statemachine. Winder2, SchneidemaschineV0 haben keine Referenzfahrt.

### 4.4 Temperatur/Druck-Grenzwerte (Extruder)
UI zeigt Limits an, aber Backend erzwingt sie nicht. Keine automatische Abschaltung bei Ueberschreitung.

### 4.5 Cycle-Time Monitoring
Kein Monitoring der Control-Loop Zykluszeit. Jitter/Overruns werden nicht erkannt oder geloggt.

---

## 5. Code-Qualitaet

### 5.1 Inkonsistenzen zwischen Maschinen
| Aspekt | BbmAutomatikV2 | Extruder | Laser |
|--------|-----------------|----------|-------|
| Error Handling | Result<(), Error> | Result<(), Error> + intern logging | Result<(), Error> |
| Emission Rate | 30 FPS | 30 FPS | Kommentar sagt 60 FPS, Code 30 FPS |
| Homing | Statemachine | Nein | N/A |
| Soft Limits | Ja | Nein | N/A |
| Alarm Detection | Ja (neu) | Mitsubishi Status-Bits (passiv) | Nein |

### 5.2 SocketIO Queue-Fehler werden verschluckt
**Datei:** `control-core/src/socketio/namespace.rs`
```rust
match self.socket_queue_tx.try_send(...) {
    Ok(_) => { /* trace */ },
    Err(e) => { /* STILLE - kein Handler */ }
}
```
Frontend bekommt kritische Events nicht mit. Kein Hinweis dass State divergiert.

### 5.3 Unbounded Channels
Message-Queues (`smol::channel::unbounded()`) koennen bei Last unbegrenzt wachsen. Kein Backpressure-Mechanismus.

---

## 6. EtherCAT Hardware-Bedenken

### 6.1 EL2522 Watchdog deaktiviert
```rust
watchdog_timer_deactive: true,  // In allen EL2522-Configs
```
**Grund:** Noetig fuer Travel Distance Control.
**Risiko:** Bei Kommunikationsausfall laeuft Stepper unkontrolliert weiter.
**Empfehlung:** Evaluieren ob Watchdog nur fuer PTO-Modus deaktiviert werden muss, oder ob er fuer die Gesamtmaschine aktivierbar ist.

### 6.2 PDO-Mapping wird nicht validiert
Kein Runtime-Check ob die empfangenen PDO-Daten zum erwarteten Layout passen. Wenn sich Device-Firmware aendert, liest das System Muell-Daten.

### 6.3 Ramp-Parameter hartcodiert
```rust
ramp_time_constant_rising: 2500,   // ms
ramp_time_constant_falling: 2250,  // ms
base_frequency_1: 5000,            // Hz
```
Sollten konfigurierbar sein (Config-File oder UI-Parameter).

---

## 7. Dokumentations-Luecken

### Vorhanden (docs/)
- architecture-overview.md, ethercat-basics.md, coe.md, control-loop.md
- rest-api.md, pdo.md, io.md, devices.md, identification.md
- machines/ (teilweise), developer-docs/adding-a-machine.md

### Fehlend
1. Safety-Dokumentation (Notaus, Watchdog, Fehlerbehandlung)
2. EtherCAT Link-Failure Recovery Anleitung
3. Soft-Limit Berechnung und Step-Loss Erkennung
4. Homing-Prozedur Spezifikation
5. Temperatur/Druck Grenzwert-Enforcement
6. Machine-to-Machine Verbindungs-Konzept
7. Deployment/Konfigurations-Guide

---

## 8. Priorisierte Aktionsliste

### Sofort (Safety-Critical)
1. **EtherCAT Watchdog/Heartbeat** - Link-Loss erkennen, Motoren stoppen
2. **`unsafe static mut` fixen** - AtomicU64 verwenden
3. **`expect()` durch Error-Handling ersetzen** - in allen act.rs Dateien

### Kurzfristig (naechste Sprint-Iteration)
4. **Alarm-Reset mit Hardware-Validierung** - Physischen Pin pruefen vor Reset
5. **SDO-Write Fehler pruefen** - Return-Werte nicht ignorieren
6. **`todo!()` Panics entfernen** - Entweder implementieren oder graceful ignorieren
7. **Laser Emission-Rate Bug fixen** - 30 FPS oder 60 FPS, aber konsistent

### Mittelfristig
8. **BbmAutomatikV2 Auto-Sequenz** - Zykluslogik aus Arduino portieren
9. **Step-Loss Recovery** - Periodische Re-Referenzierung
10. **Cycle-Time Monitoring** - Jitter/Overruns loggen
11. **SocketIO Queue-Fehler loggen** - Nicht stillschweigend verwerfen

### Langfristig
12. **Machine-to-Machine Verbindung** - ConnectToMachine implementieren
13. **API-Authentifizierung** - Fuer Deployment ausserhalb lokales Netz
14. **Konfigurierbare Parameter** - Soft Limits, Ramp-Zeiten, Homing-Geschwindigkeit
15. **Extruder Grenzwert-Enforcement** - Automatische Abschaltung

---

## 9. Vergleich Arduino v3.2 vs Rust Backend

| Feature | Arduino v3.2 | Rust Backend | Status |
|---------|-------------|--------------|--------|
| Motor JOG | Ja | Ja | OK |
| Position Control | Ja | Ja (Travel Distance) | OK |
| Homing | Ja | Ja (3-Phasen Statemachine) | OK |
| Soft Limits | Ja | Ja (aus Arduino uebernommen) | OK |
| Driver Alarm | `checkDriverAlarms()` | `check_driver_alarms()` | OK (neu) |
| Emergency Stop | `emergencyStopAll()` | `stop_all_axes()` | OK |
| Auto-Sequenz | Vollstaendig | Nur UI-Stub | FEHLT |
| Ruettelmotor-Logik | Automatisch in Zyklus | Manuell ein/aus | FEHLT |
| Ampel-Statemachine | Automatisch | Manuell | FEHLT |
| Speed Presets | Definiert | UI vorhanden, Backend fehlt | TEILWEISE |

---

*Dieses Dokument wurde durch automatisierte Deep-Analysis der gesamten control-master-clean Codebase erstellt.*

---

## 10. Neue Learnings (Maerz 2026)

### Learning 1: setup_loop darf NICHT in einem Retry-Loop aufgerufen werden

**Datum:** 2026-03-19
**Schwere:** KRITISCH

**Problem:** `setup_loop()` in `server/src/ethercat/setup.rs` registriert Machines und API-Channels in SharedState (`api_machines.insert()`, `AddMachines` Message) BEVOR die EtherCAT State-Transition (Safe-OP/OP) stattfindet. Wenn die Transition fehlschlaegt:
- Machine-Objekt ist bereits im RT-Loop (mit kaputter EtherCAT-Referenz)
- API-Channel-Sender ist in SharedState registriert
- TX/RX Thread laeuft (Box::leaked PduStorage, dedizierter Thread)

Bei erneutem Aufruf von `setup_loop()`:
- Neues Machine-Objekt mit neuem Channel wird erstellt
- Alter Channel-Sender in api_machines wird ueberschrieben
- Altes Machine-Objekt im RT-Loop hat jetzt verwaisten Receiver
- Ergebnis: `"failed sending into a closed channel"` → UI bekommt keinen State → alle Buttons ausgegraut

**Loesung:** Bei EtherCAT-Init-Fehler → `std::process::exit(1)`. Systemd (`Restart=always`) startet den Prozess sauber neu. Kein verwaister State moeglich.

**Betroffene Datei:** `server/src/main.rs` (`start_interface_discovery`)

### Learning 2: Rust↔TypeScript Konsistenz ist die haeufigste Fehlerquelle

**Datum:** 2026-03-19
**Schwere:** KRITISCH

**Problem:** Wenn Rust-Structs und TypeScript-Zod-Schemas nicht uebereinstimmen, passieren stille Fehler:
- **Array-Groesse falsch:** Zod-Validierung schlaegt fehl → `state` ist null → `isDisabled = true` → alle Buttons grau, keine Fehlermeldung sichtbar
- **Mutation fehlt:** Server gibt JSON-Parse-Error → Request schlaegt fehl → Button "reagiert nicht"
- **Output-Index falsch:** Falscher physischer Ausgang wird geschaltet → Pneumatik startet Motor

**Betroffene Dateien-Paare (muessen IMMER synchron geaendert werden):**

| Rust-Datei | TypeScript-Datei | Was muss uebereinstimmen |
|------------|-----------------|--------------------------|
| `api.rs` → `StateEvent` | `bbm*Namespace.ts` → `stateEventDataSchema` | Alle Felder, Array-Groessen |
| `api.rs` → `LiveValuesEvent` | `bbm*Namespace.ts` → `liveValuesEventDataSchema` | Alle Felder, Array-Groessen |
| `api.rs` → `Mutation` enum | `useBbm*.ts` → Mutation Schemas | Jede Variante |
| `mod.rs` → `outputs::*` | `useBbm*.ts` → `OUTPUT.*` | Exakte Index-Werte |
| `mod.rs` → `inputs::*` | `useBbm*.ts` → `INPUT.*` | Exakte Index-Werte |
| `mod.rs` → `axes::*` | `useBbm*.ts` → `AXIS.*` | Exakte Index-Werte |

### Learning 3: Hardware-Pin-Zuordnung immer von der physischen Verdrahtung ableiten

**Datum:** 2026-03-19
**Schwere:** HOCH

**Problem:** Die Software-Konstanten fuer DO/DI-Indices muessen die PHYSISCHE Verdrahtung widerspiegeln, nicht eine logische Ordnung. Beispiele:
- Ampel war als Rot-Gelb-Gruen definiert (logisch), aber physisch Gruen-Gelb-Rot verkabelt (DO1=Gruen, DO2=Gelb, DO3=Rot)
- Buerstenmotor war nicht als DO definiert (war PTO-Achse), wurde aber physisch auf DO4 umverdrahtet

**Regel:** Bei JEDER Hardware-Aenderung:
1. Physische Verdrahtung dokumentieren (welcher Draht an welcher Klemme)
2. Software-Konstanten in Rust UND TypeScript anpassen
3. Beides im selben Commit

### Learning 4: Uncommitted Changes sind die gefaehrlichste Art von Technical Debt

**Datum:** 2026-03-19
**Schwere:** KRITISCH

**Problem:** 6 Dateien mit kritischen Backend-Aenderungen (Achsen-Reduktion 4→3, neue Output-Indices, neue Mutation) waren lokal geaendert aber nie committed. Ein spaeterer UI-only Commit wurde deployed, was zu inkompatiblem Frontend/Backend fuehrte.

**Symptome:**
- UI zeigt Buttons die Backend nicht kennt → Button reagiert nicht
- UI sendet falsche Indices → falscher physischer Ausgang wird geschaltet
- Zod-Schema passt nicht zu Server-Daten → State wird nie empfangen → alles disabled

**Regel:** Vor JEDEM Commit: `git status` pruefen. Wenn unstaged Dateien existieren die zum gleichen Feature gehoeren → ALLE zusammen committen oder bewusst entscheiden sie auszulassen.

### Learning 5: EtherCAT-Init ist nicht-deterministisch nach Reboot

**Datum:** 2026-03-19
**Schwere:** MITTEL

**Problem:** Nach `nixos-rebuild boot` + Reboot schlaegt die EtherCAT-Initialisierung in `setup_loop` intermittierend fehl mit "Timeout" bei `init_single_group` oder `into_safe_op`. Beim naechsten Versuch (manueller Service-Restart) funktioniert es fast immer.

**Vermutete Ursache:** Race Condition beim Boot - EtherCAT-Hardware (EK1100 + Klemmen) braucht Zeit zum Hochfahren, aber der Server startet sofort nach dem Systemd-Target.

**Aktuelle Loesung:** `std::process::exit(1)` bei Fehler, systemd startet automatisch neu.

**Moegliche bessere Loesung (noch nicht implementiert):**
- systemd `After=network-online.target` + `ExecStartPre` mit kurzer Wartezeit
- Oder: `setup_loop` intern nur die State-Transition (Safe-OP/OP) retrien, OHNE Machines/Channels neu zu erstellen

### Learning 6: Bug 1 (unsafe static mut) wurde bereits gefixt

**Datum:** 2026-03-19
**Update zu Abschnitt 2, Bug 1**

Der `unsafe static mut LAST_DEBUG` in `act.rs` wurde durch ein normales `last_debug_log: Option<Instant>` Feld im `BbmAutomatikV2` Struct ersetzt. Kein `unsafe` mehr noetig.

### Learning 7: Bug 2 (expect bei Serialisierung) wurde bereits gefixt

**Datum:** 2026-03-19
**Update zu Abschnitt 2, Bug 2**

Die `expect()` Aufrufe bei `serde_json::to_value()` in `act.rs` wurden durch `unwrap_or_else` mit Error-Logging und `serde_json::Value::Null` Fallback ersetzt.

---

## 11. Aktualisierter Status (Maerz 2026)

### Behobene Bugs aus Abschnitt 2
| Bug | Status | Commit/Zeitraum |
|-----|--------|-----------------|
| Bug 1: unsafe static mut | GEFIXT | Maerz 2026 |
| Bug 2: expect() bei Serialisierung | GEFIXT | Maerz 2026 |
| Bug 3: Laser UnsubscribeNamespace Panic | OFFEN | - |
| Bug 4: todo!() in Machine Cross-Connection | OFFEN | - |

### Aktualisierter Arduino v3.2 Vergleich
| Feature | Arduino v3.2 | Rust Backend | Status |
|---------|-------------|--------------|--------|
| Motor JOG | Ja | Ja | OK |
| Position Control | Ja | Ja (Travel Distance) | OK |
| Homing | Ja | Ja (3-Phasen Statemachine) | OK |
| Soft Limits | Ja | Ja | OK |
| Driver Alarm | `checkDriverAlarms()` | `check_driver_alarms()` | OK |
| Emergency Stop | `emergencyStopAll()` | `stop_all_axes()` + Door Interlock | OK |
| Auto-Sequenz | Vollstaendig | Backend + UI implementiert | OK (neu) |
| Ruettelmotor-Logik | Automatisch in Zyklus | Manuell + in Auto-Sequenz | OK (neu) |
| Ampel-Statemachine | Automatisch | Manuell + in Auto-Sequenz | OK (neu) |
| Speed Presets | Definiert | Backend + UI | OK (neu) |
| Buerstenmotor | PTO-Achse | Digital Output (DO4) | GEAENDERT |
| Aktoren-Tab | - | Pneumatik, Ruettelmotor, Ampel | NEU |

---

## 12. EL2522 Beckhoff-Handbuch Learnings (2026-06-10)

**Quelle:** Beckhoff "Documentation EL252x" v4.3 (PDF, 239 Seiten), https://download.beckhoff.com/download/document/io/ethercat-terminals/el252xen.pdf + Infosys-Kapitel unter https://infosys.beckhoff.com/content/1033/el252x/. Seitenangaben unten = PDF v4.3. Gegen unsere Implementierung (`ethercat-hal/src/devices/el2522.rs`, `machines/src/bbm_automatik_v2/new.rs` + `mod.rs`) abgeglichen.

### 12.1 Was VERIFIZIERT KORREKT umgesetzt ist

**PDO-Mapping (Handbuch S.191-194):** Unsere HAL nutzt exakt das Preset "2 Ch. Standard 32 Bit (MDP 253/511)":
- RxPdo: 0x1600 (PTO Control Ch1), 0x1603 (PTO Target Ch1), 0x1605 (PTO Control Ch2), 0x1608 (PTO Target Ch2), 0x160B (ENC Control Ch1), 0x160D (ENC Control Ch2) — stimmt 1:1 mit der Beckhoff-Tabelle.
- TxPdo: 0x1A00 (PTO Status Ch1), 0x1A01 (PTO Status Ch2), 0x1A03 (ENC Status Ch1), 0x1A05 (ENC Status Ch2) — stimmt.
- Hinweis: Die Bits `Automatic Direction` / `Forward` / `Backward` (0x7000:04-06) existieren NUR in den "continuous position"-PDOs (0x1601/0x1606). In unserem Preset gibt es sie nicht — Richtung kommt in TDC ausschliesslich aus dem unsigned Vergleich target vs counter (daher der Virtual Zero Offset).

**TDC-Ablauf (S.139-140, Kapitel 6.5.1.2):** Drei Phasen: Parameterization → Trip → Reset.
- Trip start (enhanced mode): PDO `Target counter value` setzen, `Go counter = TRUE`, `Frequency value != 0` (= max. Fahrfrequenz, nur Betrag!).
- **Kritische Regel:** "Go counter" muss **gleichzeitig oder VOR** dem Frequency value gesetzt werden. Sonst laeuft die Klemme im Speed-Mode los ohne TDC! Unsere Umsetzung: alle Felder im selben PDO-Frame (`move_to_position_mm` setzt go_counter+target+frequency in einem `set_output`) = "gleichzeitig" = OK.
- Reset nach Zielerreichung: `Frequency value = 0` + `Go counter = FALSE`. Unsere Umsetzung in `update_hardware_monitor` (nach `select_end_counter`) macht genau das = OK.
- Ziel erreicht → Klemme schaltet Frequenz selbst auf 0; Host erkennt das am Status-Bit `Sel. Ack/End counter` (0x6000:01). Unsere 5-Zyklen-Grace-Period (`axis_position_ignore_cycles`) ist noetig, weil das Bit vom vorherigen Move noch gesetzt sein kann — Beckhoff dokumentiert kein explizites Clear-Timing.

**Set-Counter-Handshake (S.189):** `Set counter value` (0x7020:11) schreiben + `Set counter` (0x7020:03) setzen → Klemme uebernimmt, meldet `Set counter done` (0x6020:03). Unser Auto-Clear-Pfad in `update_hardware_monitor` (set_counter zuruecknehmen sobald done) ist korrekt — solange set_counter TRUE ist, klemmt die HW den Zaehler auf den Wert fest.

**Frequenz-Codierung:** `direct_input_mode=true` + `frequency_factor=100` → 1 Digit = 100 × 10 mHz = **1 Hz** (0x8000:16, "Digit x 10mHz", S.201). Unser Code rechnet 1 Digit = 1 Hz = korrekt. Vorzeichen: Default `sign_amount_representation=false` = Zweierkomplement (0x8000:04) — deshalb funktionieren unsere negativen frequency_values im Speed-Mode (Homing) korrekt.

**Richtung im Pulse-Direction-Mode (S.186-187):** Kanal B LOW = Rechtslauf, HIGH = Linkslauf. Moduliertes Signal auf Kanal A. (Verkabelung: A=PUL, B=DIR am CL57T/CL75t.)

**Autoset threshold (0x8020:1A, S.189-190, 203):** MUSS 0 bleiben (= inaktiv, unser Default). Wenn >0: bei grosser Soll-Ist-Differenz uebernimmt die Klemme den Zielwert DIREKT in den Zaehler **ohne Pulse auszugeben** → stiller Positionssprung. Beckhoff selbst: "nur in besonderen Ausnahmefaellen verwenden". Niemals aktivieren.

### 12.2 FUND: Rampen-Regel wird von `set_axis_acceleration` verletzt

**Beckhoff-Regel (S.139-140):** Fuer praezises TDC-Anfahren muss die **fallende Rampe ~10% steiler** sein als die steigende. Grund: Die Klemme berechnet den Bremseinsatzpunkt aus der Anzahl Schritte der Beschleunigungsphase; die Bremsrampe muss etwas steiler sein, damit die `slowing down frequency` (0x8000:17, Default 50 Hz = 2,5 mm/s Schleichgang bei 20 P/mm) VOR dem Ziel erreicht wird und die Klemme nicht mit voller Fahrt in den Endpunkt laeuft.

**Stand bei uns:**
- `new.rs` CoE-Init: rising=2500, falling=2250 (= 0,9 × rising = 10% steiler) → **korrekt**.
- `mod.rs::set_axis_acceleration` (Z.684-685): `falling_ms = rising_ms` ("Same as rising to avoid step loss") → **verletzt die Regel**. Sobald der User die Beschleunigung einmal im UI aendert, sind rising==falling und die Beckhoff-Bedingung ist weg. Moeglicher Effekt: Ziel wird mit Restgeschwindigkeit erreicht / Ueberfahren statt Schleichgang.
- **TODO:** `falling_ms = (rising_ms as f32 * 0.9) as u16` (oder rising um 10% strecken). Vorsicht: der Kommentar "avoid step loss from aggressive braking" deutet auf einen frueheren Hardware-Befund hin — beim Fix Bremsverhalten am Motor verifizieren (CL75t-Treiber + Last). Hinweis: Das Handbuch ist hier selbst inkonsistent (Tabelle S.139 sagt "t3 > 1.1 × t1" mit Einheit [Δ/sec], Prosa sagt "downward ramp ~10% steeper"). Empirisch verhaelt sich die Konstante bei uns als "ms von 0 auf base_frequency_1" (2500 ms auf 5000 Hz = 2000 Hz/s = 100 mm/s² — gemessen und stimmig), also: steiler = KLEINERER Wert.

**Rampen-Formel (empirisch validiert):** `ramp_time_ms = base_frequency_1 / accel_hz_s * 1000`. Unser `set_axis_acceleration` nutzt genau das mit hardcoded `base_freq = 5000.0` — muss synchron bleiben mit `base_frequency_1: 5000` in new.rs (bei Aenderung BEIDE Stellen!).

### 12.3 Watchdog: Beckhoff-Fakten vs. unsere Config (SAFETY)

**Beckhoff-Fakten (S.20-21 Kap. 4.3, S.201 Objekte):**
- Der **SM-Watchdog** (SyncManager, Default 100 ms) wird bei jeder erfolgreichen Prozessdaten-Kommunikation zurueckgesetzt. Er loest aus, wenn laenger als die eingestellte Zeit keine Prozessdaten ankommen (z.B. Kabelbruch, Master-Absturz). Der Klemmen-State (OP) bleibt dabei unveraendert.
- Verhalten beim Ausloesen (EL2522-spezifisch): Klemme gibt den **Manufacturer's oder User's switch-on value** aus (0x8000:09 waehlt; 0x8000:11 = User-Wert, Default 0 Hz).
- Mit `emergency_ramp_active` (0x8000:02) faehrt die Klemme beim Watchdog-Ausloesen **per Rampe** (Zeitkonstante 0x8000:18) auf den switch-on value — kein harter Stopp.
- Auslieferungszustand: Watchdog **AKTIV** (0x8000:03 = false).

**NIRGENDS im Handbuch steht, dass der Watchdog fuer Travel Distance Control deaktiviert werden muss.** Die Behauptung in ki-doku ("Noetig fuer TDC") ist unbelegt — vermutlich ein Artefakt aus der Testphase 2026-01-28 ("watchdog_timer_deactive: true (fuer Tests)"). Da unser RT-Loop jeden Zyklus (~1 ms) PDOs schreibt, wird der SM-Watchdog (100 ms) im Normalbetrieb nie ausloesen — auch nicht waehrend eines laufenden TDC-Moves.

**Empfohlene Safety-Config (Phase 1 umsetzen + Hardware-Test):**
```rust
watchdog_timer_deactive: false,        // Watchdog AKTIV (Auslieferungszustand)
emergency_ramp_active: true,           // bei Ausloesung: Rampe statt harter Stopp
user_switch_on_value_on_watchdog: true,// User-Wert ausgeben
user_switch_on_value: 0,               // = 0 Hz → Motor stoppt
ramp_time_constant_emergency: 1000,    // Brems-Rampe beim Watchdog (anpassen an Mechanik)
```
**Hardware-Test danach zwingend:** Achse im TDC-Move + EtherCAT-Kabel ziehen → Motor muss binnen ~100 ms + Rampe stehen. Und verifizieren, dass normale TDC-Moves weiterhin praezise laufen (Beweis, dass "Watchdog vs TDC" wirklich ein Mythos war). Falls die Klemme sich doch anders verhaelt: Befund hier dokumentieren.

### 12.4 Weitere relevante Handbuch-Details

- **Prozessdaten-Diagnose (S.186):** `WcState != 0` = Klemme nimmt nicht am Prozessdatenverkehr teil; `State != 8` = nicht in OP; `SyncError != 0` = keine gueltigen Prozessdaten (z.B. Drahtbruch); `TxPDO Toggle` toggelt bei jedem neuen Datensatz. → Das sind die fertigen Hardware-Signale fuer unsere geplante **Link-Loss-Detection** (Phase 1): WcState/Toggle im Control-Loop ueberwachen statt eigenen Heartbeat zu erfinden. `sync_error` ist bei uns schon im TxPdo gemappt, wird aber nirgends ausgewertet.
- **Slowing down frequency (0x8000:17, Default 50 Hz):** Schleichgang-Frequenz am Ende jedes TDC-Moves = 2,5 mm/s bei 20 P/mm. Bestimmt die End-Praezision; bei Bedarf konfigurierbar machen.
- **Base frequency 1 (0x8000:12, bei uns 5000 Hz):** dient als Rampen-Referenz UND als Frequenz-Obergrenze. Aktuelle Maximal-Speeds (150 mm/s = 3000 Hz) liegen darunter = OK. Bei schnelleren Achsen base_frequency_1 erhoehen + set_axis_acceleration-Konstante mitziehen (siehe 12.2).
- **Micro increments (0x8020:0A):** bei uns aus = korrekt. Nur fuer Encoder-Simulation mit DC-Synchron relevant; interne 100-MHz-Grenze beachten.
- **C-Track/Adapt A/B (0x8000:01):** Nur fuer Inkremental-Encoder-Modus relevant, nicht fuer Pulse-Direction. Unser Default false = OK.
- **Auslieferungszustand der Klemme:** registriert sich als "2 Ch. Standard 32 Bit (MDP 253/511)" — genau unser Preset, daher kein PDO-Umkonfigurations-Risiko bei Klemmentausch.
- **Continuous-Position-Modi (S.187-188):** EL2522 kann zyklussynchron Positionen abfahren (NC-Stil, braucht DC-Synchronous). Waere die Alternative zu TDC, wenn wir je interpolierte Bahnen brauchen — aktuell nicht noetig, TDC reicht fuer Punkt-zu-Punkt.
- **CoE-Reset vor TDC-Parametrierung empfohlen (S.139):** "Restoring the delivery state" um Seiteneffekte auszuschliessen — fuer uns relevant nur bei hartnaeckigen Konfig-Problemen (Objekt 0x1011:01 = "load").

### 12.5 Abgeleitete Massnahmen (in Phasen-Plan eingeordnet)

1. **Phase 1 (Safety):** Watchdog reaktivieren mit emergency ramp (12.3) + Link-Loss via WcState/SyncError/TxPDO-Toggle (12.4) + Hardware-Test Kabelziehen. → **UMGESETZT 2026-06-10**, siehe ki-doku Session-Eintrag.
2. **Phase 1/2:** `set_axis_acceleration` falling-Rampe 10% steiler als rising (12.2), am Motor verifizieren. → **UMGESETZT 2026-06-10** (falling = 0.9 × rising).
3. **Backlog:** `sync_error`-Status-Bit pro Achse auswerten (steht schon im PDO, wird ignoriert); slowing_down_frequency ggf. konfigurierbar; base_freq-Konstante in mod.rs und new.rs zusammenfuehren (eine Quelle).

### 12.6 Dead-Man-Logik fuer Jog: OBSOLET (Entscheidung 2026-06-10)

Das TODO aus ki-doku 2026-05-27 ("Motor stoppt wenn 200 ms kein still-pressed-Signal kommt") stammt aus der Zeit, als Jog im Speed-Mode lief (Taste halten = fahren). Seit dem Virtual-Zero-Offset-Umbau (Commit d7bdf8cd) ist Jog ein **endlicher TDC-Move** (`JogRelative` = `move_to_position_mm` mit delta): Ein Klick faehrt exakt `step` mm und stoppt hardwareseitig — ein verlorenes "Loslassen"-Paket kann nichts mehr anrichten. **Dead-Man ist damit fuer Jog obsolet.**

Verbleibender kontinuierlicher Modus: der bewusste START/STOP-Lauf (`SetAxisSpeedMmS`) auf der Motoren-Seite. Der ist ein explizites Run-Kommando (wie ein physischer Schalter), begrenzt durch Soft-Limits (nach Homing), Tuer-Interlock und Treiber-Alarme. Bei WLAN-Abriss des Tablets laeuft er weiter — bewusst akzeptiert; wer ungehomte Achsen im Dauerlauf betreibt, steht an der Maschine (Kalibrier-Szenario). Falls sich das als Problem zeigt: optionaler Max-Runtime-Timeout im Backend waere der naechste Schritt.
