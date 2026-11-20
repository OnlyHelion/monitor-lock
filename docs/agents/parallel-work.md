# Parallel work: worktrees, builds and briefs

For runs that put tickets in `git worktree`s and work them at the same time (implement-spec).

## One Cargo target dir per worktree

Every worktree builds the same package, so a shared `CARGO_TARGET_DIR` means they overwrite each other's `target/debug/deps/monitor-lock-*.exe`. A test run then executes another ticket's code. (2026-11-20: ticket #11 saw ticket #12's half-finished `ico_file` test fail.)

Give each worktree its own dir, inside the main checkout's ignored `target/`:

```powershell
$env:CARGO_TARGET_DIR = "K:\monitor-lock\target\wt\<ticket>"   # e.g. ...\target\wt\11
```

The first build in each one compiles the dependencies (a few minutes). Every build after that is incremental. Cargo's "Blocking waiting for file lock" only appears when two commands share a dir. Within one worktree that's normal: wait for it.

## Briefs and handoff files

The read tool refuses files over 20 KB.
- Give each implementer its own ticket body plus a pointer to the spec issue. The spec (#8 was 13 KB) is reference, read on demand.
- Exploration notes go in `_notes/<ticket>.md` at the repo root (root `_*` is gitignored; not `%TEMP%`, which gets cleared), one file per ticket, each under 15 KB.
- Ledger entries: one line each, at most about 300 characters. Detail (TDD steps, test lists, deviations) goes in the archive file the entry points to.

## Checks before merging a ticket back

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (the same three CI runs: `.github/workflows/ci.yml`).
