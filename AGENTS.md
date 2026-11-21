# AGENTS.md: rules for anyone who changes EH3D-CORE-LIBS

EH3D-CORE-LIBS: the readers and writers of Gerber, Excellon and G-code, the 2D geometry and the pure CAM of the suite. Software licence: GPL-3.0-or-later; every source file starts with its header.

## 1. Before you start

1. Read `docs/STATUS.md`: what exists, what is missing, what the last change decided.
2. Read `docs/SCOPE.md` (what the program is for and what it never decides) and `docs/REQUIREMENTS.md` (the register of requirements and the test that shows each one).
3. Read `docs/NOT_YET.md` before adding an idea: it may be there on purpose.

## 2. Build and test

- The workspace needs a stable Rust toolchain (1.75 or newer, pinned in `rust-toolchain.toml`) and has no dependencies outside the standard library.
- Run before every commit, with the same flags as the CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` and `python tools/make_structure.py --check --readmes`.
- `EH3D_REGENERATE=1 cargo test` rewrites the summaries of the golden files so that a change shows in the diff; read the diff before committing it.
- A heavy build runs in the CI, not on a machine that is short of memory; a change that adds a file adds it to `docs/structure.json` and regenerates the tree and the picture.

## 3. Code rules

- A reader never panics and never guesses. What it cannot understand is a diagnostic with a stable code and, when it has one, the number of its line, and the reader goes on.
- Every input file is hostile until it is shown otherwise: sizes, counts, number formats and nesting are checked before use, and everything is bounded.
- No `unwrap` or `expect` in code that handles an external value. No `unsafe`, and no dependency outside the standard library.
- Every public module, field and variant has a `///` comment: the crates compile with `missing_docs` as an error.
- A diagnostic code is added to `docs/DIAGNOSTICS.md` and to the eight headlines in the same change.
- Every value that is tuned (a limit, a tolerance, a time) is a setting in a configuration file, not a constant in the code; every option of security has a switch with the conservative value as its default.
- Output carries no date, host name or user name that the person did not give.

## 4. What the program must not do

- It does not decide that a program is safe to run and does not check the limits of a machine.
- It does not read or write a file on its own: the callers pass text in and get data out.
- Its documents and its code do not name anything private: no dates, no internal notes, no names of people or of tools that wrote the change.

## 5. Evidence

- A state in `docs/REQUIREMENTS.md` is `validated` only when a person has run the capability on the real machine it names. Nothing is green that has never run on the real thing.
- A requirement names a test that exists; when a requirement changes state, the row and `docs/STATUS.md` change in the same commit, and `crates/eh3d-parser-common/tests/reqs.rs` says whether they agree.

## 6. Documents and languages

- The README exists in eight languages (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal) and every word of the window and of the messages exists in the eight where the program has them.
- The file tree and the pictures are generated from `docs/structure.json` and `docs/architecture.json`; they are not edited by hand. The count of tests in the READMEs is the real one.
- Each change has a line in the `[Unreleased]` part of `CHANGELOG.md`, in English.

## 7. Clean room

- Ideas, flows and behaviours that can be seen from outside may be taken from other programs; their code, their texts, their files and their wording are not copied.
- Formats are learned from public specifications and from files of this project. The files of another program are read, never written.

## 8. Reporting

- A problem with a part of the program is reported with a small file that shows it. Real boards and real machine files are private: do not paste them into a report.
