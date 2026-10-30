<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Los lectores de la EH3D Fabrication Suite: Gerber RS-274X, ficheros de taladrado Excellon y G-code, tolerantes con los ficheros dañados

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** tres bibliotecas de Rust sin dependencias que leen capas Gerber (RS-274X), ficheros de taladrado Excellon y G-code (un intérprete modal de los dialectos GRBL, Marlin y PlanetCNC). 42 pruebas se ejecutan en un ordenador, con entradas dañadas y hostiles incluidas. Los lectores nunca entran en pánico e informan de lo que no pudieron entender como diagnósticos con número de línea. No dibujan nada, no calculan ninguna trayectoria y no se han probado con una colección de ficheros de producción reales; ningún programa de la suite los usa todavía y nada de aquí se ha ejecutado en una máquina.

---

> **Estado de seguridad (safety) y de seguridad informática:** estas bibliotecas no son un sistema de seguridad (safety). Leen ficheros que pueden enviar otras personas y se prueban con entradas dañadas y hostiles, pero un programa que se ha leído **no** es un programa verificado: nada de aquí decide si un G-code es seguro para ejecutarlo en una máquina, y con ellas no se ha controlado ninguna máquina.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.2` |
| Madurez | Andamiaje |
| Pruebas | 42 |
| Pila tecnológica | Rust |
| En hardware real | No aplica: lectores de ficheros, nada aquí habla con una máquina |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura](#-estructura-del-repositorio) · [Relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Lector Gerber | 🟡 | RS-274X: se leen unidades, formato de coordenadas, aperturas, macros, regiones y comandos de dibujo; solo con ficheros de prueba hasta ahora, no con una colección de ficheros de producción reales. |
| Lector Excellon | 🟡 | Se leen herramientas, taladros, ranuras y rutas simples en métrico o pulgadas; solo con ficheros de prueba hasta ahora. |
| Lector de G-code | 🟡 | Un intérprete modal de los dialectos GRBL, Marlin y PlanetCNC da milímetros absolutos, arcos incluidos; la compensación de radio y los ciclos fijos se avisan como no soportados. |
| Pruebas | 🟡 | Pasan 42 pruebas (`cargo test`): 12 de Gerber más 1 en el módulo, 11 de Excellon y 18 de G-code, con entradas dañadas y hostiles. |
| Seguridad | 🟡 | Hecho para ficheros que envían otras personas: nada entra en pánico, ningún valor se adivina y los problemas vuelven como diagnósticos con número de línea; sin revisión independiente, y nada aquí juzga si un G-code es seguro de ejecutar. |
| Geometría y HP-GL | 🔴 | La geometría 2D (offsets, booleanas) y los lectores de HP-GL y de Bungard están previstos y sin construir. |
| Uso por otros programas | 🔴 | Ningún programa de la suite depende todavía de los lectores. |
| Hardware real | 🟡 | No aplica: lectores de ficheros, nada aquí habla con una máquina. |
| Documentación | 🟡 | README en ocho idiomas y `docs/READERS.md`, que dice qué promete cada lector y qué no hace. |

## 🎯 Resumen

* El **lector Gerber** (`eh3d-parser-gerber`, RS-274X): unidades, el formato de coordenadas con su supresión de ceros, aperturas (círculo, rectángulo, obround, polígono) y macros, regiones, paso y repetición y los comandos de dibujo. Una coordenada más ancha que su formato se rechaza en lugar de adivinarse.
* El **lector Excellon** (`eh3d-parser-excellon`): herramientas con diámetro, avance y velocidad, taladros, ranuras (`G85`) y rutas simples, en métrico o pulgadas, con supresión de ceros al principio o al final.
* El **lector e intérprete de G-code** (`eh3d-parser-gcode`): palabras, comentarios, números de línea y sumas de comprobación de Marlin, y el estado modal de un controlador (unidades, modo de distancia, avance, husillo, herramienta). Cada movimiento sale en milímetros absolutos, arcos incluidos, con la longitud del corte, la longitud de los rápidos y la caja envolvente del programa.
* **Diagnósticos en lugar de fallos:** cada lector devuelve líneas con una gravedad (información, aviso, error) y el número de línea del fichero, y sigue leyendo.
* **Previsto:** el lector de HP-GL y Bungard y la geometría 2D (offsets y booleanas).

## 🧩 Papel en EH3D

EH3D-CORE-LIBS convierte ficheros en datos para los programas de la suite (CAM de aislamiento, taladrado, contorno, láser y stencil, y los visores). No tiene pantalla ni control de máquina. Está previsto que use los tipos de EH3D-COMMON; hoy los tres lectores van por su cuenta, y EH3D-DRILLPRO todavía tiene su propio lector Excellon.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-CORE-LIBS: quién lo usa, qué contiene y de qué depende" width="100%">
</p>

Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba contra el código.

## ✨ Funciones

* **Cero dependencias:** los tres crates usan solo la biblioteca estándar de Rust.
* **Nunca entra en pánico:** las pruebas dan a cada lector ficheros vacíos, basura binaria, números enormes, comentarios sin cerrar, líneas de miles de caracteres y todos los prefijos de un fichero válido.
* **Sin adivinar:** un valor que no cabe en su formato declarado se rechaza como diagnóstico, no se redondea hasta algo que parece correcto.
* **Cada comando conoce su línea:** las operaciones de Gerber, los taladros de Excellon y los bloques de G-code llevan la línea del fichero de la que vienen.
* **Arcos bien hechos:** el intérprete de G-code maneja arcos con `I`/`J` o con `R`, círculos completos, centros de arco absolutos, y avisa de un arco que no puede existir y lo dibuja recto.
* **Honesto con lo que omite:** las funciones no soportadas (compensación de radio, ciclos fijos) son errores y el bloque no mueve nada; los comandos de sistema `$` de GRBL se omiten con una nota.

## 🔒 Seguridad y protección

Los lectores aceptan texto de cualquiera, así que el diseño asume ficheros descuidados u hostiles. Cada fila tiene pruebas.

| Amenaza | Defensa |
|---|---|
| Un fichero dañado, truncado o binario | sin pánico: cada lector devuelve un resultado para cualquier texto; las pruebas cortan un fichero válido en cada posición y le dan basura |
| Un número demasiado ancho para su formato declarado, o un valor absurdo | se rechaza con un diagnóstico `Error`, nunca se redondea ni se trunca hasta algo verosímil |
| Una sola línea de longitud enorme | el lector de G-code corta una línea a 512 caracteres (`MAX_LINE_CHARS`) y lo avisa |
| Una función de G-code que el intérprete no modela (compensación de radio, ciclos fijos) | un diagnóstico `Error`, y el bloque no mueve nada, de modo que un programa nunca se lee mal en silencio |
| Una línea Marlin corrupta | se verifica la suma de comprobación y una mala se avisa |
| Un arco que no puede existir (el radio es menor que la mitad de la cuerda) | se avisa y se dibuja recto |
| Uso inseguro de memoria | los lectores son Rust seguro sin dependencias que auditar |

**Lo que no se defiende:** un programa leído sin problemas no es un programa verificado. Nada aquí comprueba los límites de una máquina, las longitudes de herramienta ni las colisiones, y los lectores no han tenido revisión independiente. Informa de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Crate | Puntos de entrada | Resultado |
|---|---|---|
| `eh3d-parser-gerber` | `parse(text)`, `decode_coordinate(text, format)` | `GerberFile`: unidades, formato, aperturas, macros, atributos, `commands`, `ended`, `diagnostics` |
| `eh3d-parser-excellon` | `parse(text)`, `decode_coordinate(...)` | `ExcellonFile`: unidades, supresión de ceros, `tools`, `drills`, `slots`, `routes`, `ended`, `diagnostics` |
| `eh3d-parser-gcode` | `parse(text, dialect)`, luego `interpret(&program)` | `Program` (bloques) y `Trace` (segmentos en milímetros, caja envolvente, longitudes de corte y de rápidos) |

Los tres comparten `Diagnostic { line, severity, message }` con `Severity` `Info`, `Warning` o `Error`, y `is_clean()` para preguntar si se encontró algún `Error`. No hay programa de línea de comandos, ni acceso a la red, ni acceso a ficheros: quien llama pasa el texto y recibe los datos.

## ⚙️ Configuración

Los lectores no tienen fichero de configuración. Lo que elige quien llama y los límites fijos son estos:

| Valor | Rango | Dónde |
|---|---|---|
| Dialecto de G-code | `Grbl`, `Marlin` o `PlanetCnc` | el segundo argumento de `parse` |
| Longitud de una línea de G-code | hasta 512 caracteres; el resto se corta y se avisa | `MAX_LINE_CHARS` en `crates/eh3d-parser-gcode/src/lib.rs` |
| Unidades | se toman del fichero (`MO` en Gerber, `METRIC` o `INCH` en Excellon, `G20`/`G21` en G-code); la salida de G-code es siempre en milímetros | los lectores |
| Formato de coordenadas | se toma del fichero (`FS` en Gerber, `FILE_FORMAT=i:d` o la cabecera en Excellon); un valor más ancho es un error | los lectores |
| Supresión de ceros | al principio o al final, según diga el fichero | los lectores |

## 🚀 Inicio rápido

Necesitas una cadena de herramientas estable de Rust (1.75 o posterior).

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

## 🧪 Pruebas

La suite tiene 42 pruebas (`cargo test`, ejecutadas en Windows y Linux en el CI): 12 de Gerber (más 1 dentro del módulo), 11 de Excellon y 18 de G-code. Comprueban que una capa de cobre típica y un fichero métrico típico se leen completos, las coordenadas omitidas, las dos supresiones de ceros, ranuras, rutas, pulgadas a milímetros, cada comando conociendo su línea, arcos por centro y por radio, círculos completos, avance y movimiento modales, sumas de Marlin, funciones no soportadas avisadas en lugar de adivinadas, y entradas dañadas: basura, prefijos cortados y líneas larguísimas nunca entran en pánico. El CI también ejecuta `cargo fmt --check`, `cargo clippy` con advertencias como errores y `python -I tools/ci_validate.py`.

## 🏗️ Compilación y publicación

La versión vive en `Cargo.toml` (el workspace), en `eh3d.project.json` y, a través del workspace, en cada uno de los tres crates; cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). Las versiones se publican en GitHub; los crates se usan desde este repositorio y no están en crates.io.

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-CORE-LIBS" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-CORE-LIBS/
├── crates/
│   ├── eh3d-parser-gerber/                      lector Gerber RS-274X
│   ├── eh3d-parser-excellon/                    lector de ficheros de taladrado Excellon
│   └── eh3d-parser-gcode/                       lector e intérprete modal de G-code
├── docs/                                        READERS.md, lo que promete cada lector y lo que no hace, y los datos de las dos imágenes
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* Los lectores solo se han probado con ficheros de prueba, no con una colección de ficheros de producción reales.
* La compensación de radio de herramienta (`G41`/`G42`) y los ciclos fijos (`G81`..`G89`) son errores; las compensaciones de longitud, los ciclos de palpado, `G28`/`G30` y los sistemas de coordenadas de trabajo se leen pero no se sigue su efecto en la posición; arcos solo en el plano XY.
* HP-GL, el formato de Bungard y la geometría 2D (offsets y booleanas) no están construidos.
* Ningún programa de la suite usa todavía los lectores, y los lectores no han tenido revisión independiente.

**Siguiente**

* Probar los tres lectores con una colección de ficheros reales y corregir lo que muestren.
* Construir la geometría 2D (offsets y booleanas) y los lectores de HP-GL y Bungard.
* Que EH3D-DRILLPRO y los programas CAM usen los lectores en lugar de los suyos.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): el contenedor de proyecto y los tipos de máquina de la suite; publicado
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): el programa de taladrado; hoy tiene su propio lector Excellon
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): previsto: fresado de aislamiento a partir del lector Gerber
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): previsto: simulación del G-code que estos lectores interpretan

## 📚 Documentación

* [Lo que promete cada lector](docs/READERS.md): las garantías, los diagnósticos y lo que no se hace.
* [Registro de cambios](CHANGELOG.md)

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecuta `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `python -I tools/ci_validate.py`: todo debe pasar. Consulta [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio en el texto se hace en los ocho archivos, y las dos imágenes se dibujan a partir de `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Un lector nunca debe entrar en pánico ni adivinar: un caso nuevo necesita una prueba con entrada dañada, y una función no soportada se avisa, no se omite en silencio.
* Informa de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
