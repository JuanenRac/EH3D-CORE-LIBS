<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Die Leser der EH3D Fabrication Suite: Gerber RS-274X, Excellon-Bohrdateien und G-Code, tolerant gegenüber beschädigten Dateien

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitsprüfung - was heute läuft:** Rust-Bibliotheken, die Gerber-Ebenen (RS-274X), Excellon-Bohrdateien und G-Code mit einem modalen Interpreter (Dialekte GRBL, Marlin und PlanetCNC) lesen, getestet mit 42 Tests. Sie geraten nie in Panik und melden, was sie nicht verstanden haben, als Diagnosen mit Zeilennummer. Sie zeichnen nichts, berechnen keinen Werkzeugweg und wurden noch nicht an einer Sammlung echter Produktionsdateien erprobt; HP-GL und 2D-Geometrie (Offsets, Boolesche Operationen) sind geplant.

---

> **Stand von Sicherheit (Safety) und IT-Sicherheit:** Diese Bibliotheken sind kein Sicherheitssystem. Sie lesen Dateien, die andere schicken können, und werden mit beschädigten und feindlichen Eingaben getestet, aber ein gelesenes Programm ist **kein** geprüftes Programm: nichts hier entscheidet, ob ein G-Code auf einer Maschine sicher ausgeführt werden kann, und mit ihnen wurde keine Maschine gesteuert.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.1` |
| Reifegrad | Gerüst |
| Tests | 42 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht zutreffend: Dateileser, nichts hier spricht mit einer Maschine |

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Software | 🟡 | Drei Leser (Gerber, Excellon, G-Code) funktionieren als Rust-Bibliotheken und lesen beschädigte Dateien weiter; noch kein Programm der Suite verwendet sie. |
| Tests | 🟢 | 42 Tests bestehen (`cargo test`), darunter beschädigte und feindliche Eingaben. |
| Gerber-Leser | 🟡 | RS-274X: Einheiten, Koordinatenformat, Blenden, Makros, Regionen und Zeichenbefehle werden gelesen; bisher nur Testdateien, keine Sammlung echter Produktionsdateien. |
| Excellon-Leser | 🟡 | Werkzeuge, Bohrungen, Langlöcher und einfache Fräsbahnen in metrisch oder Zoll werden gelesen; bisher nur Testdateien. |
| G-Code-Leser | 🟡 | Ein modaler Interpreter für die Dialekte GRBL, Marlin und PlanetCNC liefert absolute Millimeter, Bögen eingeschlossen; Werkzeugkorrektur und Bearbeitungszyklen werden als nicht unterstützt gemeldet. |
| Geometrie und HP-GL | 🔴 | Die 2D-Geometrie (Offsets, boolesche Operationen) sowie die HP-GL- und Bungard-Leser sind geplant und nicht gebaut. |
| Sicherheit | 🟡 | Für Dateien gebaut, die andere schicken: nichts bricht ab, und Probleme kommen als Diagnosen mit Zeilennummer zurück; keine unabhängige Prüfung, und nichts hier beurteilt, ob ein G-Code sicher ausgeführt werden kann. |
| Echte Hardware | 🟢 | Nicht zutreffend: Dateileser, nichts hier spricht mit einer Maschine. |
| Dokumentation | 🟢 | README in acht Sprachen und `docs/READERS.md`, das sagt, was jeder Leser verspricht und was er nicht tut. |

## 🎯 Überblick

* Der **Gerber-Leser** (`eh3d-parser-gerber`, RS-274X): Einheiten, das Koordinatenformat mit seiner Nullenunterdrückung, Aperturen und Makros, Regionen und die Zeichenbefehle. Eine Koordinate, die breiter ist als ihr Format, wird abgelehnt statt geraten.
* Der **Excellon-Leser** (`eh3d-parser-excellon`): Werkzeuge, Bohrungen, Langlöcher (`G85`) und einfache Fräswege, metrisch oder in Zoll, mit Unterdrückung führender oder nachgestellter Nullen.
* Der **G-Code-Leser und -Interpreter** (`eh3d-parser-gcode`): Wörter, Kommentare, Zeilennummern und Marlin-Prüfsummen sowie der modale Zustand einer Steuerung (Einheiten, Maßangabe, Vorschub, Spindel, Werkzeug). Jede Bewegung kommt in absoluten Millimetern heraus, Bögen eingeschlossen, mit der Länge des Schnitts und der Hüllbox des Programms.
* **Diagnosen statt Abbrüchen:** jeder Leser liefert Zeilen mit einem Schweregrad (Information, Warnung, Fehler) und der Zeilennummer der Datei und liest weiter.
* **Geplant:** der HP-GL- und Bungard-Leser und die 2D-Geometrie (Offsets und Boolesche Operationen).

## 🧩 Rolle in EH3D

EH3D-CORE-LIBS macht aus Dateien Daten für die Programme der Suite (CAM für Isolation, Bohren, Kontur, Laser und Schablone sowie die Betrachter). Es hat keinen Bildschirm und keine Maschinensteuerung. Es soll die Typen von EH3D-COMMON verwenden; heute stehen die drei Leser für sich.

## 📂 Repository-Struktur

```text
EH3D-CORE-LIBS/
├── crates/
│   ├── eh3d-parser-gerber/     Gerber RS-274X reader
│   ├── eh3d-parser-excellon/   Excellon drill file reader
│   └── eh3d-parser-gcode/      G-code reader and modal interpreter
├── docs/
│   └── READERS.md              what every reader promises and what it does not do
├── tools/
│   └── ci_validate.py          the checks the CI runs
├── Cargo.toml                  the workspace
└── eh3d.project.json           the manifest of the project
```

## 🛠️ BAUEN & AUSFÜHREN

Sie brauchen eine stabile Rust-Toolchain.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Verwandte Projekte

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): der Projektcontainer und die Maschinentypen; veröffentlicht
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): geplant: Isolationsfräsen aus dem Gerber-Leser
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): geplant: Bohren aus dem Excellon-Leser
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): geplant: Simulation des G-Codes, den diese Leser interpretieren

## 📚 Dokumentation & Community

* [Was jeder Leser verspricht](docs/READERS.md): die Garantien, die Diagnosen und was nicht gemacht wird.
* [Mitwirken](CONTRIBUTING.md), [Sicherheitsrichtlinie](SECURITY.md), [Support](SUPPORT.md) und [Verhaltenskodex](CODE_OF_CONDUCT.md).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
