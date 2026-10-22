# Security policy

EH3D-CORE-LIBS reads files that other people may send: Gerber layers, Excellon drill files and G-code. The readers never panic and never read outside the text they are given, they cut lines that are absurdly long, they refuse coordinates wider than their declared format and they report everything they could not understand as a diagnostic. These behaviours are tested with damaged and hostile input.

What these libraries do **not** do: they do not decide whether a G-code is safe to run on a machine. A program that was read without an error is not a verified program; limits, interlocks and the emergency stop belong to the machine and to the controller that runs the file.

## Reporting a problem

Please report a vulnerability privately to **electrohobby3d@gmail.com**, with the file or the steps that show it (a file that makes a reader panic, loop or allocate without bound is a vulnerability). Do not open a public issue for it. You will get an answer as soon as possible; this is a one-person project.

Only the latest release is supported.
