mod detail;
mod sidebar;
mod state;

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::detail::Detail;
use crate::sidebar::Sidebar;
use crate::state::{AgentInfo, Project, ProjectScope, SelectedTask, TaskDetail, TaskListItem};

const STYLE: &str = include_str!("../assets/style.css");

async fn refresh_detail(sel: Option<SelectedTask>, mut task_detail: Signal<Option<TaskDetail>>) {
    match sel {
        Some(selection) => {
            task_detail.set(state::load_detail(&selection.project, &selection.task_id).await);
        }
        _ => task_detail.set(None),
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_disable_context_menu(true)
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("llm-tasks")
                        .with_decorations(false)
                        .with_inner_size(dioxus::desktop::LogicalSize::new(960, 640)),
                ),
        )
        .launch(App);
}

fn setup_project_refresh(
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    mut tasks: Signal<Vec<TaskListItem>>,
) {
    use_effect(move || {
        let scope = active_scope();
        let available_projects = projects();
        spawn(async move {
            tasks.set(state::list_tasks_for_scope(&scope, &available_projects).await);
        });
    });
}

fn setup_detail_refresh(
    selected: Signal<Option<SelectedTask>>,
    tasks: Signal<Vec<TaskListItem>>,
    task_detail: Signal<Option<TaskDetail>>,
) {
    use_effect(move || {
        let sel = selected();
        let items = tasks();
        spawn(async move {
            let valid = sel.filter(|selection| {
                items.iter().any(|item| {
                    item.project == selection.project && item.task.id == selection.task_id
                })
            });
            refresh_detail(valid, task_detail).await;
        });
    });
}

fn setup_selection_cleanup(
    mut selected: Signal<Option<SelectedTask>>,
    tasks: Signal<Vec<TaskListItem>>,
) {
    use_effect(move || {
        let current = selected();
        let items = tasks();
        if let Some(selection) = current {
            let exists = items
                .iter()
                .any(|item| item.project == selection.project && item.task.id == selection.task_id);
            if !exists {
                selected.set(None);
            }
        }
    });
}

#[component]
fn App() -> Element {
    let tasks: Signal<Vec<TaskListItem>> = use_signal(Vec::new);
    let selected: Signal<Option<SelectedTask>> = use_signal(|| None);
    let filter: Signal<Option<String>> = use_signal(|| None);
    let task_detail: Signal<Option<TaskDetail>> = use_signal(|| None);
    let agent_statuses: Signal<HashMap<String, AgentInfo>> = use_signal(HashMap::new);
    let projects = use_signal(state::discover_projects);
    let active_scope: Signal<ProjectScope> = use_signal(|| {
        state::discover_projects()
            .into_iter()
            .next()
            .map(ProjectScope::Single)
            .unwrap_or(ProjectScope::All)
    });

    use_future(move || poll_tasks(active_scope, projects, tasks));
    use_future(move || poll_agent_status(active_scope, projects, agent_statuses));
    setup_project_refresh(active_scope, projects, tasks);
    setup_detail_refresh(selected, tasks, task_detail);
    setup_selection_cleanup(selected, tasks);

    rsx! {
        style { "{STYLE}" }
        div { class: "app",
            div { class: "drag-region" }
            div { class: "app-body",
                Sidebar { tasks, selected, filter, projects, active_scope }
                Detail { detail: task_detail, selected, active_scope, projects, tasks, agent_statuses }
            }
        }
    }
}

async fn poll_tasks(
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    mut tasks: Signal<Vec<TaskListItem>>,
) {
    loop {
        let scope = active_scope();
        let available_projects = projects();
        let new_tasks = state::list_tasks_for_scope(&scope, &available_projects).await;
        if state::tasks_changed(&tasks.read(), &new_tasks) {
            tasks.set(new_tasks);
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn poll_agent_status(
    active_scope: Signal<ProjectScope>,
    projects: Signal<Vec<Project>>,
    mut agent_statuses: Signal<HashMap<String, AgentInfo>>,
) {
    loop {
        let scope = active_scope();
        let available_projects = projects();
        let new_statuses = state::fetch_agent_status_for_scope(&scope, &available_projects).await;
        if *agent_statuses.read() != new_statuses {
            agent_statuses.set(new_statuses);
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
