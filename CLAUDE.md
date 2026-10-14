## Agent skills

### Issue tracker

Issues live in GitHub Issues (`toon-protocol/echo-merchant`), via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-label vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## The AFK factory

`ready-for-agent` is the factory's queue (`docs/agents/triage-labels.md`). When an issue
carries it and its blockers are closed, `.github/workflows/agent-implement.yml` runs
`.sandcastle/agent-implement-issue.ts`, which runs `/mattpocock-skills:implement`, then
`/mattpocock-skills:code-review` against `main` in a fresh session, then the gate, then
pushes `sandcastle/issue-N` and opens a PR labelled `ready-for-human`. A failed run moves
the issue to `needs-triage`. Specs, blocked issues, issues with an open PR and any issue
labelled `wayfinder:*` are skipped (`.sandcastle/ready-issues.ts`). A human merges every PR.

**The gate reads its steps from CI.** The gate is the `run:` steps of the `gate` job
(else the `checks` job) of `.github/workflows/ci.yml` **on `main`**, in order. If there is
no `ci.yml`, no such job, or no runnable step, the gate runs nothing and logs that loudly.
Today the gate is the `gate` job of `.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. The sandbox image must carry cargo, rustfmt and clippy, because the gate drops `uses:` steps such as the toolchain setup.
The rule lives in one place, `gateFromCi` in `.sandcastle/run-gate.ts`, tested by
`.sandcastle/run-gate.test.ts`.

The runner's own commands, from `.sandcastle/`: `npm ci`, `npm test`, `npm run typecheck`.
The runner's only Node manifest is `.sandcastle/package.json`, separate from anything at
the repository root. The sandbox image is the shared
`ghcr.io/toon-protocol/sandcastle-agent`; this repo has no Dockerfile for it.
`close-linked-issues.yml` is identical in every factory repo. Its source is
`templates/factory/` in `toon-protocol/toon-meta`, so change it there first.
