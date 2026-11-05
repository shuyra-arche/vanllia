# Entry validation record

Historical record for Draft #6. Its commands below describe that tree; the
current ownership layout and single discovery command are recorded in
[acceptance-ownership.md](acceptance-ownership.md).

Candidate base: `2643ee792b9fc791e24bbfc3a8efedc1f83fd69a` (Vanllia Draft #5).
The main branch observed at checkout was
`9edc868ead5a765d23eb1ff645285fc476f92dbf`.
Kernel remains `3aa0a155f825fbf377fc8d04009472ca67cbbf8c`.
Frozen legacy source remains `94e114614bec53424ac55e640fc80cf19a3e7107`.

Host: Linux x86_64, Python 3.12.14, Nushell 0.107.0.

| Check | Result |
| --- | --- |
| `python3 -B -m unittest discover -s scripts/tests -v` | 22 passed |
| `python3 -B -m unittest discover -s acceptance/tests/device-recovery -v` | 126 passed |
| `python3 -B -m unittest discover -s acceptance/tests/daemon-recovery -v` | 80 passed |
| `python3 -B -m unittest discover -s acceptance/tests/memory-transfer -v` | 77 passed |
| Old/new argument tables, 8 combinations | Equal after substituting the intended explicit firmware path |
| Python blob identity against `source-files.json` | 54 unchanged; only Shell idle Kernel-path input adapted |
| qtest final verdict suffix against frozen source | Equal after relocating checker paths |
| Product/provider/Cargo diff against base | No changes |

The 22 entry tests run the actual Nu scripts in a temporary skeleton repository
without any legacy checkout. Cargo, Git metadata and QEMU executable calls are
local stubs. They cover actual-path selection (including spaces), stdout-only
image result parsing, Cargo executable filtering, profile validation, both build
failure exits, regular-file checks, firmware validation, board-only omissions,
serial uniqueness, argument defaults/overrides, root cwd, new image invocation,
Kernel workspace selection, raw health failure, unique entry-task success,
frame-leak rejection, relative input freezing and failed-boot evidence retention.
The original Python suites use synthetic traces and local fixture sockets.

The source argument comparison covered normal and board-only modes, serial
override, IOMMU and one/two endpoint disks, held NBD, qtest socket, memory/SMP/seed
overrides, disabled icount, semihosting, GDB and extra arguments. These were
argument-table calls only. No QEMU instance was started. A separate raw-stream
probe observed its first output chunk before a two-second host stub finished,
checking the live-output mechanism used by the runner exit-code fix.

No real product, Kernel or firmware compilation was performed in this slice.
No boot, QEMU, SMP, interactive Shell, idle, firmware equivalence or guest
acceptance result is claimed. The repository still pins nightly-2026-09-04;
the environment's installed nightly-2026-10-09 was not used to claim a compiler
gate. Follow the pinned toolchain setup before a real build.

## Independent model group

Fresh OpenCode Go `deepseek-v4.1-flash` sessions for this PR only:

- namer: `vanllia-boot-namer-422e3cf0-1907-4eea-83bd-7fc2d10622a6`;
- worker: `vanllia-boot-worker-4f00f856-a307-4ade-8692-20752df65756`;
- reviewer: `vanllia-boot-reviewer-1fa9c37c-6f54-4a78-b106-6d43aa75c6ac`.

The worker supplied the implementation and initial tests. The coordinator
corrected Nu syntax and observed failures, added transport/verdict negative
tests, and recorded the firmware investigation. The independent reviewer read
the final full scripts, tests, documentation and old/new diffs and returned
**READY for staged/draft delivery**. It did not run tools itself or independently
claim runtime evidence. Its final response ID was
`chatcmpl-RAcdcEKT8JKXAiyrGks7jhyC`.

After that review, this validation record is added and all staged files receive
the whitespace gate before commit. Publication is draft-only; no merge or
archive is authorized by this result.
