# Dynamic build migration validation

The product entry is `vanllia.toml` and the independent `vanllia` CLI. The root
has no Cargo composition, bootstrap, image adapter or Nushell build/runner entry.
Provider-owned TOML supplies Unit policy and actual Cargo bin identity; the
Deployment is an in-memory selection, never a file consumed by System.

Validation used installed nightly-2026-10-09 and LLVM tools. Historical provider
toolchain pins remain unchanged. All gitlinks and SDK source locks refer to
published commits; development path-patched locks were not committed as release
locks. The CLI's Cargo overlays and their generated locks live under `.vanllia`.

## Actual release artifacts

| Selection | Unit records | Runtime ELF files | Initrd SHA-256 |
| --- | ---: | ---: | --- |
| Default Login + Daemon closure | 11 | 17 | `3120174018fdb856d6653ee851fb3041a2d1b4cb94f830374c1ee82d3f442e89` |
| Pipe roundtrip | 3 | 3 | `d74d172d1a7286760895dc5ba93df9ad855418166034e8ca208efd67591605a6` |

The Kernel is a separate built boot artifact. Debug executables remain on the
host; stripped runtime images and binary Unit records are packaged once. A Rust
inspector using the public bundle/record readers validated the default artifact:
11 identities, 17 files, correct boot image, and Shell as the sole requested
template. System-generated machine inventory is not a fake file in initrd.
The minimal artifact is checked separately for exactly System, Pipe and the
new Rust test Unit. `vanllia test -- --list` discovers only `pipe.roundtrip`.

```sh
vanllia fetch
vanllia check
vanllia plan
vanllia build
vanllia build --unit test.pipe-roundtrip
vanllia test -- --list
```

The full product compiled 17 real Cargo executables without a central image
table. Actual integration caught and fixed two production/dev SDK alias clashes:
new Shell/Login declaration tests now live in isolated workspaces, preserving
production locks. Packaging also caught the difference between initrd files and
resources System generates from the boot ledger; the System package now owns
that metadata. A deterministic fork test reproduced and fixed delayed release
of inherited build-lock descriptors. Earlier failed attempts remain failures,
not passing guest evidence.

## Scoped host verification

- CLI: 37 Rust tests, strict Clippy and formatting passed. Two tests build real
  RV64 artifacts; other cases cover discovery, source locks, overlays, workspace
  editor configuration and typed native lifecycle verdicts.
- Host library: 22 inline and 8 retained integration cases passed, including
  timeout, native nonzero exit, cleanup failures and inherited lock release.
- System: actual RV64 executable and two new host tests of the real catalog,
  binary decoder and startup selector passed. Existing server warnings remain.
- Each provider's new declaration contract tests were reviewed independently
  against public exact Unit revisions and formal locks. System, Driver and
  Session intake PRs retain their stated verification boundaries.
- Kernel event rings and hot-path writes are removed. RV64 default/semihosting
  checks and the native crash-export test passed; exceptions and backtraces stay.

## Runtime boundary

No QEMU, guest, SMP or performance acceptance was run. Typed event delivery,
native task exits and final resource reclamation still need authorized guest
acceptance. The new runner is managed and records headless serial evidence.
Boot firmware must be configured with an explicit file/URL, SHA-256 and length;
these build artifacts do not bundle a firmware binary. No firmware was silently
substituted. The old scenarios and tests are not migrated; retained composition
test sources, evidence scripts and earlier documents are historical material.

## Draft review chain

All changes are Drafts; none were merged by this work:

- [CLI and host builder](https://github.com/shuyra-arche/image-builder/pull/2),
  [System](https://github.com/shuyra-arche/system/pull/1),
  [Unit records](https://github.com/shuyra-arche/unit/pull/1),
  [initrd format](https://github.com/shuyra-arche/env/pull/2),
  [kernel event log removal](https://github.com/shuyra-arche/kernel/pull/4).
- [Pipe](https://github.com/shuyra-arche/pipe/pull/1),
  [Shell](https://github.com/shuyra-arche/shell/pull/8),
  [Daemon](https://github.com/shuyra-arche/daemon/pull/2).
- [Driver intake](https://github.com/shuyra-arche/driver/pull/6):
  [Hub](https://github.com/shuyra-arche/driver-hub/pull/1),
  [Router](https://github.com/shuyra-arche/driver-router/pull/3),
  [UART](https://github.com/shuyra-arche/driver-uart/pull/2),
  [RTC](https://github.com/shuyra-arche/driver-rtc/pull/3).
- [Session intake](https://github.com/shuyra-arche/session/pull/4):
  [Login](https://github.com/shuyra-arche/session-login/pull/2),
  [Terminal](https://github.com/shuyra-arche/session-terminal/pull/3),
  [Account](https://github.com/shuyra-arche/session-account/pull/1).
