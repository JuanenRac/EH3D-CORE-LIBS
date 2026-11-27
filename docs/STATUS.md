# EH3D-CORE-LIBS: status

## Current state

EH3D-CORE-LIBS works on a computer: the three readers, the two writers, the raster and shape geometry and the pure CAM are covered by tests that include damaged and hostile input, a corpus of synthetic files with summaries kept next to each file, a second corpus of boards written in the dialects of the common design tools, differential tests against the other readers of the suite, and oracles that do not use the code under test. They have not been tried on a collection of real production files, the files of the writers have not been opened by a viewer or a machine, and the libraries have had no independent review.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 57 requirements: 50 implemented, 1 prototype, 6 planned and 0 validated. By progress: 50 done, 1 partial, 4 pending and 2 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the HP-GL and Bungard reader belongs in this repository or in the program that needs it.
* Whether the crates should use the types of EH3D-COMMON, which would tie their versions together.
* When the two pairs of readers of the suite are unified: both pairs stay today and two differential tests keep them in agreement.

## What it needs from other repositories

* A collection of real production files from several design tools, to read and compare with the summaries.
* A viewer or a machine program to open the files that the writers make.
* `EH3D-COMMON`, if the crates are to use its types.

## Known technical debt and omitted tests

* Every test file is synthetic or written by the tests: no real production file has been read.
* The writers are tested by writing and reading back with the readers of this repository; no other program has opened their files.
* The unification of the readers of the suite is a plan that was not carried out; both pairs stay and two differential tests keep them in agreement.
* The count of tests in the READMEs is that of `cargo test`; it is measured in the CI.
