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
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** três bibliotecas Rust sem dependências que leem camadas Gerber (RS-274X), ficheiros de furação Excellon e G-code (um intérprete modal dos dialetos GRBL, Marlin e PlanetCNC). 42 testes correm num computador, incluindo entradas danificadas e hostis. Os leitores nunca entram em pânico e comunicam o que não conseguiram entender como diagnósticos com número de linha. Não desenham nada, não calculam nenhuma trajetória e não foram experimentados numa coleção de ficheiros de produção reais; nenhum programa da suite os usa ainda e nada aqui foi executado numa máquina.

---

> **Estado de segurança (safety) e de segurança informática:** estas bibliotecas não são um sistema de segurança (safety). Leem ficheiros que outras pessoas podem enviar e são testadas com entradas danificadas e hostis, mas um programa lido **não** é um programa verificado: nada aqui decide se um G-code é seguro para executar numa máquina, e nenhuma máquina foi controlada com elas.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.2` |
| Maturidade | Estrutura inicial |
| Testes | 42 |
| Tecnologias | Rust |
| Em hardware real | Não se aplica: leitores de ficheiros, nada aqui fala com uma máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Leitor Gerber | 🟡 | RS-274X: leem-se unidades, formato das coordenadas, aberturas, macros, regiões e comandos de desenho; até agora só ficheiros de teste, não uma coleção de ficheiros de produção reais. |
| Leitor Excellon | 🟡 | Leem-se ferramentas, furos, ranhuras e percursos simples em métrico ou polegadas; até agora só ficheiros de teste. |
| Leitor de G-code | 🟡 | Um intérprete modal dos dialetos GRBL, Marlin e PlanetCNC dá milímetros absolutos, arcos incluídos; a compensação do raio e os ciclos fixos são comunicados como não suportados. |
| Testes | 🟡 | Passam 42 testes (`cargo test`): 12 de Gerber mais 1 no módulo, 11 de Excellon e 18 de G-code, com entradas danificadas e hostis. |
| Segurança | 🟡 | Feitos para ficheiros enviados por outras pessoas: nada entra em pânico, nenhum valor é adivinhado e os problemas voltam como diagnósticos com número de linha; sem revisão independente, e nada aqui julga se um G-code é seguro de executar. |
| Geometria e HP-GL | 🔴 | A geometria 2D (offsets, booleanas) e os leitores de HP-GL e Bungard estão previstos e por construir. |
| Uso por outros programas | 🔴 | Nenhum programa da suite depende ainda dos leitores. |
| Hardware real | 🟡 | Não se aplica: leitores de ficheiros, nada aqui fala com uma máquina. |
| Documentação | 🟡 | README em oito idiomas e `docs/READERS.md`, que diz o que cada leitor promete e o que não faz. |

## 🎯 Visão geral

* O **leitor Gerber** (`eh3d-parser-gerber`, RS-274X): unidades, o formato das coordenadas com a sua supressão de zeros, aberturas (círculo, retângulo, obround, polígono) e macros, regiões, passo e repetição e os comandos de desenho. Uma coordenada mais larga do que o seu formato é recusada em vez de adivinhada.
* O **leitor Excellon** (`eh3d-parser-excellon`): ferramentas com diâmetro, avanço e velocidade, furos, ranhuras (`G85`) e percursos simples, em métrico ou polegadas, com supressão de zeros à frente ou no fim.
* O **leitor e intérprete de G-code** (`eh3d-parser-gcode`): palavras, comentários, números de linha e somas de verificação Marlin, e o estado modal de um controlador (unidades, modo de distância, avanço, fuso, ferramenta). Cada movimento sai em milímetros absolutos, arcos incluídos, com o comprimento do corte, o comprimento dos rápidos e a caixa envolvente do programa.
* **Diagnósticos em vez de falhas:** cada leitor devolve linhas com uma gravidade (informação, aviso, erro) e o número da linha do ficheiro, e continua a ler.
* **Previsto:** o leitor de HP-GL e Bungard e a geometria 2D (offsets e booleanas).

## 🧩 Papel no EH3D

O EH3D-CORE-LIBS transforma ficheiros em dados para os programas da suite (CAM de isolamento, furação, contorno, laser e stencil, e os visualizadores). Não tem ecrã nem controlo de máquina. Está previsto usar os tipos do EH3D-COMMON; hoje os três leitores são autónomos, e o EH3D-DRILLPRO ainda tem o seu próprio leitor Excellon.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-CORE-LIBS: quem o usa, o que contém e de que depende" width="100%">
</p>

Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Zero dependências:** os três crates usam apenas a biblioteca padrão do Rust.
* **Nunca entra em pânico:** os testes dão a cada leitor ficheiros vazios, lixo binário, números enormes, comentários por fechar, linhas com milhares de carateres e todos os prefixos de um ficheiro válido.
* **Sem adivinhar:** um valor que não cabe no seu formato declarado é recusado como diagnóstico, não arredondado para algo que parece certo.
* **Cada comando conhece a sua linha:** as operações Gerber, os furos Excellon e os blocos G-code levam a linha do ficheiro de onde vêm.
* **Arcos bem feitos:** o intérprete de G-code trata arcos com `I`/`J` ou com `R`, círculos completos, centros de arco absolutos, e comunica um arco que não pode existir desenhando-o reto.
* **Honesto sobre o que salta:** as funções não suportadas (compensação do raio, ciclos fixos) são erros e o bloco não move nada; os comandos de sistema `$` do GRBL são saltados com uma nota.

## 🔒 Segurança e proteção

Os leitores aceitam texto de qualquer pessoa, por isso o desenho pressupõe ficheiros descuidados ou hostis. Cada linha tem testes.

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, truncado ou binário | sem pânico: cada leitor devolve um resultado para qualquer texto; os testes cortam um ficheiro válido em cada posição e dão-lhe lixo |
| Um número demasiado largo para o seu formato declarado, ou um valor absurdo | recusado com um diagnóstico `Error`, nunca arredondado nem truncado para algo plausível |
| Uma única linha de comprimento enorme | o leitor de G-code corta uma linha aos 512 carateres (`MAX_LINE_CHARS`) e di-lo |
| Uma função de G-code que o intérprete não modela (compensação do raio, ciclos fixos) | um diagnóstico `Error`, e o bloco não move nada, pelo que um programa nunca é mal lido em silêncio |
| Uma linha Marlin corrompida | a soma de verificação é verificada e uma errada é comunicada |
| Um arco que não pode existir (o raio é menor do que metade da corda) | comunicado e desenhado reto |
| Uso inseguro de memória | os leitores são Rust seguro sem dependências a auditar |

**O que não é defendido:** um programa lido sem problemas não é um programa verificado. Nada aqui verifica limites de uma máquina, comprimentos de ferramenta ou colisões, e os leitores não tiveram revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Crate | Pontos de entrada | Resultado |
|---|---|---|
| `eh3d-parser-gerber` | `parse(text)`, `decode_coordinate(text, format)` | `GerberFile`: unidades, formato, aberturas, macros, atributos, `commands`, `ended`, `diagnostics` |
| `eh3d-parser-excellon` | `parse(text)`, `decode_coordinate(...)` | `ExcellonFile`: unidades, supressão de zeros, `tools`, `drills`, `slots`, `routes`, `ended`, `diagnostics` |
| `eh3d-parser-gcode` | `parse(text, dialect)`, depois `interpret(&program)` | `Program` (blocos) e `Trace` (segmentos em milímetros, caixa envolvente, comprimentos de corte e de rápidos) |

Os três partilham `Diagnostic { line, severity, message }` com `Severity` `Info`, `Warning` ou `Error`, e `is_clean()` para perguntar se foi encontrado algum `Error`. Não há programa de linha de comandos, nem acesso à rede, nem acesso a ficheiros: quem chama passa o texto e recebe os dados.

## ⚙️ Configuração

Os leitores não têm ficheiro de configuração. O que quem chama escolhe e os limites fixos são estes:

| Valor | Intervalo | Onde |
|---|---|---|
| Dialeto de G-code | `Grbl`, `Marlin` ou `PlanetCnc` | o segundo argumento de `parse` |
| Comprimento de uma linha de G-code | até 512 carateres; o resto é cortado e comunicado | `MAX_LINE_CHARS` em `crates/eh3d-parser-gcode/src/lib.rs` |
| Unidades | tomadas do ficheiro (`MO` em Gerber, `METRIC` ou `INCH` em Excellon, `G20`/`G21` em G-code); a saída de G-code é sempre em milímetros | os leitores |
| Formato das coordenadas | tomado do ficheiro (`FS` em Gerber, `FILE_FORMAT=i:d` ou o cabeçalho em Excellon); um valor mais largo é um erro | os leitores |
| Supressão de zeros | à frente ou no fim, como o ficheiro disser | os leitores |

## 🚀 Início rápido

Precisa de uma toolchain Rust estável (1.75 ou mais recente).

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

## 🧪 Testes

A suite tem 42 testes (`cargo test`, executados em Windows e Linux na CI): 12 de Gerber (mais 1 dentro do módulo), 11 de Excellon e 18 de G-code. Verificam que uma camada de cobre típica e um ficheiro métrico típico são lidos por inteiro, as coordenadas omitidas, as duas supressões de zeros, ranhuras, percursos, polegadas para milímetros, cada comando a conhecer a sua linha, arcos por centro e por raio, círculos completos, avanço e movimento modais, somas de verificação Marlin, funções não suportadas comunicadas em vez de adivinhadas, e entradas danificadas: lixo, prefixos cortados e linhas muito longas nunca entram em pânico. A CI executa também `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão vive em `Cargo.toml` (o workspace), em `eh3d.project.json` e, através do workspace, em cada um dos três crates; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). As versões são publicadas no GitHub; os crates usam-se a partir deste repositório e não estão no crates.io.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-CORE-LIBS" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-CORE-LIBS/
├── crates/
│   ├── eh3d-parser-gerber/                      leitor Gerber RS-274X
│   ├── eh3d-parser-excellon/                    leitor de ficheiros de furação Excellon
│   └── eh3d-parser-gcode/                       leitor e intérprete modal de G-code
├── docs/                                        READERS.md, o que cada leitor promete e o que não faz, e os dados das duas imagens
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* Os leitores só foram experimentados com ficheiros de teste, não com uma coleção de ficheiros de produção reais.
* A compensação do raio da ferramenta (`G41`/`G42`) e os ciclos fixos (`G81`..`G89`) são erros; as correções de comprimento de ferramenta, os ciclos de apalpação, `G28`/`G30` e os sistemas de coordenadas de trabalho são lidos mas o seu efeito na posição não é seguido; arcos só no plano XY.
* O HP-GL, o formato Bungard e a geometria 2D (offsets e booleanas) não estão construídos.
* Nenhum programa da suite usa ainda os leitores, e os leitores não tiveram revisão independente.

**A seguir**

* Experimentar os três leitores numa coleção de ficheiros reais e corrigir o que mostrarem.
* Construir a geometria 2D (offsets e booleanas) e os leitores de HP-GL e Bungard.
* Fazer o EH3D-DRILLPRO e os programas CAM usar os leitores em vez dos seus.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): o contentor de projeto e os tipos de máquina da suite; publicado
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): o programa de furação; hoje tem o seu próprio leitor Excellon
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): previsto: fresagem de isolamento a partir do leitor Gerber
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): previsto: simulação do G-code que estes leitores interpretam

## 📚 Documentação

* [O que cada leitor promete](docs/READERS.md): as garantias, os diagnósticos e o que não é feito.
* [Registo de alterações](CHANGELOG.md)

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Consulte [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` com `tools/make_architecture.py` e `tools/make_structure.py`.
* Um leitor nunca deve entrar em pânico nem adivinhar: um caso novo precisa de um teste com entrada danificada, e uma função não suportada é comunicada, não saltada em silêncio.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatórios: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
