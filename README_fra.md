<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Les lecteurs de l'EH3D Fabrication Suite : Gerber RS-274X, fichiers de perçage Excellon et G-code, tolérants aux fichiers abîmés

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** des bibliothèques Rust qui lisent des couches Gerber (RS-274X), des fichiers de perçage Excellon et du G-code avec un interpréteur modal (dialectes GRBL, Marlin et PlanetCNC), testées avec 42 tests. Elles ne paniquent jamais et rapportent ce qu'elles n'ont pas compris sous forme de diagnostics avec numéro de ligne. Elles ne dessinent rien, ne calculent aucune trajectoire et n'ont pas encore été essayées sur une collection de vrais fichiers de production ; le HP-GL et la géométrie 2D (décalages, booléens) sont prévus.

---

> **État de la sûreté et de la sécurité :** ces bibliothèques ne sont pas un système de sûreté. Elles lisent des fichiers que d'autres peuvent envoyer et sont testées avec des entrées abîmées et hostiles, mais un programme lu n'est **pas** un programme vérifié : rien ici ne décide si un G-code peut être exécuté sans danger sur une machine, et aucune machine n'a été pilotée avec.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 42 |
| Pile technique | Rust |
| Sur matériel réel | Sans objet : des lecteurs de fichiers, rien ici ne parle à une machine |

## 🎯 Vue d'ensemble

* Le **lecteur Gerber** (`eh3d-parser-gerber`, RS-274X) : unités, format des coordonnées avec sa suppression des zéros, ouvertures et macros, régions et commandes de tracé. Une coordonnée plus large que son format est refusée plutôt que devinée.
* Le **lecteur Excellon** (`eh3d-parser-excellon`) : outils, trous, rainures (`G85`) et routes simples, en métrique ou en pouces, avec suppression des zéros initiaux ou finaux.
* Le **lecteur et interpréteur de G-code** (`eh3d-parser-gcode`) : mots, commentaires, numéros de ligne et sommes de contrôle Marlin, et l'état modal d'un contrôleur (unités, mode de distance, avance, broche, outil). Chaque mouvement sort en millimètres absolus, arcs compris, avec la longueur de la coupe et la boîte englobante du programme.
* **Des diagnostics plutôt que des échecs :** chaque lecteur renvoie des lignes avec une gravité (information, avertissement, erreur) et le numéro de ligne du fichier, et continue sa lecture.
* **Prévu :** le lecteur HP-GL et Bungard et la géométrie 2D (décalages et booléens).

## 🧩 Rôle dans EH3D

EH3D-CORE-LIBS transforme des fichiers en données pour les programmes de la suite (FAO d'isolation, de perçage, de contour, laser et pochoir, et les visionneuses). Il n'a ni écran ni commande de machine. Il est prévu qu'il utilise les types d'EH3D-COMMON ; aujourd'hui les trois lecteurs sont autonomes.

## 📂 Structure du dépôt

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

## 🛠️ COMPILER ET EXÉCUTER

Il faut une chaîne d'outils Rust stable.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): le conteneur de projet et les types de machine ; publié
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): prévu : fraisage d'isolation à partir du lecteur Gerber
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): prévu : perçage à partir du lecteur Excellon
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): prévu : simulation du G-code que ces lecteurs interprètent

## 📚 Documentation et communauté

* [Ce que promet chaque lecteur](docs/READERS.md) : les garanties, les diagnostics et ce qui n'est pas fait.
* [Contribuer](CONTRIBUTING.md), [politique de sécurité](SECURITY.md), [support](SUPPORT.md) et [code de conduite](CODE_OF_CONDUCT.md).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
