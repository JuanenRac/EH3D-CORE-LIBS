<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### I lettori della EH3D Fabrication Suite: Gerber RS-274X, file di foratura Excellon e G-code, tolleranti ai file danneggiati

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Controllo di onestà - cosa funziona oggi:** librerie Rust che leggono livelli Gerber (RS-274X), file di foratura Excellon e G-code con un interprete modale (dialetti GRBL, Marlin e PlanetCNC), provate con 42 test. Non vanno mai in panic e riferiscono ciò che non hanno capito come diagnostica con numero di riga. Non disegnano nulla, non calcolano alcun percorso utensile e non sono ancora state provate su una raccolta di file di produzione reali; HP-GL e la geometria 2D (offset, booleane) sono previsti.

---

> **Stato di sicurezza (safety) e di sicurezza informatica:** queste librerie non sono un sistema di sicurezza funzionale. Leggono file che altri possono inviare e sono provate con input danneggiati e ostili, ma un programma letto **non** è un programma verificato: nulla qui decide se un G-code sia sicuro da eseguire su una macchina, e nessuna macchina è stata controllata con esse.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.1` |
| Maturità | Impalcatura |
| Test | 42 |
| Stack tecnologico | Rust |
| Su hardware reale | Non applicabile: sono lettori di file, nulla qui parla con una macchina |

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Software | 🟡 | Tre lettori (Gerber, Excellon, G-code) funzionano come librerie Rust e continuano a leggere file danneggiati; nessun programma della suite li usa ancora. |
| Test | 🟢 | 42 test passano (`cargo test`), compresi input danneggiati e ostili. |
| Lettore Gerber | 🟡 | RS-274X: si leggono unità, formato delle coordinate, aperture, macro, regioni e comandi di disegno; finora solo file di prova, non una raccolta di file di produzione reali. |
| Lettore Excellon | 🟡 | Si leggono utensili, fori, asole e percorsi semplici in metrico o in pollici; finora solo file di prova. |
| Lettore G-code | 🟡 | Un interprete modale dei dialetti GRBL, Marlin e PlanetCNC restituisce millimetri assoluti, archi inclusi; la compensazione utensile e i cicli fissi sono segnalati come non supportati. |
| Geometria e HP-GL | 🔴 | La geometria 2D (offset, operazioni booleane) e i lettori HP-GL e Bungard sono previsti e non costruiti. |
| Sicurezza | 🟡 | Pensato per file inviati da altri: niente va in panico e i problemi tornano come diagnostiche con numero di riga; nessuna revisione indipendente, e nulla qui giudica se un G-code sia sicuro da eseguire. |
| Hardware reale | 🟢 | Non applicabile: sono lettori di file, nulla qui parla con una macchina. |
| Documentazione | 🟢 | README in otto lingue e `docs/READERS.md`, che dice cosa promette ogni lettore e cosa non fa. |

## 🎯 Panoramica

* Il **lettore Gerber** (`eh3d-parser-gerber`, RS-274X): unità, formato delle coordinate con la soppressione degli zeri, aperture e macro, regioni e comandi di disegno. Una coordinata più larga del suo formato viene rifiutata invece di essere indovinata.
* Il **lettore Excellon** (`eh3d-parser-excellon`): utensili, fori, asole (`G85`) e percorsi semplici, in metrico o in pollici, con soppressione degli zeri iniziali o finali.
* Il **lettore e interprete di G-code** (`eh3d-parser-gcode`): parole, commenti, numeri di riga e checksum Marlin, e lo stato modale di un controllore (unità, modo di distanza, avanzamento, mandrino, utensile). Ogni movimento esce in millimetri assoluti, archi compresi, con la lunghezza del taglio e il riquadro di ingombro del programma.
* **Diagnostica invece di errori fatali:** ogni lettore restituisce righe con una gravità (informazione, avviso, errore) e il numero di riga del file, e continua a leggere.
* **Previsto:** il lettore HP-GL e Bungard e la geometria 2D (offset e booleane).

## 🧩 Ruolo in EH3D

EH3D-CORE-LIBS trasforma i file in dati per i programmi della suite (CAM di isolamento, foratura, contorno, laser e stencil, e i visualizzatori). Non ha schermo né controllo della macchina. È previsto che usi i tipi di EH3D-COMMON; oggi i tre lettori funzionano da soli.

## 📂 Struttura del repository

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

## 🛠️ COMPILARE ED ESEGUIRE

Serve una toolchain Rust stabile.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): il contenitore di progetto e i tipi di macchina; pubblicato
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): previsto: fresatura di isolamento dal lettore Gerber
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): previsto: foratura dal lettore Excellon
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): previsto: simulazione del G-code che questi lettori interpretano

## 📚 Documentazione e comunità

* [Cosa promette ogni lettore](docs/READERS.md): le garanzie, la diagnostica e ciò che non si fa.
* [Contribuire](CONTRIBUTING.md), [politica di sicurezza](SECURITY.md), [supporto](SUPPORT.md) e [codice di condotta](CODE_OF_CONDUCT.md).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
