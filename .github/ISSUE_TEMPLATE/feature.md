---
name: Feature (agent-ready)
about: A backlog item an agent can pick up and finish without supervision
title: "[<ID>] <short imperative title>"
labels: ""
assignees: ""
---

**Order N** · `<ID>` · **<Track>** · phase <n> · size S|M|L

**Goal:** <one sentence: the outcome, not the method>

**Steps**
- [ ] <concrete step; name files, commands, metrics>
- [ ] <…>

**Done when**
- [ ] <observable result: test, CLI output, doc, UI behaviour>
- [ ] `cargo test --release` and `cargo clippy --all-targets -- -D warnings` pass (and web checks if `web/` changed).
- [ ] Relevant `context/` docs updated; non-obvious choices in `context/decisions/`.

**Depends on:** #<n>, … (or `none`; also add native blocked-by links)
**Read first:** `<context/... files the task needs>`

_Agent: follow `CLAUDE.md`. Work on a branch, open a PR that closes this issue._
