<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Os leitores da EH3D Fabrication Suite: Gerber RS-274X, ficheiros de furação Excellon e G-code, tolerantes a ficheiros danificados

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** bibliotecas em Rust que leem camadas Gerber (RS-274X), ficheiros de furação Excellon e G-code com um intérprete modal (dialetos GRBL, Marlin e PlanetCNC), testadas com 42 testes. Nunca entram em pânico e contam o que não conseguiram perceber como diagnósticos com número de linha. Não desenham nada, não calculam nenhuma trajetória e ainda não foram experimentadas com uma coleção de ficheiros reais de produção; o HP-GL e a geometria 2D (desvios, booleanas) estão planeados.

---

> **Estado de segurança (safety) e de segurança informática:** estas bibliotecas não são um sistema de segurança funcional. Leem ficheiros que outras pessoas podem enviar e são testadas com entradas danificadas e hostis, mas um programa lido **não** é um programa verificado: nada aqui decide se um G-code é seguro para executar numa máquina, e nenhuma máquina foi controlada com isto.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.1` |
| Maturidade | Andaime |
| Testes | 42 |
| Pilha tecnológica | Rust |
| Em hardware real | Não aplicável: são leitores de ficheiros, nada aqui fala com uma máquina |

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Software | 🟡 | Três leitores (Gerber, Excellon, G-code) funcionam como bibliotecas Rust e continuam a ler ficheiros danificados; nenhum programa da suite os usa ainda. |
| Testes | 🟢 | Passam 42 testes (`cargo test`), incluindo entradas danificadas e hostis. |
| Leitor de Gerber | 🟡 | RS-274X: leem-se unidades, formato de coordenadas, aberturas, macros, regiões e comandos de desenho; até agora só ficheiros de teste, não uma coleção de ficheiros reais de produção. |
| Leitor de Excellon | 🟡 | Leem-se ferramentas, furos, ranhuras e percursos simples em métrico ou polegadas; até agora só ficheiros de teste. |
| Leitor de G-code | 🟡 | Um intérprete modal dos dialetos GRBL, Marlin e PlanetCNC devolve milímetros absolutos, arcos incluídos; a compensação de ferramenta e os ciclos fixos são indicados como não suportados. |
| Geometria e HP-GL | 🔴 | A geometria 2D (deslocamentos, operações booleanas) e os leitores de HP-GL e Bungard estão previstos e por construir. |
| Segurança | 🟡 | Pensado para ficheiros enviados por outras pessoas: nada entra em pânico e os problemas voltam como diagnósticos com número de linha; sem revisão independente, e nada aqui avalia se um G-code é seguro de executar. |
| Hardware real | 🟢 | Não se aplica: são leitores de ficheiros, nada aqui fala com uma máquina. |
| Documentação | 🟢 | README em oito idiomas e `docs/READERS.md`, que diz o que cada leitor promete e o que não faz. |

## 🎯 Visão geral

* O **leitor Gerber** (`eh3d-parser-gerber`, RS-274X): unidades, o formato das coordenadas com a sua supressão de zeros, aberturas e macros, regiões e os comandos de desenho. Uma coordenada mais larga do que o seu formato é recusada em vez de adivinhada.
* O **leitor Excellon** (`eh3d-parser-excellon`): ferramentas, furos, ranhuras (`G85`) e percursos simples, em métrico ou polegadas, com supressão de zeros iniciais ou finais.
* O **leitor e intérprete de G-code** (`eh3d-parser-gcode`): palavras, comentários, números de linha e somas de verificação Marlin, e o estado modal de um controlador (unidades, modo de distância, avanço, fuso, ferramenta). Cada movimento sai em milímetros absolutos, arcos incluídos, com o comprimento do corte e a caixa envolvente do programa.
* **Diagnósticos em vez de falhas:** cada leitor devolve linhas com uma gravidade (informação, aviso, erro) e o número da linha do ficheiro, e continua a ler.
* **Planeado:** o leitor HP-GL e Bungard e a geometria 2D (desvios e booleanas).

## 🧩 Papel no EH3D

O EH3D-CORE-LIBS transforma ficheiros em dados para os programas da suite (CAM de isolamento, furação, contorno, laser e stencil, e os visualizadores). Não tem ecrã nem controlo de máquina. Está planeado que use os tipos do EH3D-COMMON; hoje os três leitores funcionam por si.

## 📂 Estrutura do repositório

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

## 🛠️ COMPILAR E EXECUTAR

Precisa de uma cadeia de ferramentas Rust estável.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Projetos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): o contentor de projeto e os tipos de máquina; publicado
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): planeado: fresagem de isolamento a partir do leitor Gerber
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): planeado: furação a partir do leitor Excellon
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): planeado: simulação do G-code que estes leitores interpretam

## 📚 Documentação e comunidade

* [O que cada leitor promete](docs/READERS.md): as garantias, os diagnósticos e o que não se faz.
* [Como contribuir](CONTRIBUTING.md), [política de segurança](SECURITY.md), [suporte](SUPPORT.md) e [código de conduta](CODE_OF_CONDUCT.md).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Consulte [LICENSE](LICENSE). Os desenhos de hardware da suite usam a CERN OHL-S.
