use std::collections::HashMap;

use dioxus::prelude::*;
use llm_tasks::db::TaskUpdates;

use crate::state::{
    AgentInfo, LogEntry, Project, ProjectScope, SelectedTask, TaskDetail, TaskListItem,
};

const STATUSES: &[&str] = &["pending", "in_progress", "completed"];

#[component]
fn CollapsibleSection(
    title: String,
    class: String,
    header_class: String,
    children: Element,
) -> Element {
    let mut collapsed = use_signal(|| false);
    let chevron = if collapsed() { "▸" } else { "▾" };

    rsx! {
        div { class: "{class}",
            div {
                class: "{header_class} section-toggle",
                onclick: move |_| collapsed.set(!collapsed()),
                span { class: "section-chevron", "{chevron}" }
                "{title}"
            }
            if !collapsed() {
                {children}
            }
        }
    }
}

fn status_label(s: &str) -> &str {
    match s {
        "in_progress" => "In Progress",
        other => other,
    }
}

fn format_timestamp(ts: &str) -> &str {
    ts.get(..16).unwrap_or(ts)
}

fn spawn_delete(
    project: Project,
    task_id: String,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    mut selected: Signal<Option<SelectedTask>>,
    mut confirming_delete: Signal<bool>,
    mut tasks: Signal<Vec<TaskListItem>>,
) {
    spawn(async move {
        if let Some(db) = crate::state::open_db_for(&project).await {
            let _ = db.delete_task(&task_id).await;
            let refreshed = crate::state::list_tasks_for_scope(&active_scope(), &projects()).await;
            tasks.set(refreshed);
        }
        confirming_delete.set(false);
        selected.set(None);
    });
}

#[component]
fn TaskHeaderActions(
    project: Project,
    task_id: String,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
    confirming_delete: Signal<bool>,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    tasks: Signal<Vec<TaskListItem>>,
) -> Element {
    if editing() {
        return rsx! {};
    }

    rsx! {
        div { class: "header-actions",
            if confirming_delete() {
                span { class: "delete-confirm-text", "Delete?" }
                button {
                    class: "btn-delete-yes",
                    onclick: move |_| {
                        spawn_delete(
                            project.clone(),
                            task_id.clone(),
                            active_scope,
                            projects,
                            selected,
                            confirming_delete,
                            tasks,
                        )
                    },
                    "Yes"
                }
                button {
                    class: "btn-cancel",
                    onclick: move |_| confirming_delete.set(false),
                    "No"
                }
            } else {
                button {
                    class: "btn-edit",
                    onclick: move |_| editing.set(true),
                    "Edit"
                }
                button {
                    class: "btn-delete",
                    onclick: move |_| confirming_delete.set(true),
                    "Delete"
                }
            }
        }
    }
}

#[component]
fn TaskHeader(
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
    confirming_delete: Signal<bool>,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    tasks: Signal<Vec<TaskListItem>>,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let task = &detail.task;
    let status_class = format!("status-badge status-{}", task.status);
    let project = detail.project.clone();

    rsx! {
        div { class: "detail-header",
            div { class: "detail-title-row",
                span { class: "detail-title", "{task.title}" }
                span { class: "detail-id", "{task.id}" }
                span { class: "detail-project", "{project.name}" }
                AgentStatusBadge { project: project.clone(), task_id: task.id.clone(), agent_statuses }
                TaskHeaderActions {
                    project,
                    task_id: task.id.clone(),
                    editing,
                    selected,
                    confirming_delete,
                    active_scope,
                    projects,
                    tasks,
                }
            }
            div { class: "detail-meta-row",
                span { class: "{status_class}", "{status_label(&task.status)}" }
                if task.priority > 0 {
                    span { class: "badge-priority", "P{task.priority}" }
                }
                if let Some(ref assignee) = task.assignee {
                    span { class: "detail-assignee", "@{assignee}" }
                }
                span { class: "detail-timestamp",
                    "created {format_timestamp(&task.created_at)}"
                }
                if task.updated_at != task.created_at {
                    span { class: "detail-timestamp",
                        "updated {format_timestamp(&task.updated_at)}"
                    }
                }
            }
        }
    }
}

async fn persist_task_update(
    project: &Project,
    task_id: &str,
    title: &str,
    description: &str,
    status: &str,
    priority: &str,
    assignee: &str,
) -> Result<(), String> {
    let pri = priority.parse::<u8>().unwrap_or(0);
    let desc = if description.is_empty() {
        None
    } else {
        Some(description)
    };
    let assign = if assignee.is_empty() {
        None
    } else {
        Some(assignee)
    };

    let updates = TaskUpdates {
        title: Some(title),
        description: desc,
        status: Some(status),
        priority: Some(pri),
        assignee: assign,
        ..Default::default()
    };

    let db = crate::state::open_db_for(project)
        .await
        .ok_or("Failed to open database")?;
    db.update_task(task_id, updates, "viewer")
        .await
        .map_err(|e| format!("{e}"))
}

#[component]
fn TextFieldEdit(label: String, value: Signal<String>) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            input {
                r#type: "text",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn TextAreaEdit(label: String, value: Signal<String>) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            textarea {
                rows: "4",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn StatusSelect(status: Signal<String>) -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        div { class: "edit-field",
            label { "Status" }
            div { class: "dropdown",
                div {
                    class: "dropdown-trigger",
                    onclick: move |_| open.set(!open()),
                    span { class: "dropdown-value", "{status_label(&status())}" }
                    span { class: "dropdown-chevron", "▾" }
                }
                if open() {
                    StatusDropdownList { status, open }
                }
            }
        }
    }
}

#[component]
fn StatusDropdownList(status: Signal<String>, open: Signal<bool>) -> Element {
    rsx! {
        div { class: "dropdown-list",
            for s in STATUSES {
                {
                    let val = s.to_string();
                    let is_active = status() == *s;
                    rsx! {
                        div {
                            class: if is_active { "dropdown-item active" } else { "dropdown-item" },
                            onclick: move |_| {
                                status.set(val.clone());
                                open.set(false);
                            },
                            "{status_label(s)}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NumberFieldEdit(label: String, value: Signal<String>, min: String, max: String) -> Element {
    rsx! {
        div { class: "edit-field",
            label { "{label}" }
            input {
                r#type: "number",
                min: "{min}",
                max: "{max}",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn EditFields(
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
) -> Element {
    rsx! {
        TextFieldEdit { label: "Title", value: title }
        TextAreaEdit { label: "Description", value: description }
        div { class: "edit-row",
            StatusSelect { status }
            NumberFieldEdit { label: "Priority", value: priority, min: "0", max: "9" }
            TextFieldEdit { label: "Assignee", value: assignee }
        }
    }
}

fn spawn_save(
    project: Project,
    task_id: String,
    title: Signal<String>,
    description: Signal<String>,
    status: Signal<String>,
    priority: Signal<String>,
    assignee: Signal<String>,
    mut saving: Signal<bool>,
    mut error: Signal<Option<String>>,
    mut editing: Signal<bool>,
    mut selected: Signal<Option<SelectedTask>>,
) {
    spawn(async move {
        saving.set(true);
        error.set(None);
        match persist_task_update(
            &project,
            &task_id,
            &title(),
            &description(),
            &status(),
            &priority(),
            &assignee(),
        )
        .await
        {
            Ok(_) => {
                editing.set(false);
                let sel = selected();
                selected.set(None);
                selected.set(sel);
            }
            Err(e) => error.set(Some(e)),
        }
        saving.set(false);
    });
}

#[component]
fn EditForm(
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let task = &detail.task;
    let project = detail.project.clone();
    let title = use_signal(|| task.title.clone());
    let description = use_signal(|| task.description.clone().unwrap_or_default());
    let status = use_signal(|| task.status.clone());
    let priority = use_signal(|| task.priority.to_string());
    let assignee = use_signal(|| task.assignee.clone().unwrap_or_default());
    let saving = use_signal(|| false);
    let error = use_signal(|| Option::<String>::None);
    let task_id = task.id.clone();

    let on_save = move |_| {
        spawn_save(
            project.clone(),
            task_id.clone(),
            title,
            description,
            status,
            priority,
            assignee,
            saving,
            error,
            editing,
            selected,
        );
    };

    rsx! {
        div { class: "edit-form",
            EditFields { title, description, status, priority, assignee }
            if let Some(err) = error() {
                div { class: "edit-error", "{err}" }
            }
            div { class: "edit-actions",
                button {
                    class: "btn-save",
                    disabled: saving(),
                    onclick: on_save,
                    if saving() { "Saving..." } else { "Save" }
                }
                button {
                    class: "btn-cancel",
                    disabled: saving(),
                    onclick: move |_| editing.set(false),
                    "Cancel"
                }
            }
        }
    }
}

#[component]
fn DepLink(
    project: Project,
    id: String,
    title: String,
    status: String,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    let nav_id = id.clone();
    let nav_project = project.clone();
    let status_class = format!("dep-status dep-status-{status}");

    rsx! {
        span {
            class: "dep-link",
            onclick: move |_| {
                selected.set(Some(SelectedTask {
                    project: nav_project.clone(),
                    task_id: nav_id.clone(),
                }))
            },
            span { class: "{status_class}" }
            "[{id}] {title}"
        }
    }
}

#[component]
fn DependenciesSection(detail: TaskDetail, selected: Signal<Option<SelectedTask>>) -> Element {
    let has_deps = !detail.depends_on.is_empty() || !detail.blocks.is_empty();
    if !has_deps {
        return rsx! {};
    }
    let project = detail.project.clone();

    rsx! {
        CollapsibleSection {
            title: "DEPENDENCIES",
            class: "detail-deps",
            header_class: "deps-header",
            if !detail.depends_on.is_empty() {
                div { class: "dep-group",
                    span { class: "dep-label", "Depends on:" }
                    for (id, title, status) in &detail.depends_on {
                        DepLink {
                            key: "{id}",
                            project: project.clone(),
                            id: id.clone(),
                            title: title.clone(),
                            status: status.clone(),
                            selected,
                        }
                    }
                }
            }
            if !detail.blocks.is_empty() {
                div { class: "dep-group",
                    span { class: "dep-label", "Blocks:" }
                    for (id, title, status) in &detail.blocks {
                        DepLink {
                            key: "{id}",
                            project: project.clone(),
                            id: id.clone(),
                            title: title.clone(),
                            status: status.clone(),
                            selected,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CommentsSection(detail: TaskDetail) -> Element {
    if detail.comments.is_empty() {
        return rsx! {};
    }

    rsx! {
        CollapsibleSection {
            title: "COMMENTS",
            class: "detail-comments",
            header_class: "comments-header",
            for comment in &detail.comments {
                div { class: "comment-row",
                    div { class: "comment-meta",
                        span { class: "comment-actor", "{comment.actor}" }
                        span { class: "comment-time", "{format_timestamp(&comment.created_at)}" }
                    }
                    div { class: "comment-content", "{comment.content}" }
                }
            }
        }
    }
}

#[component]
fn EventTimeline(detail: TaskDetail) -> Element {
    if detail.events.is_empty() {
        return rsx! {};
    }

    rsx! {
        CollapsibleSection {
            title: "EVENTS",
            class: "detail-timeline",
            header_class: "timeline-header",
            for event in &detail.events {
                {
                    let time = event.timestamp.get(11..16).unwrap_or("??:??");
                    let desc = format_event(event);
                    rsx! {
                        div { class: "timeline-row",
                            span { class: "timeline-time", "{time}" }
                            span { class: "timeline-actor", "{event.actor}" }
                            span { class: "timeline-desc", "{desc}" }
                        }
                    }
                }
            }
        }
    }
}

fn format_event(event: &llm_tasks::db::Event) -> String {
    match event.action.as_str() {
        "created" => "created".into(),
        "claimed" => "claimed".into(),
        "closed" => "completed".into(),
        "updated" => {
            let field = event.field.as_deref().unwrap_or("?");
            let new = event.new_value.as_deref().unwrap_or("?");
            format!("{field} → {new}")
        }
        other => other.into(),
    }
}

#[component]
fn AgentStatusBadge(
    project: Project,
    task_id: String,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let statuses = agent_statuses.read();
    let key = crate::state::task_key(&project, &task_id);
    let Some(agent) = statuses.get(&key) else {
        return rsx! {};
    };

    rsx! {
        span { class: "agent-badge agent-running",
            "{agent.name}"
        }
    }
}

#[component]
fn AgentLogSection(project: Project, task_id: String) -> Element {
    let mut entries = use_signal(Vec::<LogEntry>::new);
    let tid = task_id.clone();

    use_future(move || {
        let tid = tid.clone();
        let project = project.clone();
        async move {
            loop {
                let logs = crate::state::read_agent_log(&project, &tid, 50);
                entries.set(logs);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    });

    let logs = entries.read();
    if logs.is_empty() {
        return rsx! {};
    }

    rsx! {
        CollapsibleSection {
            title: "AGENT LOG",
            class: "detail-agent-log",
            header_class: "agent-log-header",
            div { class: "agent-log-entries",
                for entry in logs.iter() {
                    { render_log_entry(entry) }
                }
            }
        }
    }
}

fn render_log_entry(entry: &LogEntry) -> Element {
    let time = entry.timestamp.get(11..19).unwrap_or("");
    let kind_class = format!("log-entry log-entry-{}", entry.kind);
    let text = truncate_log_text(&entry.text, 500);

    rsx! {
        div { class: "{kind_class}",
            if !time.is_empty() {
                span { class: "log-time", "{time}" }
            }
            span { class: "log-kind", "{entry.kind}" }
            span { class: "log-text", "{text}" }
        }
    }
}

fn truncate_log_text(text: &str, max: usize) -> &str {
    if text.len() <= max {
        text
    } else {
        &text[..max]
    }
}

#[component]
pub fn Detail(
    detail: Signal<Option<TaskDetail>>,
    selected: Signal<Option<SelectedTask>>,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    tasks: Signal<Vec<TaskListItem>>,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let editing = use_signal(|| false);
    let confirming_delete = use_signal(|| false);

    let Some(d) = detail() else {
        return rsx! {
            div { class: "detail-empty", "Select a task" }
        };
    };

    let task_id = d.task.id.clone();
    let project = d.project.clone();

    rsx! {
        div { class: "detail-area",
            TaskHeader {
                detail: d.clone(),
                editing,
                selected,
                confirming_delete,
                active_scope,
                projects,
                tasks,
                agent_statuses,
            }
            if editing() {
                EditForm { detail: d.clone(), editing, selected }
            } else {
                if let Some(ref desc) = d.task.description {
                    div { class: "detail-description", "{desc}" }
                }
            }
            DependenciesSection { detail: d.clone(), selected }
            CommentsSection { detail: d.clone() }
            EventTimeline { detail: d }
            AgentLogSection { project, task_id }
        }
    }
}
