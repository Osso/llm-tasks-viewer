# llm-tasks-viewer

Dioxus 0.6 desktop app for viewing llm-tasks in real-time.

## Architecture

- `src/state.rs` — Polling logic, TaskDetail struct, change detection
- `src/sidebar.rs` — Task list with status filter pills
- `src/detail.rs` — Task detail: header, deps, event timeline
- `src/main.rs` — Dioxus desktop launch + polling future + layout

## Data Source

- `~/.local/share/llm-tasks/tasks.db` — SQLite via turso (llm-tasks crate)

## Build & Run

```bash
cargo run
```
