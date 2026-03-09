# llm-tasks-viewer

Dioxus 0.6 desktop app for viewing llm-tasks in real-time.

Monitors the llm-tasks SQLite database and displays task status, dependencies, and event timelines with live polling and change detection.

## Features

- Real-time task monitoring with automatic polling
- Status filter pills for quick task filtering
- Task detail view with gradient header, visual timeline, and terminal log
- Dependency graph visualization
- Quick status switching from the detail view
- All-projects task browsing

## Requirements

- Rust 2024 edition
- [llm-tasks](../llm-tasks) crate (local dependency)

## Build & Run

```bash
cargo run
```

## Architecture

- `src/main.rs` — Dioxus desktop launch, polling future, layout
- `src/state.rs` — Polling logic, TaskDetail struct, change detection
- `src/sidebar.rs` — Task list with status filter pills
- `src/detail.rs` — Task detail: header, deps, event timeline

## Data Source

SQLite database at `~/.local/share/llm-tasks/tasks.db`, accessed via the llm-tasks crate (turso/libsql).
