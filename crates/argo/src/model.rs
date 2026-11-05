use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SUPPORTED: [&str; 6] = ["claude", "codex", "hermes", "orca", "minimax", "multica"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment { pub name: String, pub sha256: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub title: String,
    pub prompt: String,
    pub provider: String,
    #[serde(default)] pub model: Option<String>,
    #[serde(default)] pub target_node: Option<String>,
    #[serde(default)] pub repo_alias: Option<String>,
    #[serde(default)] pub context: Value,
    #[serde(default)] pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
    pub target_node: Option<String>,
    pub assigned_node: Option<String>,
    pub repo_alias: Option<String>,
    pub context_json: String,
    pub attachments_json: String,
    pub artifacts_json: String,
    pub state: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub lease_until: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeRow {
    pub id: String,
    pub hostname: String,
    pub os: String,
    pub providers_json: String,
    pub repos_json: String,
    pub max_parallel: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNode {
    pub id: String,
    pub hostname: String,
    pub os: String,
    pub providers: Vec<String>,
    pub repos: Vec<String>,
    pub max_parallel: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerNode { pub node_id: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIn {
    pub node_id: String,
    pub stream: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishIn {
    pub node_id: String,
    pub status: String,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactIn { pub node_id: String, pub name: String, pub sha256: String, pub size: usize }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRow {
    pub seq: i64,
    pub stream: String,
    pub content: String,
    pub created_at: i64,
}

pub fn validate(t: &NewTask) -> Result<(), String> {
    if !SUPPORTED.contains(&t.provider.as_str()) { return Err("unsupported provider".into()); }
    if t.title.trim().is_empty() || t.title.len() > 200 { return Err("title must be 1..200 characters".into()); }
    if t.prompt.trim().is_empty() || t.prompt.len() > 120_000 { return Err("prompt must be 1..120000 characters".into()); }
    if t.attachments.len() > 20 { return Err("max 20 attachments".into()); }
    if t.attachments.iter().any(|a| a.name.contains('/') || a.name.contains('\\') || a.name == "." || a.name == ".." || a.name.is_empty() || a.name.len() > 120 || a.sha256.len() != 64 || !a.sha256.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("invalid attachment name/hash".into());
    }
    for v in [&t.model, &t.target_node, &t.repo_alias] {
        if v.as_deref().is_some_and(|s| s.len() > 160 || s.contains('\0')) { return Err("invalid selector".into()); }
    }
    if t.provider == "orca" && ctx(&t.context,"argo_action") != Some("discover_repos") {
        needed(&t.context, "orca_repo_id")?;
        needed(&t.context, "orca_agent")?;
    }
    if t.provider == "multica" {
        needed(&t.context, "multica_workspace_id")?;
        if ctx(&t.context,"multica_mode")==Some("existing") && !has(&t.context,"multica_issue_id") { return Err("existing multica issue id is required".into()); }
        match ctx(&t.context,"argo_action") {
            Some("discover_projects")|Some("discover_agents") => {},
            Some("discover_issues") => {needed(&t.context,"multica_project_id")?;},
            _ => {
                if !has(&t.context, "multica_issue_id") {needed(&t.context, "multica_project_id")?;}
                needed(&t.context, "multica_assignee")?;
            }
        }
    }
    Ok(())
}

pub fn ctx<'a>(ctx: &'a Value, name: &str) -> Option<&'a str> {
    ctx.get(name).and_then(Value::as_str).filter(|x| !x.trim().is_empty())
}
fn has(c: &Value, name: &str) -> bool { ctx(c,name).is_some() }
fn needed(c: &Value, n: &str) -> Result<(), String> {
    if has(c,n) { Ok(()) } else { Err(format!("missing required field: {n}")) }
}

/// CLI argv is constructed as separate arguments, never interpolated into a shell.
/// `cwd` and prompt files are resolved by the worker, not by the browser.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation { pub program: &'static str, pub args: Vec<String>, pub acceptance_only: bool }

pub fn invocation(task: &TaskRow, prompt_path: &str) -> Result<Invocation, String> {
    let c: Value = serde_json::from_str(&task.context_json).map_err(|e| e.to_string())?;
    let mut args: Vec<String>;
    let program;
    let acceptance_only;
    match task.provider.as_str() {
        "claude" => {
            program = "claude";
            args = vec!["-p".into(), task.prompt.clone(), "--output-format".into(), "stream-json".into()];
            if let Some(model) = &task.model { args.extend(["--model".into(),model.clone()]); }
            acceptance_only = false;
        }
        "codex" => {
            program = "codex";
            args = vec!["exec".into(), "--json".into(), "--sandbox".into(), "workspace-write".into(), "--skip-git-repo-check".into()];
            if let Some(model) = &task.model { args.extend(["--model".into(),model.clone()]); }
            args.push(task.prompt.clone());
            acceptance_only = false;
        }
        "hermes" => {
            program = "hermes";
            args = vec!["chat".into(), "--query-file".into(), prompt_path.into(), "--oneshot".into()];
            if let Some(model) = &task.model { args.extend(["--model".into(),model.clone()]); }
            acceptance_only = false;
        }
        "minimax" => {
            program = "mcode";
            args = vec!["exec".into(), task.prompt.clone()];
            if let Some(model) = &task.model { args.extend(["--model".into(),model.clone()]); }
            acceptance_only = false;
        }
        "orca" => {
            program = "orca";
            if ctx(&c,"argo_action")==Some("discover_repos") {
                return Ok(Invocation {program,args:vec!["repo".into(),"list".into(),"--json".into()],acceptance_only:false});
            }
            args = vec!["worktree".into(),"create".into(),"--repo".into(),format!("id:{}",ctx(&c,"orca_repo_id").ok_or("orca_repo_id missing")?),"--name".into(),format!("argo-{}",&task.id[..8.min(task.id.len())]),"--agent".into(),ctx(&c,"orca_agent").ok_or("orca_agent missing")?.to_string(),"--prompt".into(),task.prompt.clone(),"--json".into()];
            // Orca CLI creates/launches a worktree+agent; the command's exit is not model completion.
            acceptance_only = true;
        }
        "multica" => {
            program = "multica";
            let prefix=vec!["--workspace-id".into(),ctx(&c,"multica_workspace_id").ok_or("workspace id missing")?.to_string()];
            match ctx(&c,"argo_action") {
                Some("discover_projects") => return Ok(Invocation {program,args:[prefix,vec!["project".into(),"list".into(),"--output".into(),"json".into()]].concat(),acceptance_only:false}),
                Some("discover_agents") => return Ok(Invocation {program,args:[prefix,vec!["agent".into(),"list".into(),"--output".into(),"json".into()]].concat(),acceptance_only:false}),
                Some("discover_issues") => return Ok(Invocation {program,args:[prefix,vec!["issue".into(),"list".into(),"--project".into(),ctx(&c,"multica_project_id").ok_or("project missing")?.into(),"--output".into(),"json".into()]].concat(),acceptance_only:false}),
                _ => {}
            }
            args = prefix;
            args.push("issue".into());
            if let Some(issue) = ctx(&c,"multica_issue_id") {
                args.extend(["comment".into(),"add".into(),issue.into(),"--content-file".into(),prompt_path.into(),"--output".into(),"json".into()]);
            } else {
                args.extend(["create".into(),"--title".into(),task.title.clone(),"--description-file".into(),prompt_path.into(),"--project".into(),ctx(&c,"multica_project_id").ok_or("project id missing")?.to_string(),"--assignee".into(),ctx(&c,"multica_assignee").ok_or("assignee missing")?.to_string(),"--output".into(),"json".into()]);
            }
            // Multica creates/updates an Issue; remote agent execution is asynchronous.
            acceptance_only = true;
        }
        _ => return Err("unrecognized provider".into())
    };
    Ok(Invocation { program, args, acceptance_only })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(provider: &str, context: &str) -> NewTask {
        NewTask { title: "Review".into(),prompt:"hello; $(rm -rf /)".into(),provider:provider.into(),model:None,target_node:None,repo_alias:None,context:serde_json::from_str(context).unwrap(),attachments:vec![] }
    }
    #[test] fn validates_multica_requirements() {
        assert!(validate(&sample("multica","{}" )).is_err());
        assert!(validate(&sample("multica",r#"{"multica_workspace_id":"w","multica_project_id":"p","multica_assignee":"bot"}"#)).is_ok());
        assert!(validate(&sample("orca",r#"{"orca_repo_id":"r","orca_agent":"codex"}"#)).is_ok());
    }
    #[test] fn prompt_remains_one_argv_value() {
        let t=sample("codex","{}");
        let row=TaskRow { id:"abc12345".into(),title:t.title,prompt:t.prompt.clone(),provider:t.provider,model:None,target_node:None,assigned_node:None,repo_alias:None,context_json:"{}".into(),attachments_json:"[]".into(),artifacts_json:"[]".into(),state:"queued".into(),created_at:0,started_at:None,finished_at:None,lease_until:None,output:None,error:None };
        let invoke=invocation(&row,"prompt.txt").unwrap();
        assert_eq!(invoke.args.last().unwrap(),&t.prompt);
        assert_eq!(invoke.program,"codex");
    }
}
