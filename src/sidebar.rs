use dioxus::prelude::*;

use crate::state::{Project, ProjectScope, SelectedTask, TaskListItem};

const STATUSES: &[&str] = &["pending", "in_progress", "completed"];

#[component]
fn ProjectPicker(projects: Signal<Vec<Project>>, active_scope: Signal<ProjectScope>) -> Element {
    let mut open = use_signal(|| false);
    let active_name = active_scope().label();

    rsx! {
        div { class: "project-picker",
            div {
                class: "dropdown",
                div {
                    class: "dropdown-trigger",
                    onclick: move |_| open.set(!open()),
                    span { class: "dropdown-value", "{active_name}" }
                    span { class: "dropdown-chevron", "▾" }
                }
                if open() {
                    ProjectDropdownList { projects, active_scope, open }
                }
            }
        }
    }
}

#[component]
fn ProjectDropdownList(
    projects: Signal<Vec<Project>>,
    active_scope: Signal<ProjectScope>,
    open: Signal<bool>,
) -> Element {
    let active_name = active_scope().label();

    rsx! {
        div { class: "dropdown-list",
            div {
                class: if active_name == "All projects" { "dropdown-item active" } else { "dropdown-item" },
                onclick: move |_| {
                    active_scope.set(ProjectScope::All);
                    open.set(false);
                },
                "All projects"
            }
            for proj in projects() {
                {
                    let is_active = proj.name == active_name;
                    let name = proj.name.clone();
                    rsx! {
                        div {
                            class: if is_active { "dropdown-item active" } else { "dropdown-item" },
                            onclick: move |_| {
                                let p = projects().into_iter().find(|p| p.name == name);
                                if let Some(project) = p {
                                    active_scope.set(ProjectScope::Single(project));
                                }
                                open.set(false);
                            },
                            "{proj.name}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusFilter(filter: Signal<Option<String>>) -> Element {
    rsx! {
        div { class: "filter-pills",
            button {
                class: if filter().is_none() { "pill active" } else { "pill" },
                onclick: move |_| filter.set(None),
                "All"
            }
            for s in STATUSES {
                {
                    let status = s.to_string();
                    let label = match *s {
                        "in_progress" => "In Progress",
                        other => other,
                    };
                    rsx! {
                        button {
                            class: if filter().as_deref() == Some(&status) { "pill active" } else { "pill" },
                            onclick: move |_| filter.set(Some(status.clone())),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TaskRow(
    item: TaskListItem,
    is_active: bool,
    selected: Signal<Option<SelectedTask>>,
    show_project: bool,
) -> Element {
    let selection = SelectedTask {
        project: item.project.clone(),
        task_id: item.task.id.clone(),
    };
    let status_class = format!("status-dot status-{}", item.task.status);
    let priority_label = if item.task.priority > 0 {
        format!("P{}", item.task.priority)
    } else {
        String::new()
    };

    rsx! {
        div {
            class: if is_active { "task-row active" } else { "task-row" },
            onclick: move |_| selected.set(Some(selection.clone())),
            span { class: "{status_class}" }
            div { class: "task-row-main",
                span { class: "task-row-title", "{item.task.title}" }
                if show_project {
                    span { class: "task-row-project", "{item.project.name}" }
                }
            }
            if !priority_label.is_empty() {
                span { class: "badge-priority", "{priority_label}" }
            }
        }
    }
}

#[component]
pub fn Sidebar(
    tasks: Signal<Vec<TaskListItem>>,
    selected: Signal<Option<SelectedTask>>,
    filter: Signal<Option<String>>,
    projects: Signal<Vec<Project>>,
    active_scope: Signal<ProjectScope>,
) -> Element {
    let filtered: Vec<TaskListItem> = tasks
        .read()
        .iter()
        .filter(|t| match filter().as_deref() {
            Some(s) => t.task.status == s,
            None => t.task.status != "completed",
        })
        .cloned()
        .collect();
    let show_project = matches!(active_scope(), ProjectScope::All);

    rsx! {
        div { class: "sidebar",
            ProjectPicker { projects, active_scope }
            div { class: "sidebar-header", "TASKS" }
            StatusFilter { filter }
            div { class: "sidebar-list",
                for item in filtered {
                    {
                        let is_active = selected()
                            .as_ref()
                            .map(|current| current.project == item.project && current.task_id == item.task.id)
                            .unwrap_or(false);
                        let key = format!("{}::{}", item.project.name, item.task.id);
                        rsx! {
                            TaskRow { key: "{key}", item: item.clone(), is_active, selected, show_project }
                        }
                    }
                }
            }
        }
    }
}
