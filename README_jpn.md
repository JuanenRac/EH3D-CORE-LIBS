<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CORE-LIBS banner" width="100%">
</p>

# 📖 EH3D-CORE-LIBS

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite のリーダー群：Gerber RS-274X、Excellon ドリルファイル、G-code。壊れたファイルにも耐えます

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さのチェック - 今日動いているもの：** Gerber（RS-274X）レイヤー、Excellon ドリルファイル、そしてモーダルインタープリター付きの G-code（GRBL、Marlin、PlanetCNC の各方言）を読み込む Rust ライブラリで、42 件のテストがあります。パニックすることはなく、理解できなかった内容は行番号付きの診断として報告します。何も描画せず、ツールパスも計算せず、実際の製造用ファイルの集まりではまだ試していません。HP-GL と 2D ジオメトリ（オフセット、ブーリアン）は計画中です。

---

> **安全性（safety）とセキュリティの現状：** これらのライブラリは安全システムではありません。他人から送られてくるファイルを読み、壊れた入力や悪意ある入力でテストされていますが、読み込んだプログラムは検証済みのプログラム**ではありません**。G-code を機械で安全に実行できるかどうかをここで判断することはなく、これらで機械を制御したこともありません。

## 📋 概要

| | |
|---|---|
| バージョン | `0.0.1` |
| 成熟度 | 足場段階 |
| テスト | 42 |
| 技術スタック | Rust |
| 実機 | 該当なし：ファイルリーダーであり、ここに機械と通信するものはありません |

## 🎯 概要

* **Gerber リーダー**（`eh3d-parser-gerber`、RS-274X）：単位、ゼロ抑制付きの座標フォーマット、アパーチャとマクロ、リージョン、描画コマンド。フォーマットより桁数の多い座標は推測せずに拒否します。
* **Excellon リーダー**（`eh3d-parser-excellon`）：ツール、ドリル穴、スロット（`G85`）、簡単なルート。メートルまたはインチ、先頭または末尾のゼロ抑制に対応します。
* **G-code リーダー兼インタープリター**（`eh3d-parser-gcode`）：ワード、コメント、行番号、Marlin のチェックサム、コントローラーのモーダル状態（単位、距離モード、送り、スピンドル、ツール）。すべての動作は円弧を含めて絶対ミリメートルで出力され、切削長とプログラムのバウンディングボックスが得られます。
* **失敗ではなく診断：** 各リーダーは重大度（情報、警告、エラー）とファイルの行番号付きの行を返し、読み込みを続けます。
* **計画中：** HP-GL と Bungard のリーダー、および 2D ジオメトリ（オフセットとブーリアン）。

## 🧩 EH3D での役割

EH3D-CORE-LIBS は、ファイルをスイートの各プログラム（アイソレーション、ドリル、外形、レーザー、ステンシルの CAM とビューア）が使えるデータに変換します。画面も機械制御もありません。EH3D-COMMON の型を使う予定ですが、現在は3つのリーダーが単独で動作します。

## 📂 リポジトリ構成

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

## 🛠️ ビルドと実行

安定版の Rust ツールチェーンが必要です。

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 関連プロジェクト

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): プロジェクトコンテナとマシンの型。公開済み
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 計画中：Gerber リーダーを使ったアイソレーション切削
* [EH3D-DRILLPRO](https://github.com/JuanenRac/EH3D-DRILLPRO): 計画中：Excellon リーダーを使った穴あけ
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 計画中：これらのリーダーが解釈する G-code のシミュレーション

## 📚 ドキュメントとコミュニティ

* [各リーダーの約束](docs/READERS.md)：保証、診断、そして行わないこと。
* [コントリビュート](CONTRIBUTING.md)、[セキュリティポリシー](SECURITY.md)、[サポート](SUPPORT.md)、[行動規範](CODE_OF_CONDUCT.md)。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使用します。
