use dioxus::prelude::*;
use llm_tasks::db::Task;

use crate::state::Project;

const STATUSES: &[&str] = &["pending", "in_progress", "completed"];

#[component]
fn ProjectPicker(
    projects: Signal<Vec<Project>>,
    active_project: Signal<Option<Project>>,
) -> Element {
    let active_name = active_project().map(|p| p.name).unwrap_or_default();

    rsx! {
        div { class: "project-picker",
            select {
                class: "project-select",
                value: "{active_name}",
                onchange: move |e| {
                    let name = e.value();
                    let proj = projects().into_iter().find(|p| p.name == name);
                    active_project.set(proj);
                },
                for proj in projects() {
                    option {
                        value: "{proj.name}",
                        selected: proj.name == active_name,
                        "{proj.name}"
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
fn TaskRow(task: Task, is_active: bool, selected: Signal<Option<String>>) -> Element {
    let id = task.id.clone();
    let status_class = format!("status-dot status-{}", task.status);
    let priority_label = if task.priority > 0 {
        format!("P{}", task.priority)
    } else {
        String::new()
    };

    rsx! {
        div {
            class: if is_active { "task-row active" } else { "task-row" },
            onclick: move |_| selected.set(Some(id.clone())),
            span { class: "{status_class}" }
            span { class: "task-row-title", "{task.title}" }
            if !priority_label.is_empty() {
                span { class: "badge-priority", "{priority_label}" }
            }
        }
    }
}

#[component]
pub fn Sidebar(
    tasks: Signal<Vec<Task>>,
    selected: Signal<Option<String>>,
    filter: Signal<Option<String>>,
    projects: Signal<Vec<Project>>,
    active_project: Signal<Option<Project>>,
) -> Element {
    let filtered: Vec<Task> = tasks
        .read()
        .iter()
        .filter(|t| match filter().as_deref() {
            Some(s) => t.status == s,
            None => true,
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "sidebar",
            ProjectPicker { projects, active_project }
            div { class: "sidebar-header", "TASKS" }
            StatusFilter { filter }
            div { class: "sidebar-list",
                for task in filtered {
                    {
                        let is_active = selected().as_deref() == Some(task.id.as_str());
                        rsx! {
                            TaskRow { key: "{task.id}", task, is_active, selected }
                        }
                    }
                }
            }
        }
    }
}
