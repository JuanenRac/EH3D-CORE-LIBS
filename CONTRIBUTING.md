# Contributing to EH3D-CORE-LIBS

Thank you for looking. These readers sit under the CAM programs of the suite, so a wrong reading is worse than a refused file.

1. **A reader never panics and never guesses.** What it cannot understand becomes a diagnostic with the number of the line, and the reading goes on. A change that makes a reader guess needs a very good reason.
2. **Every behaviour has a test**, including the refusals: a coordinate too wide for its format, an arc that cannot exist, a line that is too long, a checksum that does not match.
3. **Before a pull request:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py` must pass.
4. **Texts exist in eight languages** (English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese) wherever the project shows text to people. The README files keep the same structure.
5. Commits are written in English, small and about one thing.
