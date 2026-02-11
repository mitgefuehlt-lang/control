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
