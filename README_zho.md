<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite 的读取库：Gerber RS-274X、Excellon 钻孔文件和 G-code，能容忍损坏的文件

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性检查 - 今天真正能运行的部分：** Rust 库，可读取 Gerber（RS-274X）图层、Excellon 钻孔文件和 G-code（带模态解释器，支持 GRBL、Marlin 和 PlanetCNC 方言），共有 42 个测试。它们绝不会 panic，并把无法理解的内容作为带行号的诊断报告出来。它们不绘制任何东西，不计算任何刀路，也还没有在一批真实的生产文件上试过；HP-GL 和二维几何（偏移、布尔运算）在计划中。

---

> **安全（safety）与信息安全状态：** 这些库不是安全系统。它们读取别人可能发来的文件，并用损坏和恶意的输入测试过，但读取过的程序**不等于**经过验证的程序：这里没有任何东西能判断一段 G-code 是否可以安全地在机器上运行，也没有用它们控制过任何机器。

## 📋 概览

| | |
|---|---|
| 版本 | `0.0.1` |
| 成熟度 | 脚手架 |
| 测试 | 42 |
| 技术栈 | Rust |
| 真实硬件 | 不适用：只是文件读取库，这里没有任何东西与机器通信 |

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 软件 | 🟡 | 三个读取器（Gerber、Excellon、G 代码）可作为 Rust 库使用，遇到损坏的文件会继续读取；套件中还没有任何程序在用它们。 |
| 测试 | 🟢 | 42 项测试通过（`cargo test`），其中包括损坏和恶意的输入。 |
| Gerber 读取器 | 🟡 | RS-274X：可读取单位、坐标格式、光圈、宏、区域和绘图命令；目前只用测试文件验证，没有用过成批的真实生产文件。 |
| Excellon 读取器 | 🟡 | 可读取公制或英制的刀具、钻孔、槽和简单走刀；目前只用测试文件验证。 |
| G 代码读取器 | 🟡 | 针对 GRBL、Marlin 和 PlanetCNC 方言的模态解释器输出绝对毫米坐标（含圆弧）；刀具补偿和固定循环会被报告为不支持。 |
| 几何与 HP-GL | 🔴 | 二维几何（偏移、布尔运算）以及 HP-GL 和 Bungard 读取器只在计划中，尚未构建。 |
| 安全性 | 🟡 | 为他人发来的文件而设计：不会崩溃，问题以带行号的诊断信息返回；没有独立审查，这里也不判断某段 G 代码是否可以安全运行。 |
| 真实硬件 | 🟢 | 不适用：这里是文件读取器，没有任何部分与机床通信。 |
| 文档 | 🟢 | 八种语言的 README 和 `docs/READERS.md`，其中说明了每个读取器承诺什么、不做什么。 |

## 🎯 概述

* **Gerber 读取器**（`eh3d-parser-gerber`，RS-274X）：单位、带零抑制的坐标格式、光圈和宏、区域以及绘图命令。比其格式更宽的坐标会被拒绝，而不是被猜测。
* **Excellon 读取器**（`eh3d-parser-excellon`）：刀具、钻孔、槽（`G85`）和简单的铣削路径，支持公制或英寸，以及前导或尾随零抑制。
* **G-code 读取器与解释器**（`eh3d-parser-gcode`）：字、注释、行号和 Marlin 校验和，以及控制器的模态状态（单位、距离模式、进给、主轴、刀具）。每个运动都以绝对毫米输出（包括圆弧），并给出切削长度和程序的包围盒。
* **用诊断代替失败：** 每个读取器都返回带严重程度（信息、警告、错误）和文件行号的条目，并继续读取。
* **计划中：** HP-GL 和 Bungard 读取器以及二维几何（偏移和布尔运算）。

## 🧩 在 EH3D 中的角色

EH3D-CORE-LIBS 把文件转换为套件各程序（隔离、钻孔、轮廓、激光和钢网 CAM，以及查看器）所需的数据。它没有界面，也没有机器控制。计划使用 EH3D-COMMON 的类型；目前三个读取器各自独立。

## 📂 仓库结构

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

## 🛠️ 构建与运行

需要稳定版 Rust 工具链。

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 相关项目

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): 项目容器和机器类型；已发布
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 计划中：基于 Gerber 读取器的隔离铣削
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): 计划中：基于 Excellon 读取器的钻孔
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 计划中：对这些读取器所解释的 G-code 进行仿真

## 📚 文档与社区

* [每个读取器的承诺](docs/READERS.md)：保证、诊断以及未做的事情。
* [贡献指南](CONTRIBUTING.md)、[安全政策](SECURITY.md)、[支持](SUPPORT.md)和[行为准则](CODE_OF_CONDUCT.md)。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。参见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
