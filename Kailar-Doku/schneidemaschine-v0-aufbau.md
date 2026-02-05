# Aufbau SchneidemaschineV0

## Maschinen-Beschreibung

**Zweck:** Zellulose-Schneidmaschine
**Status:** In Entwicklung (Hardware im Aufbau)

---

## Hardware-Konfiguration

### EtherCAT-Komponenten

| Anzahl | Typ | Funktion |
|--------|-----|----------|
| 1x | EK1100 | Bus Coupler |
| 1x | EL1008 | 8 digitale Eingänge |
| 1x | EL2008 | 8 digitale Ausgänge |
| 5x | EL2522 | PTO (Pulse Train Output), je 2 Kanäle |

**Gesamt:** 10 PTO-Kanäle verfügbar, 9 genutzt

---

## 9 Achsen

### Linear-Achsen (7)

| # | Kürzel | Name | Funktion |
|---|--------|------|----------|
| 1 | MT | Magazin Transport | Transportiert fertiges Magazin zur Schneidestation und zum Auswurf |
| 2 | ME | Magazin Einwurf | Vereinzelt Magazin aus Stapel, schiebt auf MT |
| 3 | CT | Cellulose Transporter | Fährt über leeres Magazin, transportiert Zellulose |
| 4 | CD | Cellulose Drücker | Drückt Zellulose-Stück ins Magazin |
| 5 | ST | Schneider Transport | Fährt Schneider-Motor vor zum Schneiden |
| 6 | MA | Magazin Auswurf | Wirft volles Magazin aus |
| 7 | MD | Müll Drücker | Wirft letzten Zellulose-Rest aus (Abfall) |

### Rotations-Achsen (2)

| # | Kürzel | Name | Funktion |
|---|--------|------|----------|
| 8 | CR | Cellulose Revolver | Dreht sich, befüllt CT mit Zellulose-Stäben |
| 9 | SK | Schneider Klinge | Rotierende Klinge, schneidet Zellulose durch |

---

## Sensoren & Ausgänge

### Digitale Eingänge (DI)

| Anzahl | Typ | Funktion | UI-Anzeige |
|--------|-----|----------|------------|
| 7x | Referenzschalter | Homing für Linear-Achsen | Nein (nur Backend) |
| 2x | Türsensoren | Sicherheit - Maschine stoppt wenn Tür offen | Ja (Test + Automatik) |

### Digitale Ausgänge (DO)

| Anzahl | Typ | Funktion | Steuerung |
|--------|-----|----------|-----------|
| 1x | Ampel Rot | Fehler/Stop | Automatisch |
| 1x | Ampel Gelb | Warnung | Automatisch |
| 1x | Ampel Grün | Läuft | Automatisch |

---

## Mechanik-Parameter

### Motor-Konfiguration (CL57T Stepper)

```
Pulse pro Umdrehung: 200 (Treiber-DIP-Schalter)
Kugelgewindespindel Lead: 10 mm
Pulse pro mm: 20 (200/10)
Max. Geschwindigkeit: 230 mm/s = 4600 Hz
Max. Frequenz EL2522: 5000 Hz
```

### Magazin

```
Reihen pro Magazin: 17
```

---

## Maschinen-Ablauf

### Produktionszyklus

1. **Referenzfahrt** - Alle Achsen fahren auf Home-Position
2. **ME** vereinzelt Magazin → schiebt auf **MT**
3. **CR** dreht sich → befüllt **CT** mit Zellulose-Stab
4. **CT** fährt über leeres Magazin
5. **CD** drückt Zellulose-Stück ins Magazin
6. **ST** fährt vor + **SK** schneidet (rotierend + linear)
7. Repeat Schritte 3-6 bis Reihe voll
8. Repeat Schritte 3-7 für alle 17 Reihen
9. **MT** fährt zum Auswurf
10. **MA** wirft Magazin aus
11. **CT** fährt zu **MD** → Rest auswerfen
12. Zurück zu Schritt 2

---

## UI-Konzept

### 4 Seiten

#### 1. Motoren-Seite
**Zweck:** Alle 9 Achsen einzeln testen und konfigurieren

**Pro Linear-Achse:**
- Geschwindigkeit (mm/s)
- Beschleunigung (mm/s²)
- Sollposition (mm)
- Aktuelle Position (Anzeige)
- Buttons: START | ZUR POSITION | STOP | HOME

**Pro Rotations-Achse:**
- Drehzahl (Hz oder RPM)
- Buttons: START | STOP

#### 2. Testbetrieb-Seite
**Zweck:** Sequenzen einzeln ausführen

**9 Sequenz-Buttons:**
1. Magazin in Schneidposition
2. Cellulose vereinzeln + Schneidposition
3. Erste Reihe drücken
4. Erste Reihe schneiden
5. 5x schneiden
6. Fertig schneiden
7. Magazin auswerfen
8. Müll drücken
9. Reset auf Start

**Status:** Türsensoren anzeigen

#### 3. Automatik-Seite
**Zweck:** Parametrierbare Produktion

**Parameter:**
- Schnittlänge (mm)
- Stückzahl pro Magazin
- Geschwindigkeit (%)
- Anzahl Magazine

**Anzeige:**
- Türsensoren
- Produzierte Stückzahl
- Aktuelle Reihe (1-17)
- Laufzeit
- Fortschrittsbalken

#### 4. Status/Debug-Seite
**Zweck:** Überwachung und Fehleranalyse

**Anzeige:**
- Ampel-Status
- Alle Achsen-Positionen
- Alle Sensor-Zustände
- Aktuelle Warnungen
- Fehler-Historie

---

## Presets

Parameter sollen als Presets speicherbar sein:
- Geschwindigkeit pro Achse
- Beschleunigung pro Achse
- Schnittlänge
- Produktions-Parameter

---

## Sicherheit

### Türsensoren
- 2 Türsensoren überwachen Maschinengehäuse
- Maschine stoppt sofort wenn Tür geöffnet wird
- Anzeige auf Test- und Automatik-Seite

### Referenzfahrt (Homing)
- Alle Linear-Achsen haben Referenzschalter
- Homing fährt zum Schalter, dann Null setzen
- Homing-Button auf Motoren-Seite

### Ampel
- **Grün:** Maschine läuft normal
- **Gelb:** Warnung (z.B. Material niedrig)
- **Rot:** Fehler oder Stop

---

## EL2522 Kanal-Zuordnung

| EL2522 # | Kanal 1 | Kanal 2 |
|----------|---------|---------|
| 1 | MT (Magazin Transport) | ME (Magazin Einwurf) |
| 2 | CT (Cellulose Transporter) | CD (Cellulose Drücker) |
| 3 | ST (Schneider Transport) | MA (Magazin Auswurf) |
| 4 | MD (Müll Drücker) | CR (Cellulose Revolver) |
| 5 | SK (Schneider Klinge) | - (ungenutzt) |

---

## Implementierungs-Status

- [x] Basis-Backend (2 Achsen)
- [x] Basis-UI (Motoren-Seite rudimentär)
- [ ] Backend für 9 Achsen
- [ ] Motoren-Seite vollständig
- [ ] Testbetrieb-Seite
- [ ] Automatik-Seite
- [ ] Status/Debug-Seite
- [ ] Presets
- [ ] Hardware-Integration (5x EL2522)
