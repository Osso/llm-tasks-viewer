use dioxus::prelude::*;

use crate::state::{Project, ProjectScope, SelectedTask, TaskListItem};

const STATUSES: &[&str] = &["pending", "in_progress", "completed"];

#[component]
fn ProjectPicker(
    projects: Signal<Vec<Project>>,
    active_scope: Signal<ProjectScope>,
    tasks: Signal<Vec<TaskListItem>>,
) -> Element {
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
                    ProjectDropdownList { projects, active_scope, open, tasks }
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
    tasks: Signal<Vec<TaskListItem>>,
) -> Element {
    let confirming_delete = use_signal(|| None::<String>);
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
                ProjectDropdownItem {
                    key: "{proj.name}",
                    project: proj.clone(),
                    active_name: active_name.clone(),
                    projects,
                    active_scope,
                    open,
                    tasks,
                    confirming_delete,
                }
            }
        }
    }
}

fn spawn_project_delete(
    project: Project,
    mut active_scope: Signal<ProjectScope>,
    mut projects: Signal<Vec<Project>>,
    mut tasks: Signal<Vec<TaskListItem>>,
    mut confirming_delete: Signal<Option<String>>,
) {
    spawn(async move {
        if crate::state::delete_project_db(&project).is_ok() {
            let current_scope = active_scope();
            let refreshed_projects = crate::state::discover_projects();
            let next_scope = crate::state::normalize_scope(&current_scope, &refreshed_projects);
            let refreshed_tasks =
                crate::state::list_tasks_for_scope(&next_scope, &refreshed_projects).await;

            projects.set(refreshed_projects);
            active_scope.set(next_scope);
            tasks.set(refreshed_tasks);
        }

        confirming_delete.set(None);
    });
}

#[component]
fn ProjectDropdownItem(
    project: Project,
    active_name: String,
    projects: Signal<Vec<Project>>,
    active_scope: Signal<ProjectScope>,
    open: Signal<bool>,
    tasks: Signal<Vec<TaskListItem>>,
    confirming_delete: Signal<Option<String>>,
) -> Element {
    let is_active = project.name == active_name;
    let is_confirming = confirming_delete().as_deref() == Some(project.name.as_str());
    let can_delete = crate::state::can_delete_project_db(&project);
    let select_name = project.name.clone();
    let confirm_name = project.name.clone();
    let delete_project = project.clone();
    let row_class = if is_active {
        "dropdown-item dropdown-item-row active"
    } else {
        "dropdown-item dropdown-item-row"
    };

    rsx! {
        div {
            class: row_class,
            onclick: move |_| {
                confirming_delete.set(None);
                let selected_project = projects().into_iter().find(|candidate| candidate.name == select_name);
                if let Some(project) = selected_project {
                    active_scope.set(ProjectScope::Single(project));
                }
                open.set(false);
            },
            span { class: "dropdown-item-label", "{project.name}" }
            if can_delete {
                div { class: "dropdown-item-actions",
                    if is_confirming {
                        span { class: "dropdown-confirm-text", "Delete?" }
                        button {
                            class: "dropdown-inline-btn dropdown-inline-btn-danger",
                            onclick: move |evt: Event<MouseData>| {
                                evt.stop_propagation();
                                spawn_project_delete(
                                    delete_project.clone(),
                                    active_scope,
                                    projects,
                                    tasks,
                                    confirming_delete,
                                );
                            },
                            "Yes"
                        }
                        button {
                            class: "dropdown-inline-btn",
                            onclick: move |evt: Event<MouseData>| {
                                evt.stop_propagation();
                                confirming_delete.set(None);
                            },
                            "No"
                        }
                    } else {
                        button {
                            class: "dropdown-inline-btn",
                            title: "Delete project database",
                            onclick: move |evt: Event<MouseData>| {
                                evt.stop_propagation();
                                confirming_delete.set(Some(confirm_name.clone()));
                            },
                            "Del"
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
            ProjectPicker { projects, active_scope, tasks }
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
