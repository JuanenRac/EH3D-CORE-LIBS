<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Les lecteurs de l'EH3D Fabrication Suite : Gerber RS-274X, fichiers de perçage Excellon et G-code, tolérants aux fichiers endommagés

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** trois bibliothèques Rust sans dépendances qui lisent des couches Gerber (RS-274X), des fichiers de perçage Excellon et du G-code (un interpréteur modal des dialectes GRBL, Marlin et PlanetCNC). 42 tests tournent sur un ordinateur, y compris avec des entrées endommagées et hostiles. Les lecteurs ne paniquent jamais et signalent ce qu'ils n'ont pas pu comprendre sous forme de diagnostics avec numéro de ligne. Ils ne dessinent rien, ne calculent aucun parcours d'outil et n'ont pas été essayés sur une collection de vrais fichiers de production ; aucun programme de la suite ne les utilise encore et rien ici n'a tourné sur une machine.

---

> **État de la sûreté et de la sécurité :** ces bibliothèques ne sont pas un système de sûreté. Elles lisent des fichiers que d'autres personnes peuvent envoyer et sont testées avec des entrées endommagées et hostiles, mais un programme qui a été lu n'est **pas** un programme vérifié : rien ici ne décide si un G-code peut être exécuté sans danger sur une machine, et aucune machine n'a été pilotée avec elles.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 42 |
| Pile technique | Rust |
| Sur du matériel réel | Sans objet : des lecteurs de fichiers, rien ici ne parle à une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Lecteur Gerber | 🟡 | RS-274X : unités, format des coordonnées, outils d'ouverture, macros, régions et commandes de dessin sont lus ; seulement des fichiers de test pour l'instant, pas une collection de vrais fichiers de production. |
| Lecteur Excellon | 🟡 | Outils, trous, rainures et parcours simples en métrique ou en pouces sont lus ; seulement des fichiers de test pour l'instant. |
| Lecteur G-code | 🟡 | Un interpréteur modal des dialectes GRBL, Marlin et PlanetCNC donne des millimètres absolus, arcs compris ; la compensation de rayon et les cycles fixes sont signalés comme non pris en charge. |
| Tests | 🟢 | 42 tests réussissent (`cargo test`) : 12 pour Gerber plus 1 dans le module, 11 pour Excellon et 18 pour G-code, avec des entrées endommagées et hostiles. |
| Sécurité | 🟡 | Conçus pour des fichiers envoyés par d'autres personnes : rien ne panique, aucune valeur n'est devinée et les problèmes reviennent sous forme de diagnostics avec numéro de ligne ; pas de revue indépendante, et rien ici ne juge si un G-code peut être exécuté sans danger. |
| Géométrie et HP-GL | 🔴 | La géométrie 2D (décalages, booléennes) et les lecteurs HP-GL et Bungard sont prévus et non construits. |
| Utilisation par d'autres programmes | 🔴 | Aucun programme de la suite ne dépend encore des lecteurs. |
| Matériel réel | 🟢 | Sans objet : des lecteurs de fichiers, rien ici ne parle à une machine. |
| Documentation | 🟢 | README en huit langues et `docs/READERS.md`, qui dit ce que chaque lecteur promet et ce qu'il ne fait pas. |

## 🎯 Présentation

* Le **lecteur Gerber** (`eh3d-parser-gerber`, RS-274X) : unités, le format des coordonnées avec sa suppression des zéros, ouvertures (cercle, rectangle, oblong, polygone) et macros, régions, répétition (step and repeat) et commandes de dessin. Une coordonnée plus large que son format est refusée au lieu d'être devinée.
* Le **lecteur Excellon** (`eh3d-parser-excellon`) : outils avec diamètre, avance et vitesse, trous, rainures (`G85`) et parcours simples, en métrique ou en pouces, avec suppression des zéros en tête ou en fin.
* Le **lecteur et interpréteur G-code** (`eh3d-parser-gcode`) : mots, commentaires, numéros de ligne et sommes de contrôle Marlin, et l'état modal d'un contrôleur (unités, mode de distance, avance, broche, outil). Chaque mouvement sort en millimètres absolus, arcs compris, avec la longueur de coupe, la longueur des rapides et la boîte englobante du programme.
* **Des diagnostics plutôt que des échecs :** chaque lecteur renvoie des lignes avec une gravité (information, avertissement, erreur) et le numéro de ligne du fichier, et continue la lecture.
* **Prévu :** le lecteur HP-GL et Bungard et la géométrie 2D (décalages et booléennes).

## 🧩 Rôle dans EH3D

EH3D-CORE-LIBS transforme des fichiers en données pour les programmes de la suite (FAO d'isolation, de perçage, de contour, laser et pochoir, et les visionneuses). Il n'a ni écran ni pilotage de machine. Il est prévu qu'il utilise les types d'EH3D-COMMON ; aujourd'hui les trois lecteurs sont autonomes, et EH3D-DRILLPRO a encore son propre lecteur Excellon.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-CORE-LIBS : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>

Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée à partir de `docs/architecture.json` et vérifiée par rapport au code.

## ✨ Fonctionnalités

* **Zéro dépendance :** les trois crates n'utilisent que la bibliothèque standard de Rust.
* **Ne panique jamais :** les tests donnent à chaque lecteur des fichiers vides, du charabia binaire, des nombres énormes, des commentaires non fermés, des lignes de milliers de caractères et chaque préfixe d'un fichier valide.
* **Pas de devinettes :** une valeur qui ne tient pas dans son format déclaré est refusée comme diagnostic, pas arrondie en quelque chose qui a l'air juste.
* **Chaque commande connaît sa ligne :** les opérations Gerber, les trous Excellon et les blocs G-code portent la ligne du fichier d'où ils viennent.
* **Des arcs bien faits :** l'interpréteur G-code gère les arcs avec `I`/`J` ou avec `R`, les cercles complets, les centres d'arc absolus, et signale un arc qui ne peut pas exister en le traçant droit.
* **Honnête sur ce qu'il saute :** les fonctions non prises en charge (compensation de rayon, cycles fixes) sont des erreurs et le bloc ne déplace rien ; les commandes système `$` de GRBL sont sautées avec une note.

## 🔒 Sécurité et protection

Les lecteurs prennent du texte de n'importe qui ; la conception suppose donc des fichiers négligés ou hostiles. Chaque ligne a ses tests.

| Menace | Défense |
|---|---|
| Un fichier endommagé, tronqué ou binaire | pas de panique : chaque lecteur renvoie un résultat pour n'importe quel texte ; les tests coupent un fichier valide à chaque position et lui donnent du charabia |
| Un nombre trop large pour son format déclaré, ou une valeur absurde | refusé avec un diagnostic `Error`, jamais arrondi ni tronqué en quelque chose de plausible |
| Une seule ligne d'une longueur énorme | le lecteur G-code coupe une ligne à 512 caractères (`MAX_LINE_CHARS`) et le dit |
| Une fonction G-code que l'interpréteur ne modélise pas (compensation de rayon, cycles fixes) | un diagnostic `Error`, et le bloc ne déplace rien : un programme n'est jamais mal lu en silence |
| Une ligne Marlin corrompue | la somme de contrôle est vérifiée et une mauvaise est signalée |
| Un arc qui ne peut pas exister (le rayon est inférieur à la moitié de la corde) | signalé et tracé droit |
| Usage dangereux de la mémoire | les lecteurs sont du Rust sûr sans dépendance à auditer |

**Ce qui n'est pas défendu :** un programme lu proprement n'est pas un programme vérifié. Rien ici ne contrôle les limites d'une machine, les longueurs d'outil ni les collisions, et les lecteurs n'ont pas eu de revue indépendante. Signale une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Crate | Points d'entrée | Résultat |
|---|---|---|
| `eh3d-parser-gerber` | `parse(text)`, `decode_coordinate(text, format)` | `GerberFile` : unités, format, ouvertures, macros, attributs, `commands`, `ended`, `diagnostics` |
| `eh3d-parser-excellon` | `parse(text)`, `decode_coordinate(...)` | `ExcellonFile` : unités, suppression des zéros, `tools`, `drills`, `slots`, `routes`, `ended`, `diagnostics` |
| `eh3d-parser-gcode` | `parse(text, dialect)`, puis `interpret(&program)` | `Program` (blocs) et `Trace` (segments en millimètres, boîte englobante, longueurs de coupe et de rapides) |

Les trois partagent `Diagnostic { line, severity, message }` avec `Severity` `Info`, `Warning` ou `Error`, et `is_clean()` pour savoir si un `Error` a été trouvé. Il n'y a ni programme en ligne de commande, ni accès réseau, ni accès aux fichiers : l'appelant passe le texte et reçoit les données.

## ⚙️ Configuration

Les lecteurs n'ont pas de fichier de configuration. Ce que choisit l'appelant et les limites fixes sont les suivants :

| Valeur | Plage | Où |
|---|---|---|
| Dialecte G-code | `Grbl`, `Marlin` ou `PlanetCnc` | le second argument de `parse` |
| Longueur d'une ligne G-code | jusqu'à 512 caractères ; le reste est coupé et signalé | `MAX_LINE_CHARS` dans `crates/eh3d-parser-gcode/src/lib.rs` |
| Unités | prises dans le fichier (`MO` en Gerber, `METRIC` ou `INCH` en Excellon, `G20`/`G21` en G-code) ; la sortie G-code est toujours en millimètres | les lecteurs |
| Format des coordonnées | pris dans le fichier (`FS` en Gerber, `FILE_FORMAT=i:d` ou l'en-tête en Excellon) ; une valeur plus large est une erreur | les lecteurs |
| Suppression des zéros | en tête ou en fin, selon le fichier | les lecteurs |

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.75 ou plus récente).

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

## 🧪 Tests

La suite compte 42 tests (`cargo test`, exécutés sous Windows et Linux dans la CI) : 12 pour Gerber (plus 1 dans le module), 11 pour Excellon et 18 pour G-code. Ils vérifient qu'une couche de cuivre typique et un fichier métrique typique sont lus en entier, les coordonnées omises, les deux suppressions de zéros, les rainures, les parcours, les pouces en millimètres, chaque commande connaissant sa ligne, les arcs par centre et par rayon, les cercles complets, l'avance et le mouvement modaux, les sommes de contrôle Marlin, les fonctions non prises en charge signalées au lieu d'être devinées, et les entrées endommagées : charabia, préfixes coupés et lignes très longues ne provoquent jamais de panique. La CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements en erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` (le workspace), dans `eh3d.project.json` et, via le workspace, dans chacun des trois crates ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). Les versions sont publiées sur GitHub ; les crates s'utilisent depuis ce dépôt et ne sont pas sur crates.io.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-CORE-LIBS" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-CORE-LIBS/
├── crates/
│   ├── eh3d-parser-gerber/                      lecteur Gerber RS-274X
│   ├── eh3d-parser-excellon/                    lecteur de fichiers de perçage Excellon
│   └── eh3d-parser-gcode/                       lecteur et interpréteur modal de G-code
├── docs/                                        READERS.md, ce que promet chaque lecteur et ce qu'il ne fait pas, et les données des deux images
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* Les lecteurs n'ont été essayés qu'avec des fichiers de test, pas avec une collection de vrais fichiers de production.
* La compensation de rayon d'outil (`G41`/`G42`) et les cycles fixes (`G81`..`G89`) sont des erreurs ; les corrections de longueur d'outil, les cycles de palpage, `G28`/`G30` et les systèmes de coordonnées de travail sont lus mais leur effet sur la position n'est pas suivi ; arcs seulement dans le plan XY.
* HP-GL, le format Bungard et la géométrie 2D (décalages et booléennes) ne sont pas construits.
* Aucun programme de la suite n'utilise encore les lecteurs, et les lecteurs n'ont pas eu de revue indépendante.

**Ensuite**

* Essayer les trois lecteurs sur une collection de vrais fichiers et corriger ce qu'ils révèlent.
* Construire la géométrie 2D (décalages et booléennes) et les lecteurs HP-GL et Bungard.
* Faire utiliser les lecteurs par EH3D-DRILLPRO et les programmes de FAO à la place des leurs.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON) : le conteneur de projet et les types de machine de la suite ; publié
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO) : le programme de perçage ; il a aujourd'hui son propre lecteur Excellon
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM) : prévu : fraisage d'isolation à partir du lecteur Gerber
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER) : prévu : simulation du G-code que ces lecteurs interprètent

## 📚 Documentation

* [Ce que promet chaque lecteur](docs/READERS.md) : les garanties, les diagnostics et ce qui n'est pas fait.
* [Journal des modifications](CHANGELOG.md)

## 🤝 Contribuer et communauté

* Avant une pull request, exécute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit réussir. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : une modification du texte se fait dans les huit fichiers, et les deux images sont dessinées à partir de `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Un lecteur ne doit jamais paniquer ni deviner : un nouveau cas exige un test avec une entrée endommagée, et une fonction non prise en charge est signalée, pas sautée en silence.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
