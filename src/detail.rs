use std::collections::HashMap;

use dioxus::prelude::*;

use crate::state::{AgentInfo, Project, ProjectScope, SelectedTask, TaskDetail, TaskListItem};

mod editing;
mod sections;

use sections::{AgentStatusBadge, CommentsSection, DependenciesSection, EventTimeline};

const STATUSES: &[&str] = &["pending", "in_progress", "completed"];

#[component]
pub fn CollapsibleSection(
    title: String,
    class: String,
    header_class: String,
    children: Element,
) -> Element {
    let mut collapsed = use_signal(|| false);
    let chevron = if collapsed() { "▸" } else { "▾" };

    rsx! {
        div { class: "{class}",
            SectionHeader {
                title,
                header_class,
                chevron: chevron.to_string(),
                on_toggle: move |_| collapsed.set(!collapsed()),
            }
            SectionBody { collapsed: collapsed(), children }
        }
    }
}

#[component]
fn SectionHeader(
    title: String,
    header_class: String,
    chevron: String,
    on_toggle: EventHandler,
) -> Element {
    rsx! {
        div {
            class: "{header_class} section-toggle",
            onclick: move |_| on_toggle.call(()),
            span { class: "section-chevron", "{chevron}" }
            "{title}"
        }
    }
}

#[component]
fn SectionBody(collapsed: bool, children: Element) -> Element {
    if collapsed {
        return rsx! {};
    }

    rsx! {
        {children}
    }
}

pub(crate) fn status_label(status: &str) -> &str {
    match status {
        "in_progress" => "In Progress",
        other => other,
    }
}

pub(crate) fn format_timestamp(ts: &str) -> &str {
    ts.get(..16).unwrap_or(ts)
}

pub(crate) fn quick_status_targets(current: &str) -> Vec<&'static str> {
    STATUSES
        .iter()
        .copied()
        .filter(|status| *status != current)
        .collect()
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

    let action = if confirming_delete() {
        rsx! {
            ConfirmDeleteActions {
                project,
                task_id,
                selected,
                confirming_delete,
                active_scope,
                projects,
                tasks,
            }
        }
    } else {
        rsx! { IdleHeaderActions { editing, confirming_delete } }
    };

    rsx! {
        div { class: "header-actions",
            {action}
        }
    }
}

#[component]
fn ConfirmDeleteActions(
    project: Project,
    task_id: String,
    selected: Signal<Option<SelectedTask>>,
    confirming_delete: Signal<bool>,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    tasks: Signal<Vec<TaskListItem>>,
) -> Element {
    rsx! {
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
    }
}

#[component]
fn IdleHeaderActions(editing: Signal<bool>, confirming_delete: Signal<bool>) -> Element {
    rsx! {
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
    let project = detail.project.clone();
    let status_class = format!("status-badge status-{}", task.status);

    rsx! {
        div { class: "detail-header",
            TaskTitleRow {
                detail: detail.clone(),
                editing,
                selected,
                confirming_delete,
                active_scope,
                projects,
                tasks,
                agent_statuses,
            }
            TaskMetaRow { project, detail, editing, selected, status_class }
        }
    }
}

#[component]
fn TaskTitleRow(
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
    let project = detail.project.clone();

    rsx! {
        div { class: "detail-title-row",
            span { class: "detail-title", "{task.title}" }
            span { class: "detail-id", "{task.id}" }
            span { class: "detail-project", "{project.name}" }
            AgentStatusBadge {
                project: project.clone(),
                task_id: task.id.clone(),
                agent_statuses,
            }
            TaskHeaderActions {
                project: project.clone(),
                task_id: task.id.clone(),
                editing,
                selected,
                confirming_delete,
                active_scope,
                projects,
                tasks,
            }
        }
    }
}

#[component]
fn TaskMetaRow(
    project: Project,
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
    status_class: String,
) -> Element {
    let task = &detail.task;

    rsx! {
        div { class: "detail-meta-row",
            span { class: "{status_class}", "{status_label(&task.status)}" }
            MetaQuickSwitch {
                editing: editing(),
                project,
                task_id: task.id.clone(),
                current_status: task.status.clone(),
                selected,
            }
            PriorityBadge { priority: task.priority }
            AssigneeBadge { assignee: task.assignee.clone() }
            span { class: "detail-timestamp", "created {format_timestamp(&task.created_at)}" }
            UpdatedTimestamp {
                created_at: task.created_at.clone(),
                updated_at: task.updated_at.clone(),
            }
        }
    }
}

#[component]
fn MetaQuickSwitch(
    editing: bool,
    project: Project,
    task_id: String,
    current_status: String,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    if editing {
        return rsx! {};
    }

    rsx! {
        editing::StatusQuickSwitch {
            project,
            task_id,
            current_status,
            selected,
        }
    }
}

#[component]
fn PriorityBadge(priority: u8) -> Element {
    if priority == 0 {
        return rsx! {};
    }

    rsx! {
        span { class: "badge-priority", "P{priority}" }
    }
}

#[component]
fn AssigneeBadge(assignee: Option<String>) -> Element {
    let Some(assignee) = assignee else {
        return rsx! {};
    };

    rsx! {
        span { class: "detail-assignee", "@{assignee}" }
    }
}

#[component]
fn UpdatedTimestamp(created_at: String, updated_at: String) -> Element {
    if updated_at == created_at {
        return rsx! {};
    }

    rsx! {
        span { class: "detail-timestamp", "updated {format_timestamp(&updated_at)}" }
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

    let Some(detail) = detail() else {
        return rsx! {
            div { class: "detail-empty", "Select a task" }
        };
    };

    rsx! {
        DetailContent {
            detail,
            editing,
            selected,
            confirming_delete,
            active_scope,
            projects,
            tasks,
            agent_statuses,
        }
    }
}

#[component]
fn DetailContent(
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
    confirming_delete: Signal<bool>,
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    tasks: Signal<Vec<TaskListItem>>,
    agent_statuses: Signal<HashMap<String, AgentInfo>>,
) -> Element {
    let task_id = detail.task.id.clone();
    let project = detail.project.clone();

    rsx! {
        div { class: "detail-area",
            TaskHeader {
                detail: detail.clone(),
                editing,
                selected,
                confirming_delete,
                active_scope,
                projects,
                tasks,
                agent_statuses,
            }
            DetailBody { detail: detail.clone(), editing, selected }
            DependenciesSection { detail: detail.clone(), selected }
            CommentsSection { detail: detail.clone() }
            EventTimeline { detail: detail.clone() }
            crate::chat::AgentLogSection {
                project: project.clone(),
                task_id: task_id.clone(),
                agent_statuses,
            }
            crate::chat::StickyChat { project, task_id, agent_statuses }
        }
    }
}

#[component]
fn DetailBody(
    detail: TaskDetail,
    editing: Signal<bool>,
    selected: Signal<Option<SelectedTask>>,
) -> Element {
    if editing() {
        return rsx! { editing::EditForm { detail, editing, selected } };
    }

    rsx! { TaskDescription { description: detail.task.description.clone() } }
}

#[component]
fn TaskDescription(description: Option<String>) -> Element {
    let Some(desc) = description else {
        return rsx! {};
    };

    rsx! {
        div { class: "detail-description", "{desc}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_status_targets_skip_current_status() {
        assert_eq!(
            quick_status_targets("pending"),
            vec!["in_progress", "completed"]
        );
        assert_eq!(
            quick_status_targets("in_progress"),
            vec!["pending", "completed"]
        );
        assert_eq!(
            quick_status_targets("completed"),
            vec!["pending", "in_progress"]
        );
    }
}
