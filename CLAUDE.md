# Relay

## Architecture

Relay is a Rust cargo workspace, not a single app. Current members:

- `relay-core` — pure engine/domain logic lib crate. No UI or CLI framework dependencies. This is where every real capability lives.
- `relay-cli` — a `clap`-based binary depending on `relay-core`. Must expose **every** capability of the app — nothing is CLI-only or UI-only.
- A UI app (planned, likely Tauri) — will depend on `relay-core` directly (same as the CLI, not by shelling out to the CLI binary) and stay a thin convenience layer with no business logic of its own.

**Rule:** a new capability goes into `relay-core` first, is exposed via `relay-cli`, and only then (optionally) surfaced in the UI. Don't let logic leak into `main.rs` or a future UI backend. Everything must be doable from the CLI — the UI can never outpace it.

The whole workspace is one git repo at this root (`relay/`). It was previously two separate, uncommitted git repos nested inside `relay-cli/` and `relay-core/` from running `cargo new` in each — those were consolidated on 2026-09-08.

## Reference project: relay-ui

`~/Development/relay-ui` is the current MVP — a working Tauri v2 app (Rust backend + React frontend). It's a separate project/repo, not part of this workspace, and we're not integrating it yet. When it becomes relevant (schemas, data models, how the Tauri backend talks to the frontend, etc.), treat it as a reference for "what's already been figured out and works well" rather than code to reuse directly — this rebuild is a different architecture (core/cli/UI split).

## Working with the user on this repo

The user doesn't know Rust yet and is deliberately using this project to learn it. They want to write as much of the code themselves as possible. Default to acting as a guide/reviewer — explain concepts, point to the right file/function, review code they've written — rather than implementing full features for them. Ask before assuming "write it for me" mode on a given task.

## Process

All Relay work is tracked against a Linear issue in the "Relay" team (`REL-` prefix). Commits should reference the relevant issue ID.
