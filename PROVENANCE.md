# Source attribution

The image-builder implementation originated in `crates/image` of
[anreonyr/sqware](https://github.com/anreonyr/sqware) at
`aaed018fb2c6b9088f2d06fb6a8d5e23f984a21a`.
The upstream history and original attribution remain authoritative.

That snapshot contained no project-level license file. This extraction does not
add a license grant or relicense that source. Preserve existing source and
third-party notices. Subsequent changes are recorded in this repository's Git
history.

The CLI, build library, tests, examples and workflow contract were imported from
[shuyra-arche/image-builder](https://github.com/shuyra-arche/image-builder/tree/82f81b121a9c8f0590ceceb51c77fa0369355217)
at `82f81b121a9c8f0590ceceb51c77fa0369355217`. That repository retains the
intermediate history. The CLI now lives in `src/` and `tests/cli/`; the library,
its tests and examples live in `crates/build/`. This consolidation adds no license
grant. Existing `acceptance/source-files.json` attribution remains unchanged.
