# EH3D-CORE-LIBS: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| A reader of HP-GL and of the Bungard plot files | The programs that need it have their own readers and the formats are small | a program of the suite is moved to a shared reader and the differential test is written |
| Cutter compensation and canned cycles in G-code | They move the origin or repeat moves, so reading them wrongly would misread a program; they are reported as unsupported today | a rule for them is agreed and a test compares the trace with a known program |
| Moving the other Gerber and Excellon readers to these | Two pairs of readers stay and two differential tests keep them in agreement; moving needs the corpus of boards to pass on both | the corpus passes on the reader that would be replaced |
| A reader that repairs a damaged file | A damaged file is reported with its line and read as far as it can be; repairing it would guess | a person asks for a repair that a test can show to be exact |
| Publication on crates.io | The crates are used from this repository and the suite pins them by path or by commit | a program outside the suite needs them |
| Writing G-code from the pure CAM | The CAM crate returns paths in millimetres and the programs decide how to write them | the programs agree on one writer and one header |
