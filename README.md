<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### The readers of the EH3D Fabrication Suite: Gerber RS-274X, Excellon drill files and G-code, tolerant of damaged files

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** Rust libraries that read Gerber (RS-274X) layers, Excellon drill files and G-code with a modal interpreter (GRBL, Marlin and PlanetCNC dialects), tested with 42 tests. They never panic and report what they could not understand as diagnostics with line numbers. They draw nothing, compute no toolpath and have not been tried on a collection of real production files yet; HP-GL and 2D geometry (offsets, booleans) are planned.

---

> **Safety and security status:** these libraries are not a safety system. They read files that other people may send, and they are tested with damaged and hostile input, but a program that was read is **not** a verified program: nothing here decides whether a G-code is safe to run on a machine, and no machine has been controlled with it.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.1` |
| Maturity | Scaffolding |
| Tests | 42 |
| Stack | Rust |
| On real hardware | Not applicable: file readers, nothing here talks to a machine |

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Software | 🟡 | Three readers (Gerber, Excellon, G-code) work as Rust libraries and go on reading damaged files; no program of the suite uses them yet. |
| Tests | 🟢 | 42 tests pass (`cargo test`), including damaged and hostile input. |
| Gerber reader | 🟡 | RS-274X: units, coordinate format, apertures, macros, regions and drawing commands are read; only test files so far, not a collection of real production files. |
| Excellon reader | 🟡 | Tools, drill hits, slots and simple routes in metric or inches are read; only test files so far. |
| G-code reader | 🟡 | A modal interpreter for the GRBL, Marlin and PlanetCNC dialects gives absolute millimetres, arcs included; cutter compensation and canned cycles are reported as unsupported. |
| Geometry and HP-GL | 🔴 | The 2D geometry (offsets, booleans) and the HP-GL and Bungard readers are planned and not built. |
| Security | 🟡 | Built for files that other people send: nothing panics and problems come back as diagnostics with line numbers; no independent review, and nothing here judges whether a G-code is safe to run. |
| Real hardware | 🟢 | Not applicable: file readers, nothing here talks to a machine. |
| Documentation | 🟢 | README in eight languages and `docs/READERS.md`, which says what each reader promises and what it does not do. |

## 🎯 Overview

* The **Gerber reader** (`eh3d-parser-gerber`, RS-274X): units, the coordinate format with its zero suppression, apertures and macros, regions and the drawing commands. A coordinate wider than its format is refused instead of guessed.
* The **Excellon reader** (`eh3d-parser-excellon`): tools, drill hits, slots (`G85`) and simple routes, in metric or inches, with leading or trailing zero suppression.
* The **G-code reader and interpreter** (`eh3d-parser-gcode`): words, comments, line numbers and Marlin checksums, and the modal state of a controller (units, distance mode, feed, spindle, tool). Every motion comes out in absolute millimetres, arcs included, with the length of the cut and the bounding box of the program.
* **Diagnostics instead of failures:** each reader returns lines with a severity (information, warning, error) and the number of the line of the file, and goes on reading.
* **Planned:** the HP-GL and Bungard reader and the 2D geometry (offsets and booleans).

## 🧩 Role in EH3D

EH3D-CORE-LIBS turns files into data for the programs of the suite (isolation, drilling, contour, laser and stencil CAM, and the viewers). It has no screen and no machine control. It is planned to use the types of EH3D-COMMON; today the three readers stand on their own.

## 📂 Repository Structure

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

## 🛠️ BUILD & RUN

You need a stable Rust toolchain.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Related Projects

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the project container and the machine types; published
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): planned: isolation milling from the Gerber reader
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): planned: drilling from the Excellon reader
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): planned: simulation of the G-code that these readers interpret

## 📚 Documentation & Community

* [What each reader promises](docs/READERS.md): the guarantees, the diagnostics and what is not done.
* [Contributing](CONTRIBUTING.md), [Security policy](SECURITY.md), [Support](SUPPORT.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
