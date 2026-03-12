use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    /// Whether this project comes from agent-orchestrator rather than the legacy db path.
    pub fn is_orchestrator(&self) -> bool {
        let Some(data_dir) = dirs::data_dir() else {
            return false;
        };
        is_orchestrator_project_at(self, &data_dir)
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

#[derive(Deserialize)]
struct ChatToolCall {
    id: String,
    name: String,
    arguments: String,
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
    tool_calls: Option<Vec<ChatToolCall>>,
    #[serde(default)]
    tool_call_id: Option<String>,
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

fn orchestrator_db_path_at(data_dir: &Path, project_name: &str) -> PathBuf {
    data_dir
        .join("agent-orchestrator")
        .join(project_name)
        .join("tasks.db")
}

fn is_orchestrator_project_at(project: &Project, data_dir: &Path) -> bool {
    project.db_path == orchestrator_db_path_at(data_dir, &project.name)
}

pub fn can_delete_project_db(project: &Project) -> bool {
    let Some(data_dir) = dirs::data_dir() else {
        return false;
    };
    can_delete_project_db_at(project, &data_dir)
}

fn can_delete_project_db_at(project: &Project, data_dir: &Path) -> bool {
    is_orchestrator_project_at(project, data_dir)
}

pub fn delete_project_db(project: &Project) -> Result<(), String> {
    let Some(data_dir) = dirs::data_dir() else {
        return Err("Could not resolve data directory".into());
    };
    delete_project_db_at(project, &data_dir)
}

fn delete_project_db_at(project: &Project, data_dir: &Path) -> Result<(), String> {
    if !can_delete_project_db_at(project, data_dir) {
        return Err("Only agent-orchestrator project databases can be deleted".into());
    }

    match std::fs::remove_file(&project.db_path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(format!(
                "Failed to delete {}: {err}",
                project.db_path.display()
            ));
        }
    }

    if let Some(project_dir) = project.db_path.parent() {
        match std::fs::remove_dir(project_dir) {
            Ok(()) => {}
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::DirectoryNotEmpty
                        | std::io::ErrorKind::Other
                ) => {}
            Err(err) => {
                return Err(format!(
                    "Deleted database but failed to remove project directory {}: {err}",
                    project_dir.display()
                ));
            }
        }
    }

    Ok(())
}

pub fn normalize_scope(scope: &ProjectScope, projects: &[Project]) -> ProjectScope {
    match scope {
        ProjectScope::All => ProjectScope::All,
        ProjectScope::Single(project) => projects
            .iter()
            .find(|candidate| **candidate == *project)
            .cloned()
            .map(ProjectScope::Single)
            .or_else(|| projects.first().cloned().map(ProjectScope::Single))
            .unwrap_or(ProjectScope::All),
    }
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
            .flat_map(chat_log_entry_to_log_entries)
            .collect(),
        max,
    ))
}

fn chat_log_entry_to_log_entries(entry: ChatLogEntry) -> Vec<LogEntry> {
    let mut out = Vec::new();

    if let Some(content) = entry.content {
        if !content.is_empty() {
            let text = if entry.role == "tool" {
                match entry.tool_call_id.as_deref() {
                    Some(id) if !id.is_empty() => format!("[{id}] {content}"),
                    _ => content,
                }
            } else {
                content
            };
            out.push(LogEntry {
                kind: entry.role.clone(),
                text,
                timestamp: entry.timestamp.clone(),
            });
        }
    }

    if let Some(tool_calls) = entry.tool_calls {
        out.extend(tool_calls.into_iter().map(|tool_call| LogEntry {
            kind: "tool_call".into(),
            text: format!(
                "{} [{}] {}",
                tool_call.name, tool_call.id, tool_call.arguments
            ),
            timestamp: entry.timestamp.clone(),
        }));
    }

    out
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
            target_branch: None,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }

    fn temp_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "llm-tasks-viewer-{prefix}-{}-{nanos}",
            std::process::id()
        ))
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

    #[test]
    fn chat_log_entry_expands_tool_calls() {
        let entries = chat_log_entry_to_log_entries(ChatLogEntry {
            role: "assistant".into(),
            content: Some("Running checks".into()),
            tool_calls: Some(vec![ChatToolCall {
                id: "call_1".into(),
                name: "Bash".into(),
                arguments: r#"{"command":"pwd"}"#.into(),
            }]),
            tool_call_id: None,
            timestamp: "2026-03-12T22:10:00Z".into(),
        });

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, "assistant");
        assert_eq!(entries[0].text, "Running checks");
        assert_eq!(entries[1].kind, "tool_call");
        assert!(entries[1].text.contains("Bash [call_1]"));
    }

    #[test]
    fn delete_project_db_removes_orchestrator_db() {
        let root = temp_path("delete-project");
        let project_dir = root.join("agent-orchestrator").join("alpha");
        std::fs::create_dir_all(&project_dir).unwrap();
        let db_path = project_dir.join("tasks.db");
        std::fs::write(&db_path, "db").unwrap();

        let project = Project {
            name: "alpha".into(),
            db_path: db_path.clone(),
        };

        delete_project_db_at(&project, &root).unwrap();

        assert!(!db_path.exists());
        assert!(!project_dir.exists());
    }

    #[test]
    fn delete_project_db_rejects_legacy_database() {
        let root = temp_path("delete-legacy");
        let legacy_dir = root.join("llm-tasks");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        let db_path = legacy_dir.join("tasks.db");
        std::fs::write(&db_path, "db").unwrap();

        let project = Project {
            name: "llm-tasks".into(),
            db_path: db_path.clone(),
        };

        let result = delete_project_db_at(&project, &root);

        assert!(result.is_err());
        assert!(db_path.exists());
    }

    #[test]
    fn delete_project_db_allows_orchestrator_project_named_llm_tasks() {
        let root = temp_path("delete-orch-llm-tasks");
        let project_dir = root.join("agent-orchestrator").join("llm-tasks");
        std::fs::create_dir_all(&project_dir).unwrap();
        let db_path = project_dir.join("tasks.db");
        std::fs::write(&db_path, "db").unwrap();

        let project = Project {
            name: "llm-tasks".into(),
            db_path,
        };

        assert!(can_delete_project_db_at(&project, &root));
    }

    #[test]
    fn normalize_scope_replaces_missing_selected_project() {
        let alpha = project("alpha");
        let beta = project("beta");

        let normalized = normalize_scope(&ProjectScope::Single(alpha), std::slice::from_ref(&beta));

        assert_eq!(normalized, ProjectScope::Single(beta));
    }
}
