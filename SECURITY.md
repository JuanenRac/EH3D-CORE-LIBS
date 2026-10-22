# Security policy

EH3D-COMMON reads files that other people may send. The container refuses entry names that could leave its folder, duplicate entries, oversized entries, too many entries, and any entry that does not match its SHA-256 record. These checks are tested.

What this library does **not** do: it does not sign or encrypt a project, so a package does not prove who wrote it, and whoever can rewrite the entries can rewrite the integrity record too.

## Reporting a problem

Please report a vulnerability privately to **electrohobby3d@gmail.com**, with the package or the steps that show it. Do not open a public issue for it. You will get an answer as soon as possible; this is a one-person project.
