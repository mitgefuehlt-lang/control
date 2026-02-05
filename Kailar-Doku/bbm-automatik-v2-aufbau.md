# Aufbau BBM Automatik V2 (Blockbefüllmaschine)

## Maschinen-Beschreibung

**Zweck:** Vereinzelt Filterhülsen und befüllt Magazin-Blöcke
**Status:** In Planung

---

## Funktionsprinzip

1. Bürste + Rüttler vereinzeln Filterhülsen
2. Filter fallen durch Schläuche zum Schieber
3. Schieber nimmt 22 Filter auf (Einwurf-Position)
4. Schieber fährt über Magazin-Block (Auswurf-Position)
5. Filter fallen ins Magazin
6. Drücker drückt hängende Hülsen nach
7. Repeat bis Block voll (19 Zyklen wegen Überlauf)
8. MT bewegt Magazin kontinuierlich weiter
9. 3 Blöcke pro Magazin-Set

---

## Hardware-Konfiguration

### EtherCAT-Komponenten

| Anzahl | Typ | Funktion |
|--------|-----|----------|
| 1x | EK1100 | Bus Coupler |
| 1x | EL1008 | 8 digitale Eingänge |
| 1x | EL2008 | 8 digitale Ausgänge |
| 2x | EL2522 | PTO (Pulse Train Output), je 2 Kanäle |

**Gesamt:** 4 PTO-Kanäle, alle genutzt

---

## 4 Achsen

| # | Kürzel | Name | Typ | Funktion |
|---|--------|------|-----|----------|
| 1 | MT | Magazin Transporter | Linear | Bewegt Magazin kontinuierlich zwischen den 3 Blöcken |
| 2 | - | Schieber | Linear | Nimmt 22 Filter auf, fährt Einwurf ↔ Auswurf |
| 3 | - | Drücker | Linear | Drückt hängende Hülsen ins Magazin |
| 4 | - | Bürste | Rotation | Treibt Bürste an, vereinzelt Filter (Drehzahl einstellbar) |

### EL2522 Kanal-Zuordnung

| EL2522 # | Kanal 1 | Kanal 2 |
|----------|---------|---------|
| 1 | MT (Magazin Transporter) | Schieber |
| 2 | Drücker | Bürste |

---

## Sensoren & Ausgänge

### Digitale Eingänge (DI)

| Anzahl | Typ | Funktion | UI-Anzeige |
|--------|-----|----------|------------|
| 3x | Referenzschalter | Homing für Linear-Achsen (MT, Schieber, Drücker) | Nein (nur Backend) |
| 2x | Türsensoren | Sicherheit - Maschine stoppt wenn Tür offen | Ja (Test + Automatik) |

### Digitale Ausgänge (DO)

| Anzahl | Typ | Funktion | Steuerung |
|--------|-----|----------|-----------|
| 1x | Rüttelmotor | Hilft bei Vereinzelung | An/Aus (manuell oder automatisch) |
| 1x | Ampel Rot | Fehler/Stop | Automatisch |
| 1x | Ampel Gelb | Warnung | Automatisch |
| 1x | Ampel Grün | Läuft | Automatisch |

---

## Magazin-Spezifikation

```
Blöcke pro Magazin-Set: 3
Filterplätze pro Block: 102
Block-Layout: 6 Spalten x 17 Reihen
Filter pro Schieber-Zyklus: 22
Zyklen pro Block: 19 (inkl. Überlauf)
```

---

## UI-Konzept

### 4 Seiten (wie Schneidemaschine)

#### 1. Motoren-Seite
**Zweck:** Alle 4 Achsen einzeln testen und konfigurieren

**Pro Linear-Achse (MT, Schieber, Drücker):**
- Geschwindigkeit (mm/s)
- Beschleunigung (mm/s²)
- Sollposition (mm)
- Aktuelle Position (Anzeige)
- Buttons: START | ZUR POSITION | STOP | HOME

**Bürste (Rotation):**
- Drehzahl (Hz oder RPM)
- Buttons: START | STOP

**Rüttelmotor:**
- Toggle: AN / AUS

#### 2. Testbetrieb-Seite
**Zweck:** Sequenzen einzeln ausführen

**4 Sequenz-Buttons:**
1. 1x befüllen
2. 5x befüllen
3. 1 Magazin (19x)
4. Reset

**Status-Anzeige:**
- Türsensoren (2x) - Grün/Rot Badge
- Aktive Sequenz - Highlight
- Aktueller Zyklus (z.B. "5/19")

#### 3. Automatik-Seite
**Zweck:** Parametrierbare Produktion

**Parameter:**
- Geschwindigkeits-Level: Langsam / Mittel / Schnell
- Anzahl Magazin-Sets (je 3 Blöcke)

**Anzeige:**
- Türsensoren (Sicherheit)
- Produzierte Sets
- Aktueller Block (1-3)
- Aktuelle Reihe (1-19)
- Fortschrittsbalken

**Buttons:** START | PAUSE | STOP

#### 4. Status/Debug-Seite
**Zweck:** Überwachung und Fehleranalyse

**Anzeige:**
- Ampel-Status (Rot/Gelb/Grün)
- Rüttelmotor-Status
- Alle Achsen-Positionen
- Alle Sensor-Zustände
- Aktuelle Warnungen
- Fehler-Historie

---

## Presets

Parameter sollen als Presets speicherbar sein:
- Geschwindigkeit pro Achse
- Beschleunigung pro Achse
- Bürsten-Drehzahl
- Geschwindigkeits-Level Zuordnung

---

## Sicherheit

### Türsensoren
- 2 Türsensoren überwachen Maschinengehäuse
- Maschine stoppt sofort wenn Tür geöffnet wird
- Anzeige auf Test- und Automatik-Seite

### Referenzfahrt (Homing)
- 3 Linear-Achsen haben Referenzschalter
- Homing fährt zum Schalter, dann Null setzen
- Homing-Button auf Motoren-Seite

### Ampel
- **Grün:** Maschine läuft normal
- **Gelb:** Warnung
- **Rot:** Fehler oder Stop

---

## Maschinen-Ablauf (Automatik)

### Ein Befüll-Zyklus

1. Bürste + Rüttler laufen → Filter fallen in Schläuche
2. Schieber in Einwurf-Position → 22 Filter werden aufgenommen
3. Schieber fährt zur Auswurf-Position (über Block)
4. Filter fallen ins Magazin
5. Drücker drückt hängende Hülsen nach
6. Schieber fährt zurück zur Einwurf-Position

### Ein Block (19 Zyklen)

1. MT positioniert Block unter Schieber
2. 19x Befüll-Zyklus ausführen
3. Block voll (102 + Überlauf)

### Ein Magazin-Set (3 Blöcke)

1. Block 1 befüllen (19 Zyklen)
2. MT fährt weiter zu Block 2
3. Block 2 befüllen (19 Zyklen)
4. MT fährt weiter zu Block 3
5. Block 3 befüllen (19 Zyklen)
6. Magazin-Set fertig

---

## Implementierungs-Status

- [ ] Backend erstellen (4 Achsen)
- [ ] Frontend Namespace + Hook
- [ ] Motoren-Seite
- [ ] Testbetrieb-Seite (4 Sequenzen)
- [ ] Automatik-Seite
- [ ] Status/Debug-Seite
- [ ] Presets
- [ ] Hardware-Integration (2x EL2522)
