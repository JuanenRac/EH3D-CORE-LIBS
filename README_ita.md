<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### I lettori della EH3D Fabrication Suite: Gerber RS-274X, file di foratura Excellon e G-code, tolleranti verso i file danneggiati

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** tre librerie Rust senza dipendenze che leggono livelli Gerber (RS-274X), file di foratura Excellon e G-code (un interprete modale dei dialetti GRBL, Marlin e PlanetCNC). 42 test girano su un computer, compresi input danneggiati e ostili. I lettori non vanno mai in panico e segnalano ciò che non hanno capito come diagnostiche con numero di riga. Non disegnano nulla, non calcolano alcun percorso utensile e non sono stati provati su una raccolta di file di produzione reali; nessun programma della suite li usa ancora e nulla qui è stato eseguito su una macchina.

---

> **Stato di sicurezza funzionale e informatica:** queste librerie non sono un sistema di sicurezza funzionale. Leggono file che altre persone possono inviare e sono testate con input danneggiati e ostili, ma un programma letto **non** è un programma verificato: nulla qui decide se un G-code sia sicuro da eseguire su una macchina e nessuna macchina è stata controllata con esse.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.1` |
| Maturità | Impalcatura |
| Test | 42 |
| Stack tecnologico | Rust |
| Su hardware reale | Non applicabile: lettori di file, nulla qui parla con una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Lettore Gerber | 🟡 | RS-274X: unità, formato delle coordinate, aperture, macro, regioni e comandi di disegno vengono letti; finora solo file di prova, non una raccolta di file di produzione reali. |
| Lettore Excellon | 🟡 | Vengono letti utensili, fori, asole e percorsi semplici in metrico o pollici; finora solo file di prova. |
| Lettore G-code | 🟡 | Un interprete modale dei dialetti GRBL, Marlin e PlanetCNC dà millimetri assoluti, archi inclusi; compensazione del raggio e cicli fissi sono segnalati come non supportati. |
| Test | 🟡 | Passano 42 test (`cargo test`): 12 di Gerber più 1 nel modulo, 11 di Excellon e 18 di G-code, con input danneggiati e ostili. |
| Sicurezza | 🟡 | Pensati per file inviati da altre persone: nulla va in panico, nessun valore viene indovinato e i problemi tornano come diagnostiche con numero di riga; nessuna revisione indipendente e nulla qui giudica se un G-code sia sicuro da eseguire. |
| Geometria e HP-GL | 🔴 | La geometria 2D (offset, booleane) e i lettori HP-GL e Bungard sono previsti e non costruiti. |
| Uso da altri programmi | 🔴 | Nessun programma della suite dipende ancora dai lettori. |
| Hardware reale | 🟡 | Non applicabile: lettori di file, nulla qui parla con una macchina. |
| Documentazione | 🟡 | README in otto lingue e `docs/READERS.md`, che dice cosa promette ogni lettore e cosa non fa. |

## 🎯 Panoramica

* Il **lettore Gerber** (`eh3d-parser-gerber`, RS-274X): unità, il formato delle coordinate con la sua soppressione degli zeri, aperture (cerchio, rettangolo, obround, poligono) e macro, regioni, step and repeat e i comandi di disegno. Una coordinata più larga del suo formato viene rifiutata invece di essere indovinata.
* Il **lettore Excellon** (`eh3d-parser-excellon`): utensili con diametro, avanzamento e velocità, fori, asole (`G85`) e percorsi semplici, in metrico o pollici, con soppressione degli zeri iniziali o finali.
* Il **lettore e interprete G-code** (`eh3d-parser-gcode`): parole, commenti, numeri di riga e checksum Marlin, e lo stato modale di un controllore (unità, modo di distanza, avanzamento, mandrino, utensile). Ogni movimento esce in millimetri assoluti, archi inclusi, con la lunghezza del taglio, la lunghezza dei rapidi e il riquadro del programma.
* **Diagnostiche invece di errori fatali:** ogni lettore restituisce righe con una gravità (informazione, avviso, errore) e il numero di riga del file, e continua a leggere.
* **Previsto:** il lettore HP-GL e Bungard e la geometria 2D (offset e booleane).

## 🧩 Ruolo in EH3D

EH3D-CORE-LIBS trasforma i file in dati per i programmi della suite (CAM di isolamento, foratura, contorno, laser e stencil, e i visualizzatori). Non ha schermo né controllo macchina. È previsto che usi i tipi di EH3D-COMMON; oggi i tre lettori sono autonomi, ed EH3D-DRILLPRO ha ancora il proprio lettore Excellon.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-CORE-LIBS: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>

Come si incastrano le parti di questo repository: chi lo usa, che cosa contiene e da che cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata rispetto al codice.

## ✨ Funzionalità

* **Zero dipendenze:** i tre crate usano solo la libreria standard di Rust.
* **Non va mai in panico:** i test danno a ogni lettore file vuoti, spazzatura binaria, numeri enormi, commenti non chiusi, righe di migliaia di caratteri e ogni prefisso di un file valido.
* **Niente indovinelli:** un valore che non entra nel formato dichiarato viene rifiutato come diagnostica, non arrotondato in qualcosa che sembra giusto.
* **Ogni comando conosce la sua riga:** le operazioni Gerber, i fori Excellon e i blocchi G-code portano la riga del file da cui vengono.
* **Archi fatti bene:** l'interprete G-code gestisce archi con `I`/`J` o con `R`, cerchi completi, centri d'arco assoluti, e segnala un arco che non può esistere disegnandolo dritto.
* **Onesto su ciò che salta:** le funzioni non supportate (compensazione del raggio, cicli fissi) sono errori e il blocco non muove nulla; i comandi di sistema `$` di GRBL vengono saltati con una nota.

## 🔒 Sicurezza e protezione

I lettori accettano testo da chiunque, quindi il progetto presuppone file trascurati o ostili. Ogni riga ha test.

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, troncato o binario | nessun panico: ogni lettore restituisce un risultato per qualsiasi testo; i test tagliano un file valido in ogni posizione e gli danno spazzatura |
| Un numero troppo largo per il formato dichiarato, o un valore assurdo | rifiutato con una diagnostica `Error`, mai arrotondato né troncato in qualcosa di plausibile |
| Una singola riga di lunghezza enorme | il lettore G-code taglia una riga a 512 caratteri (`MAX_LINE_CHARS`) e lo segnala |
| Una funzione G-code che l'interprete non modella (compensazione del raggio, cicli fissi) | una diagnostica `Error`, e il blocco non muove nulla, così un programma non viene mai letto male in silenzio |
| Una riga Marlin corrotta | il checksum viene verificato e uno sbagliato viene segnalato |
| Un arco che non può esistere (il raggio è minore di metà corda) | segnalato e disegnato dritto |
| Uso non sicuro della memoria | i lettori sono Rust sicuro senza dipendenze da verificare |

**Ciò che non è difeso:** un programma letto senza problemi non è un programma verificato. Nulla qui controlla i limiti di una macchina, le lunghezze utensile o le collisioni, e i lettori non hanno avuto una revisione indipendente. Segnala una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

| Crate | Punti d'ingresso | Risultato |
|---|---|---|
| `eh3d-parser-gerber` | `parse(text)`, `decode_coordinate(text, format)` | `GerberFile`: unità, formato, aperture, macro, attributi, `commands`, `ended`, `diagnostics` |
| `eh3d-parser-excellon` | `parse(text)`, `decode_coordinate(...)` | `ExcellonFile`: unità, soppressione degli zeri, `tools`, `drills`, `slots`, `routes`, `ended`, `diagnostics` |
| `eh3d-parser-gcode` | `parse(text, dialect)`, poi `interpret(&program)` | `Program` (blocchi) e `Trace` (segmenti in millimetri, riquadro, lunghezze di taglio e di rapidi) |

Tutti e tre condividono `Diagnostic { line, severity, message }` con `Severity` `Info`, `Warning` o `Error`, e `is_clean()` per chiedere se è stato trovato un `Error`. Non c'è programma a riga di comando, né accesso alla rete, né accesso ai file: chi chiama passa il testo e riceve i dati.

## ⚙️ Configurazione

I lettori non hanno un file di configurazione. Ciò che sceglie chi chiama e i limiti fissi sono questi:

| Valore | Intervallo | Dove |
|---|---|---|
| Dialetto G-code | `Grbl`, `Marlin` o `PlanetCnc` | il secondo argomento di `parse` |
| Lunghezza di una riga G-code | fino a 512 caratteri; il resto viene tagliato e segnalato | `MAX_LINE_CHARS` in `crates/eh3d-parser-gcode/src/lib.rs` |
| Unità | prese dal file (`MO` in Gerber, `METRIC` o `INCH` in Excellon, `G20`/`G21` in G-code); l'uscita G-code è sempre in millimetri | i lettori |
| Formato delle coordinate | preso dal file (`FS` in Gerber, `FILE_FORMAT=i:d` o l'intestazione in Excellon); un valore più largo è un errore | i lettori |
| Soppressione degli zeri | iniziale o finale, come dice il file | i lettori |

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.75 o successiva).

```rust
use eh3d_parser_gcode::{interpret, parse, Dialect};

fn main() {
    let program = parse("G21\nG90\nG1 X10 Y0 F300\nM2\n", Dialect::Grbl);
    let trace = interpret(&program);
    println!("{} mm of cut, clean: {}", trace.cutting_length_mm, program.is_clean());
}
```

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🧪 Test

La suite ha 42 test (`cargo test`, eseguiti su Windows e Linux nella CI): 12 su Gerber (più 1 nel modulo), 11 su Excellon e 18 su G-code. Verificano che un tipico livello di rame e un tipico file metrico siano letti per intero, le coordinate omesse, entrambe le soppressioni degli zeri, asole, percorsi, pollici in millimetri, ogni comando che conosce la sua riga, archi per centro e per raggio, cerchi completi, avanzamento e movimento modali, checksum Marlin, funzioni non supportate segnalate invece che indovinate, e input danneggiati: spazzatura, prefissi tagliati e righe lunghissime non vanno mai in panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` (il workspace), in `eh3d.project.json` e, tramite il workspace, in ciascuno dei tre crate; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). Le versioni vengono pubblicate su GitHub; i crate si usano da questo repository e non sono su crates.io.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-CORE-LIBS" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-CORE-LIBS/
├── crates/
│   ├── eh3d-parser-gerber/                      lettore Gerber RS-274X
│   ├── eh3d-parser-excellon/                    lettore di file di foratura Excellon
│   └── eh3d-parser-gcode/                       lettore e interprete modale di G-code
├── docs/                                        READERS.md, cosa promette ogni lettore e cosa non fa, e i dati delle due immagini
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* I lettori sono stati provati solo con file di prova, non con una raccolta di file di produzione reali.
* La compensazione del raggio utensile (`G41`/`G42`) e i cicli fissi (`G81`..`G89`) sono errori; le correzioni di lunghezza utensile, i cicli di tastatura, `G28`/`G30` e i sistemi di coordinate di lavoro vengono letti ma il loro effetto sulla posizione non è seguito; archi solo nel piano XY.
* HP-GL, il formato Bungard e la geometria 2D (offset e booleane) non sono costruiti.
* Nessun programma della suite usa ancora i lettori, e i lettori non hanno avuto una revisione indipendente.

**Prossimi passi**

* Provare i tre lettori su una raccolta di file reali e correggere ciò che mostrano.
* Costruire la geometria 2D (offset e booleane) e i lettori HP-GL e Bungard.
* Far usare i lettori a EH3D-DRILLPRO e ai programmi CAM al posto dei propri.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): il contenitore di progetto e i tipi di macchina della suite; pubblicato
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): il programma di foratura; oggi ha il proprio lettore Excellon
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): previsto: fresatura di isolamento dal lettore Gerber
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): previsto: simulazione del G-code che questi lettori interpretano

## 📚 Documentazione

* [Cosa promette ogni lettore](docs/READERS.md): le garanzie, le diagnostiche e ciò che non viene fatto.
* [Registro delle modifiche](CHANGELOG.md)

## 🤝 Contribuire e comunità

* Prima di una pull request esegui `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Un lettore non deve mai andare in panico né indovinare: un nuovo caso richiede un test con input danneggiato, e una funzione non supportata viene segnalata, non saltata in silenzio.
* Segnala una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
