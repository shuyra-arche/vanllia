# Shuyra maintenance and contribution rules

These are the shared rules for `shuyra-arche`. Vanllia is their temporary home
until the organization has a `.github` repository; other repositories link here
instead of copying this document.

## Changes

Use one short-lived branch and one pull request for an independently reviewable
purpose. Start new work from the current default branch; the target development
branch name is `main`. Use `feat/`, `fix/`, `refactor/`, `docs/`, `test/` or `chore/`
(or the tool-required `codex/` prefix). Keep architecture changes separate from
repository cleanup, formatting and documentation work.

Use titles such as `docs(repo): describe current build and validation entry points`.
A PR states its purpose, changes, validation and impact. Record actual commands
and results, including failures and work not run. Host tests, target compilation,
QEMU execution and physical hardware establish different claims. Use Draft while
implementation or necessary validation remains unfinished.

Changes enter the main line through PRs. Review the final diff, resolve discussions
and pass applicable real checks before merging. The single-maintainer beta phase
requires no additional GitHub approval; it still requires review. Prefer squash
merging and remove the completed working branch. Do not force-push or delete the
main line, bypass protections, or weaken assertions to obtain a passing result.

Before removing an old branch, record its tip and examine unique changes, PRs,
source pins, gitlinks and workflow references. Preserve valuable unmerged work
and keep published dependency commits fetchable. Squash merge status alone is
not an ancestry test. Never replace all old branches with permanent backup branches.

## Documentation and verification

README describes today's responsibilities, source entry points, prerequisites,
commands and known limits. Put ownership, failure conditions and unsafe premises
beside the API. Keep separate design prose only for enduring contracts and reasons
that code cannot explain. Preserve licenses, notices and source attribution.

Remove obsolete migration narratives, task reports and repeated validation logs
from the current tree; Git history retains them. Check code, tests and publishing
references before deletion. Future work belongs in Issues; change rationale and
verification belong in PRs; raw outputs belong in CI artifacts or release assets.
Do not create empty architecture documents or copy this policy into each repository.

Use the repository's declared toolchain and actual check entry points. Add required
CI checks only after they run successfully; do not add placeholder checks. Keep
Actions permissions minimal and third-party Actions pinned to full commit SHAs.

## Multiple repositories and agents

Publish and validate a provider change first, then update consumer pins and run
the smallest applicable integration. Pin released Git dependencies to full SHAs.
Do not commit personal paths or depend on unmerged working branches as the baseline.
Cluster describes repository/build organization; Unit describes runtime policy.

Give concurrent workers distinct repositories or worktrees. Handoffs state scope,
commit, interface constraints, checks and open problems. Agents follow the same
review and validation rules; another agent's success claim is not test evidence.
Repository-specific `AGENTS.md` files contain only local entry points and exceptions.
Settings changes, merging, closing unrelated work and repository retirement require
appropriate task authorization; no worker may expand its own scope.
