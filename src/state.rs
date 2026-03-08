use std::collections::HashMap;
use std::path::PathBuf;

use llm_tasks::db::{Comment, Database, Event, Task};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub struct TaskDetail {
    pub project: Project,
    pub task: Task,
    pub depends_on: Vec<(String, String, String)>,
    pub blocks: Vec<(String, String, String)>,
    pub events: Vec<Event>,
    pub comments: Vec<Comment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub name: String,
    pub db_path: PathBuf,
}

impl Project {
    /// Whether this project is an orchestrator project (not the legacy llm-tasks one).
    pub fn is_orchestrator(&self) -> bool {
        self.name != "llm-tasks"
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProjectScope {
    Single(Project),
    All,
}

impl ProjectScope {
    pub fn label(&self) -> String {
        match self {
            Self::Single(project) => project.name.clone(),
            Self::All => "All projects".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedTask {
    pub project: Project,
    pub task_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskListItem {
    pub project: Project,
    pub task: Task,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub role: String,
    pub task_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct StatusResponse {
    agents: Vec<AgentInfo>,
}

/// Unified log entry for display, sourced from either JSONL or message_logs JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub kind: String,
    pub text: String,
    pub timestamp: String,
}

/// JSONL session log entry (Claude backend).
#[derive(Deserialize)]
struct JsonlEntry {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    timestamp: String,
}

/// message_logs JSON entry (Codex/OpenRouter backend).
#[derive(Deserialize)]
struct ChatLogEntry {
    role: String,
    content: Option<String>,
    #[serde(default)]
    timestamp: String,
}

pub fn discover_projects() -> Vec<Project> {
    let Some(data_dir) = dirs::data_dir() else {
        return Vec::new();
    };
    let mut projects = Vec::new();

    // Legacy llm-tasks location
    let legacy = data_dir.join("llm-tasks/tasks.db");
    if legacy.exists() {
        projects.push(Project {
            name: "llm-tasks".into(),
            db_path: legacy,
        });
    }

    // agent-orchestrator per-project DBs
    let orch_dir = data_dir.join("agent-orchestrator");
    if let Ok(entries) = std::fs::read_dir(&orch_dir) {
        for entry in entries.flatten() {
            let db_path = entry.path().join("tasks.db");
            if db_path.exists() {
                let name = entry.file_name().to_string_lossy().into_owned();
                projects.push(Project { name, db_path });
            }
        }
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

pub async fn open_db_for(project: &Project) -> Option<Database> {
    Database::open(&project.db_path).await.ok()
}

pub fn task_key(project: &Project, task_id: &str) -> String {
    format!("{}::{task_id}", project.name)
}

pub async fn list_tasks_for_scope(scope: &ProjectScope, projects: &[Project]) -> Vec<TaskListItem> {
    match scope {
        ProjectScope::Single(project) => list_tasks_for_project(project).await,
        ProjectScope::All => {
            let mut all_tasks = Vec::new();
            for project in projects {
                all_tasks.extend(list_tasks_for_project(project).await);
            }
            sort_task_items(&mut all_tasks);
            all_tasks
        }
    }
}

async fn list_tasks_for_project(project: &Project) -> Vec<TaskListItem> {
    let Some(db) = open_db_for(project).await else {
        return Vec::new();
    };
    let Ok(tasks) = db.list_tasks(None, None).await else {
        return Vec::new();
    };
    tasks
        .into_iter()
        .map(|task| TaskListItem {
            project: project.clone(),
            task,
        })
        .collect()
}

fn sort_task_items(tasks: &mut [TaskListItem]) {
    tasks.sort_by(|a, b| {
        b.task
            .priority
            .cmp(&a.task.priority)
            .then_with(|| a.task.created_at.cmp(&b.task.created_at))
            .then_with(|| a.project.name.cmp(&b.project.name))
            .then_with(|| a.task.id.cmp(&b.task.id))
    });
}

pub fn tasks_changed(old: &[TaskListItem], new: &[TaskListItem]) -> bool {
    if old.len() != new.len() {
        return true;
    }
    old.iter().zip(new.iter()).any(|(a, b)| {
        a.project != b.project || a.task.id != b.task.id || a.task.updated_at != b.task.updated_at
    })
}

pub async fn load_detail(project: &Project, task_id: &str) -> Option<TaskDetail> {
    let db = open_db_for(project).await?;
    let task = db.get_task(task_id).await.ok()?;
    let deps = db.get_dependencies(task_id).await.unwrap_or_default();
    let rev_deps = db
        .get_reverse_dependencies(task_id)
        .await
        .unwrap_or_default();
    let events = db.get_events(task_id).await.unwrap_or_default();
    let comments = db.get_comments(task_id).await.unwrap_or_default();

    let depends_on = collect_dep_details(&db, &deps, |d| &d.depends_on).await;
    let blocks = collect_dep_details(&db, &rev_deps, |d| &d.task_id).await;

    Some(TaskDetail {
        project: project.clone(),
        task,
        depends_on,
        blocks,
        events,
        comments,
    })
}

async fn collect_dep_details(
    db: &Database,
    deps: &[llm_tasks::db::Dependency],
    id_fn: fn(&llm_tasks::db::Dependency) -> &String,
) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for dep in deps {
        let id = id_fn(dep);
        let (title, status) = match db.get_task(id).await {
            Ok(t) => (t.title, t.status),
            Err(_) => (id.clone(), "unknown".into()),
        };
        out.push((id.clone(), title, status));
    }
    out
}

/// Query agent-orchestrator for running agents. Returns task_id -> AgentInfo map.
pub async fn fetch_agent_status(project: &Project) -> HashMap<String, AgentInfo> {
    if !project.is_orchestrator() {
        return HashMap::new();
    }
    let name = project.name.clone();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("agent-orchestrator")
            .args(["status", "--project", &name])
            .output()
    })
    .await;

    let output = match result {
        Ok(Ok(o)) if o.status.success() => o.stdout,
        _ => return HashMap::new(),
    };

    let resp: StatusResponse = match serde_json::from_slice(&output) {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };

    resp.agents
        .into_iter()
        .filter(|a| a.role == "task_agent" && !a.name.ends_with("-tools"))
        .filter_map(|a| {
            let tid = a.task_id.clone()?;
            Some((task_key(project, &tid), a))
        })
        .collect()
}

pub async fn fetch_agent_status_for_scope(
    scope: &ProjectScope,
    projects: &[Project],
) -> HashMap<String, AgentInfo> {
    match scope {
        ProjectScope::Single(project) => fetch_agent_status(project).await,
        ProjectScope::All => {
            let mut statuses = HashMap::new();
            for project in projects {
                statuses.extend(fetch_agent_status(project).await);
            }
            statuses
        }
    }
}

fn tail_entries<T>(items: Vec<T>, max: usize) -> Vec<T> {
    let skip = items.len().saturating_sub(max);
    items.into_iter().skip(skip).collect()
}

fn read_message_log_json(path: &std::path::Path, max: usize) -> Option<Vec<LogEntry>> {
    let content = std::fs::read_to_string(path).ok()?;
    let entries: Vec<ChatLogEntry> = serde_json::from_str(&content).ok()?;
    Some(tail_entries(
        entries
            .into_iter()
            .map(|e| LogEntry {
                kind: e.role,
                text: e.content.unwrap_or_default(),
                timestamp: e.timestamp,
            })
            .collect(),
        max,
    ))
}

fn read_session_log_jsonl(path: &std::path::Path, max: usize) -> Vec<LogEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    tail_entries(
        content
            .lines()
            .filter_map(|line| serde_json::from_str::<JsonlEntry>(line).ok())
            .map(|e| LogEntry {
                kind: e.kind,
                text: e.text,
                timestamp: e.timestamp,
            })
            .collect(),
        max,
    )
}

/// Read the last N entries of the agent's log for a task.
/// Tries message_logs JSON first (Codex/OpenRouter), then JSONL (Claude).
pub fn read_agent_log(project: &Project, task_id: &str, max_entries: usize) -> Vec<LogEntry> {
    if !project.is_orchestrator() {
        return Vec::new();
    }
    let Some(data_dir) = dirs::data_dir() else {
        return Vec::new();
    };
    let agent_name = format!("task-{task_id}");
    let base = data_dir.join("agent-orchestrator").join(&project.name);

    let msg_log = base.join("message_logs").join(format!("{agent_name}.json"));
    if let Some(entries) = read_message_log_json(&msg_log, max_entries) {
        return entries;
    }

    let jsonl_path = base.join("logs").join(format!("{agent_name}.jsonl"));
    read_session_log_jsonl(&jsonl_path, max_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> Project {
        Project {
            name: name.into(),
            db_path: PathBuf::from(format!("/tmp/{name}/tasks.db")),
        }
    }

    fn task(id: &str, priority: u8, created_at: &str, updated_at: &str) -> Task {
        Task {
            id: id.into(),
            title: format!("Task {id}"),
            description: None,
            status: "pending".into(),
            priority,
            assignee: None,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }

    #[test]
    fn sort_task_items_orders_by_priority_then_created_at_then_project() {
        let mut tasks = vec![
            TaskListItem {
                project: project("beta"),
                task: task("b", 1, "2026-03-02T10:00:00Z", "2026-03-02T10:00:00Z"),
            },
            TaskListItem {
                project: project("alpha"),
                task: task("a", 3, "2026-03-03T10:00:00Z", "2026-03-03T10:00:00Z"),
            },
            TaskListItem {
                project: project("aardvark"),
                task: task("c", 3, "2026-03-03T10:00:00Z", "2026-03-03T10:00:00Z"),
            },
        ];

        sort_task_items(&mut tasks);

        let ordered: Vec<_> = tasks
            .iter()
            .map(|item| format!("{}:{}", item.project.name, item.task.id))
            .collect();
        assert_eq!(ordered, vec!["aardvark:c", "alpha:a", "beta:b"]);
    }

    #[test]
    fn tasks_changed_detects_project_or_task_updates() {
        let alpha = project("alpha");
        let beta = project("beta");
        let original = vec![TaskListItem {
            project: alpha.clone(),
            task: task("t1", 1, "2026-03-02T10:00:00Z", "2026-03-02T10:00:00Z"),
        }];

        let changed_project = vec![TaskListItem {
            project: beta,
            task: task("t1", 1, "2026-03-02T10:00:00Z", "2026-03-02T10:00:00Z"),
        }];
        assert!(tasks_changed(&original, &changed_project));

        let changed_task = vec![TaskListItem {
            project: alpha,
            task: task("t1", 1, "2026-03-02T10:00:00Z", "2026-03-02T11:00:00Z"),
        }];
        assert!(tasks_changed(&original, &changed_task));
    }
}
