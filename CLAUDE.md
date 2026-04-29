# trading-ig (Rust port) — agent guide

The project's knowledge lives under [`_knowledge/`](_knowledge/).

**Start with [`_knowledge/index.md`](_knowledge/index.md)** — it lists
every knowledge file with a one-line summary so you can load only what
matches your current task.

**Do not read the whole knowledge base by default.** It is split into
focused files specifically so agents can stay narrow. Your task brief
will list the files that apply.

## Quick orientation (only if you have no task brief)

- Reviewing existing code or fixing a bug → `_knowledge/architecture.md`
  + the file matching the area you're touching.
- Implementing a new REST endpoint → `_knowledge/adding_endpoint.md`,
  which lists the other files you need.
- Working on streaming → `_knowledge/api/streaming.md` (Vague 3 only).

## Hard rules (apply always)

- **Never** call `reqwest` directly from a domain module — go through
  `Transport`. (Why: `_knowledge/http_transport.md`.)
- **Never** define a per-domain error type — extend `crate::Error`.
  (Why: `_knowledge/errors.md`.)
- **Never** log credentials or tokens. (Why: `_knowledge/tracing.md`.)
- **Never** hit the real IG API in `cargo test`. (Why:
  `_knowledge/testing.md`.)
- **Always** run `cargo test --all-targets` and `cargo clippy
  --all-targets --no-deps` before declaring a task complete.
