## Pointers

- Parallel tickets in worktrees (one Cargo target dir each), brief and ledger sizes: `docs/agents/parallel-work.md`.
- Window / egui 0.37 calls (verified signatures): `docs/agents/egui-0.37.md`.
- Dot-matrix design: images, mockups, renderer: `docs/design/dot-matrix/README.md`.
- Checks (same as CI): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## Agent skills

### Issue tracker

GitHub issues via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels, strings equal to the role names. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
