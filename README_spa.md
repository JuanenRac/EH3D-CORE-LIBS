<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Los lectores de la EH3D Fabrication Suite: Gerber RS-274X, ficheros de taladros Excellon y G-code, tolerantes a ficheros dañados

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** bibliotecas en Rust que leen capas Gerber (RS-274X), ficheros de taladros Excellon y G-code con un intérprete modal (dialectos GRBL, Marlin y PlanetCNC), probadas con 42 tests. Nunca entran en pánico y cuentan lo que no pudieron entender como diagnósticos con número de línea. No dibujan nada, no calculan ninguna trayectoria y aún no se han probado con una colección de ficheros reales de producción; HP-GL y la geometría 2D (desplazamientos, booleanas) están previstos.

---

> **Estado de seguridad (safety) y de seguridad informática:** estas bibliotecas no son un sistema de seguridad funcional. Leen ficheros que pueden enviar otras personas y se prueban con entradas dañadas y hostiles, pero un programa leído **no** es un programa verificado: nada aquí decide si un G-code es seguro para ejecutarlo en una máquina, y no se ha controlado ninguna máquina con esto.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.1` |
| Madurez | Andamiaje |
| Pruebas | 42 |
| Pila tecnológica | Rust |
| En hardware real | No aplica: son lectores de ficheros, nada aquí habla con una máquina |

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Software | 🟡 | Tres lectores (Gerber, Excellon, G-code) funcionan como bibliotecas de Rust y siguen leyendo ficheros dañados; todavía ningún programa de la suite los usa. |
| Pruebas | 🟢 | Pasan 42 pruebas (`cargo test`), incluidas entradas dañadas y hostiles. |
| Lector de Gerber | 🟡 | RS-274X: se leen unidades, formato de coordenadas, aperturas, macros, regiones y órdenes de dibujo; hasta ahora solo ficheros de prueba, no una colección de ficheros reales de producción. |
| Lector de Excellon | 🟡 | Se leen herramientas, taladros, ranuras y rutas simples en métrico o pulgadas; hasta ahora solo ficheros de prueba. |
| Lector de G-code | 🟡 | Un intérprete modal de los dialectos GRBL, Marlin y PlanetCNC da milímetros absolutos, arcos incluidos; la compensación de herramienta y los ciclos fijos se notifican como no admitidos. |
| Geometría y HP-GL | 🔴 | La geometría 2D (desplazamientos, booleanas) y los lectores de HP-GL y Bungard están previstos y no construidos. |
| Seguridad | 🟡 | Pensado para ficheros que envían otras personas: nada entra en pánico y los problemas vuelven como diagnósticos con número de línea; sin revisión independiente, y nada aquí juzga si un G-code es seguro de ejecutar. |
| Hardware real | 🟢 | No aplica: son lectores de ficheros, nada aquí habla con una máquina. |
| Documentación | 🟢 | README en ocho idiomas y `docs/READERS.md`, que dice lo que promete cada lector y lo que no hace. |

## 🎯 Descripción general

* El **lector Gerber** (`eh3d-parser-gerber`, RS-274X): unidades, el formato de coordenadas con su supresión de ceros, aperturas y macros, regiones y las órdenes de dibujo. Una coordenada más ancha que su formato se rechaza en lugar de adivinarla.
* El **lector Excellon** (`eh3d-parser-excellon`): herramientas, taladros, ranuras (`G85`) y rutas simples, en métrico o en pulgadas, con supresión de ceros iniciales o finales.
* El **lector e intérprete de G-code** (`eh3d-parser-gcode`): palabras, comentarios, números de línea y sumas de comprobación de Marlin, y el estado modal de un controlador (unidades, modo de distancia, avance, husillo, herramienta). Cada movimiento sale en milímetros absolutos, arcos incluidos, con la longitud del corte y la caja envolvente del programa.
* **Diagnósticos en lugar de fallos:** cada lector devuelve líneas con una gravedad (información, aviso, error) y el número de línea del fichero, y sigue leyendo.
* **Previsto:** el lector HP-GL y Bungard y la geometría 2D (desplazamientos y booleanas).

## 🧩 Papel en EH3D

EH3D-CORE-LIBS convierte ficheros en datos para los programas de la suite (CAM de aislamiento, taladrado, contorno, láser y plantilla, y los visores). No tiene pantalla ni control de máquina. Se prevé que use los tipos de EH3D-COMMON; hoy los tres lectores funcionan por separado.

## 📂 Estructura del repositorio

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

## 🛠️ COMPILAR Y EJECUTAR

Necesitas una cadena de herramientas de Rust estable.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Proyectos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): el contenedor de proyecto y los tipos de máquina; publicado
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): previsto: fresado de aislamiento a partir del lector Gerber
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): previsto: taladrado a partir del lector Excellon
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): previsto: simulación del G-code que interpretan estos lectores

## 📚 Documentación y comunidad

* [Qué promete cada lector](docs/READERS.md): las garantías, los diagnósticos y lo que no se hace.
* [Cómo contribuir](CONTRIBUTING.md), [política de seguridad](SECURITY.md), [soporte](SUPPORT.md) y [código de conducta](CODE_OF_CONDUCT.md).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
