# Contributing to EH3D-COMMON

Thank you for looking. This library is the base of the other EH3D repositories, so changes here are small and carefully tested.

1. **Format changes need a new format version.** A change that an existing reader cannot understand raises the major version of the format and ships a migration (see `docs/FORMAT.md`).
2. **Every behaviour has a test**, including the refusals: a hostile name, a changed byte, a missing entry.
3. **Before a pull request:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py` must pass.
4. **Texts exist in eight languages** (English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese) wherever the project shows text to people. The README files keep the same structure.
5. Commits are written in English, small and about one thing.
