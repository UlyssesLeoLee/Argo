use crate::model::*;
use axum::{body::Bytes, extract::{Path, Query, State, DefaultBodyLimit}, http::{header, HeaderMap, StatusCode}, routing::{get,post}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, Row, sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}};
use std::{path::{Path as FilePath, PathBuf},sync::Arc,time::{SystemTime, UNIX_EPOCH}};
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct ServerState { pub pool: SqlitePool, pub admin_token: String, pub worker_token: String, pub data_dir: PathBuf }
type ApiError = (StatusCode, String);
type ApiResult<T> = Result<Json<T>,ApiError>;
fn err(c: StatusCode, msg: impl ToString)->ApiError { (c,msg.to_string()) }
fn internal(e: impl ToString)->ApiError { err(StatusCode::INTERNAL_SERVER_ERROR,e) }
fn now()->i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 }
fn auth(headers: &HeaderMap, token: &str)->Result<(),ApiError> {
    let got=headers.get(header::AUTHORIZATION).and_then(|v|v.to_str().ok()).and_then(|s|s.strip_prefix("Bearer "));
    // This API uses bearer auth; restrict dashboard to localhost unless using TLS reverse proxy.
    if got.is_some_and(|s|s==token) { Ok(()) } else { Err(err(StatusCode::UNAUTHORIZED,"invalid bearer token")) }
}
fn auth_either(h:&HeaderMap, s:&ServerState)->Result<(),ApiError>{
    if auth(h,&s.admin_token).is_ok() || auth(h,&s.worker_token).is_ok(){Ok(())}else{Err(err(StatusCode::UNAUTHORIZED,"invalid bearer token"))}
}

pub async fn serve(bind:&str,data_dir:&str, static_dir:&str, admin:String, worker:String)->Result<(),Box<dyn std::error::Error>>{
    if admin.len()<24 || worker.len()<24 || admin==worker {return Err("independent ADMIN/WORKER tokens of >=24 chars required".into())}
    let root=PathBuf::from(data_dir);tokio::fs::create_dir_all(root.join("blobs")).await?;
    let options=SqliteConnectOptions::new().filename(root.join("argo.sqlite")).create_if_missing(true).journal_mode(SqliteJournalMode::Wal);
    let pool=SqlitePoolOptions::new().max_connections(8).connect_with(options).await?;
    migrate(&pool).await?;
    crate::rag::migrate(&pool).await?;
    let state=ServerState {pool,admin_token:admin,worker_token:worker,data_dir:root};
    let cleaner=state.pool.clone();
    tokio::spawn(async move {loop {tokio::time::sleep(std::time::Duration::from_secs(30)).await;if let Err(e)=expiration_scan(&cleaner).await {tracing::error!(error=%e,"lease scanner failed");}}});
    let app=Router::new()
        .route("/health",get(||async{Json(json!({"status":"ok","service":"argo","version":"0.3.0"}))}))
        .route("/api/v1/providers",get(providers))
        .route("/api/v1/nodes",get(nodes))
        .route("/api/v1/tasks",get(tasks).post(create_task))
        .route("/api/v1/tasks/{id}",get(task))
        .route("/api/v1/tasks/{id}/events",get(events))
        .route("/api/v1/tasks/{id}/cancel",post(cancel))
        .route("/api/v1/blobs",post(upload_blob))
        .route("/api/v1/blobs/{sha}",get(download_blob))
        .route("/api/v1/worker/register",post(register))
        .route("/api/v1/worker/claim",post(claim))
        .route("/api/v1/worker/tasks/{id}/event",post(worker_event))
        .route("/api/v1/worker/tasks/{id}/artifact",post(worker_artifact))
        .route("/api/v1/worker/tasks/{id}/renew",post(renew))
        .route("/api/v1/worker/tasks/{id}/state",post(worker_state))
        .route("/api/v1/worker/tasks/{id}/finish",post(finish))
        .merge(crate::rag::routes())
        .merge(crate::ai::routes())
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
        .with_state(Arc::new(state));
    let listener=tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind,"Argo controller started");
    axum::serve(listener,app).await?;
    Ok(())
}

async fn migrate(pool:&SqlitePool)->Result<(),sqlx::Error>{
    for stmt in [
        "CREATE TABLE IF NOT EXISTS nodes(id TEXT PRIMARY KEY,hostname TEXT NOT NULL,os TEXT NOT NULL,providers_json TEXT NOT NULL,repos_json TEXT NOT NULL,max_parallel INTEGER NOT NULL,last_seen INTEGER NOT NULL)",
        "CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY,title TEXT NOT NULL,prompt TEXT NOT NULL,provider TEXT NOT NULL,model TEXT,target_node TEXT,assigned_node TEXT,repo_alias TEXT,context_json TEXT NOT NULL,attachments_json TEXT NOT NULL,artifacts_json TEXT NOT NULL DEFAULT '[]',state TEXT NOT NULL,created_at INTEGER NOT NULL,started_at INTEGER,finished_at INTEGER,lease_until INTEGER,output TEXT,error TEXT)",
        "CREATE INDEX IF NOT EXISTS ix_tasks_state ON tasks(state,created_at)",
        "CREATE TABLE IF NOT EXISTS task_events(task_id TEXT NOT NULL,seq INTEGER NOT NULL,stream TEXT NOT NULL,content TEXT NOT NULL,created_at INTEGER NOT NULL,PRIMARY KEY(task_id,seq))",
    ] { sqlx::query(stmt).execute(pool).await?; }
    Ok(())
}

async fn providers(State(_s):State<Arc<ServerState>>,h:HeaderMap)->ApiResult<Value>{
    // Reading provider metadata is gated consistently with all private state endpoints.
    let s=_s;auth(&h,&s.admin_token)?;
    Ok(Json(json!([
      {"id":"claude","label":"Claude Code","type":"cli","needs":[]},
      {"id":"codex","label":"Codex","type":"cli","needs":[]},
      {"id":"hermes","label":"Hermes","type":"cli","needs":[]},
      {"id":"minimax","label":"MiniMax Code","type":"cli","needs":[]},
      {"id":"orca","label":"Orca","type":"orchestrator","needs":["orca_repo_id","orca_agent"]},
      {"id":"multica","label":"Multica","type":"issue","needs":["multica_workspace_id","multica_project_id","multica_assignee"]},
      {"id":"star","label":"STAR","type":"rest_webhook","needs":["star_project_id"]}
    ])))
}
async fn nodes(State(s):State<Arc<ServerState>>,h:HeaderMap)->ApiResult<Value>{
    auth(&h,&s.admin_token)?;
    let rows=sqlx::query_as::<_,NodeRow>("SELECT * FROM nodes ORDER BY last_seen DESC").fetch_all(&s.pool).await.map_err(internal)?;
    Ok(Json(json!(rows)))
}
#[derive(Deserialize)]struct Pagination{limit:Option<i64>}
async fn tasks(State(s):State<Arc<ServerState>>,h:HeaderMap,Query(q):Query<Pagination>)->ApiResult<Value>{
    auth(&h,&s.admin_token)?;
    let limit=q.limit.unwrap_or(100).clamp(1,300);
    let rows=sqlx::query_as::<_,TaskRow>("SELECT * FROM tasks ORDER BY created_at DESC LIMIT ?")
        .bind(limit).fetch_all(&s.pool).await.map_err(internal)?;
    Ok(Json(json!(rows)))
}
async fn task(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>)->ApiResult<Value>{
    auth(&h,&s.admin_token)?;
    let row=sqlx::query_as::<_,TaskRow>("SELECT * FROM tasks WHERE id=?").bind(id).fetch_optional(&s.pool).await.map_err(internal)?;
    row.map(|r|Json(json!(r))).ok_or(err(StatusCode::NOT_FOUND,"task not found"))
}
async fn create_task(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(t):Json<NewTask>)->ApiResult<Value>{
    auth(&h,&s.admin_token)?; validate(&t).map_err(|e|err(StatusCode::BAD_REQUEST,e))?;
    for a in &t.attachments {
        if !s.data_dir.join("blobs").join(&a.sha256).is_file() {return Err(err(StatusCode::BAD_REQUEST,format!("attachment not uploaded: {}",a.name)))}
    }
    let id=uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tasks(id,title,prompt,provider,model,target_node,repo_alias,context_json,attachments_json,state,created_at) VALUES(?,?,?,?,?,?,?,?,?,'queued',?)")
        .bind(&id).bind(&t.title).bind(&t.prompt).bind(&t.provider).bind(&t.model).bind(&t.target_node).bind(&t.repo_alias)
        .bind(serde_json::to_string(&t.context).map_err(internal)?)
        .bind(serde_json::to_string(&t.attachments).map_err(internal)?)
        .bind(now()).execute(&s.pool).await.map_err(internal)?;
    Ok(Json(json!({"id":id,"state":"queued"})))
}
#[derive(Deserialize)]struct Since{since:Option<i64>}
async fn events(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Query(q):Query<Since>)->ApiResult<Value>{
    auth(&h,&s.admin_token)?;
    let rows=sqlx::query_as::<_,EventRow>("SELECT seq,stream,content,created_at FROM task_events WHERE task_id=? AND seq>? ORDER BY seq LIMIT 200")
        .bind(id).bind(q.since.unwrap_or(0).max(0)).fetch_all(&s.pool).await.map_err(internal)?;
    Ok(Json(json!(rows)))
}
async fn cancel(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>)->ApiResult<Value>{
    auth(&h,&s.admin_token)?;
    let r=sqlx::query("UPDATE tasks SET state=CASE WHEN state='queued' THEN 'cancelled' ELSE 'cancelling' END,finished_at=CASE WHEN state='queued' THEN ? ELSE finished_at END WHERE id=? AND state IN ('queued','running')")
        .bind(now()).bind(id).execute(&s.pool).await.map_err(internal)?;
    if r.rows_affected()==0{return Err(err(StatusCode::CONFLICT,"task is not cancellable"))}
    Ok(Json(json!({"ok":true})))
}
async fn upload_blob(State(s):State<Arc<ServerState>>,h:HeaderMap,bytes:Bytes)->ApiResult<Value>{
    auth_either(&h,&s)?;
    if bytes.is_empty() || bytes.len()>10*1024*1024{return Err(err(StatusCode::BAD_REQUEST,"file must be 1..10 MiB"))}
    let sha=hex::encode(Sha256::digest(&bytes));
    let destination=s.data_dir.join("blobs").join(&sha);
    if !destination.is_file(){
        let temp=s.data_dir.join("blobs").join(format!("{}.tmp",uuid::Uuid::new_v4()));
        tokio::fs::write(&temp,&bytes).await.map_err(internal)?;
        if tokio::fs::rename(&temp,&destination).await.is_err() && !destination.is_file(){return Err(internal("blob persist failure"))}
        let _=tokio::fs::remove_file(&temp).await;
    }
    Ok(Json(json!({"sha256":sha,"size":bytes.len()})))
}
async fn download_blob(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(sha):Path<String>)->Result<Vec<u8>,ApiError>{
    auth_either(&h,&s)?;
    if sha.len()!=64 || !sha.bytes().all(|b|b.is_ascii_hexdigit()) {return Err(err(StatusCode::BAD_REQUEST,"invalid hash"))}
    tokio::fs::read(s.data_dir.join("blobs").join(sha)).await.map_err(|_|err(StatusCode::NOT_FOUND,"blob not found"))
}
async fn register(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(n):Json<RegisterNode>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    if n.id.trim().is_empty() || n.id.len()>100 || n.max_parallel<1 || n.max_parallel>64 {return Err(err(StatusCode::BAD_REQUEST,"invalid worker"))}
    sqlx::query("INSERT INTO nodes(id,hostname,os,providers_json,repos_json,max_parallel,last_seen) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET hostname=excluded.hostname,os=excluded.os,providers_json=excluded.providers_json,repos_json=excluded.repos_json,max_parallel=excluded.max_parallel,last_seen=excluded.last_seen")
        .bind(n.id).bind(n.hostname).bind(n.os).bind(serde_json::to_string(&n.providers).map_err(internal)?)
        .bind(serde_json::to_string(&n.repos).map_err(internal)?).bind(n.max_parallel).bind(now())
        .execute(&s.pool).await.map_err(internal)?;
    Ok(Json(json!({"ok":true})))
}
async fn claim(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(n):Json<WorkerNode>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    let node=sqlx::query_as::<_,NodeRow>("SELECT * FROM nodes WHERE id=? AND last_seen>?")
        .bind(&n.node_id).bind(now()-60).fetch_optional(&s.pool).await.map_err(internal)?
        .ok_or(err(StatusCode::BAD_REQUEST,"worker not registered or stale"))?;
    let running:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE assigned_node=? AND state IN ('running','cancelling')")
        .bind(&n.node_id).fetch_one(&s.pool).await.map_err(internal)?;
    if running>=node.max_parallel{return Ok(Json(Value::Null))}
    let available:Vec<String>=serde_json::from_str(&node.providers_json).unwrap_or_default();
    let repos:Vec<String>=serde_json::from_str(&node.repos_json).unwrap_or_default();
    let queue=sqlx::query_as::<_,TaskRow>("SELECT * FROM tasks WHERE state='queued' ORDER BY created_at ASC LIMIT 100").fetch_all(&s.pool).await.map_err(internal)?;
    for item in queue {
        if !available.contains(&item.provider) {continue}
        if item.target_node.as_deref().is_some_and(|target|target!=n.node_id){continue}
        if item.repo_alias.as_deref().is_some_and(|repo|!repos.iter().any(|r|r==repo)){continue}
        // Compare-and-swap ensures multiple daemon claims cannot take the same task.
        let changed=sqlx::query("UPDATE tasks SET state='running',assigned_node=?,started_at=?,lease_until=? WHERE id=? AND state='queued'")
            .bind(&n.node_id).bind(now()).bind(now()+90).bind(&item.id).execute(&s.pool).await.map_err(internal)?;
        if changed.rows_affected()==1 {
            let row=sqlx::query_as::<_,TaskRow>("SELECT * FROM tasks WHERE id=?").bind(item.id).fetch_one(&s.pool).await.map_err(internal)?;
            return Ok(Json(json!(row)));
        }
    }
    Ok(Json(Value::Null))
}
async fn own(s:&ServerState,id:&str,node:&str)->Result<String,ApiError>{
    sqlx::query_scalar::<_,String>("SELECT state FROM tasks WHERE id=? AND assigned_node=?")
        .bind(id).bind(node).fetch_optional(&s.pool).await.map_err(internal)?
        .ok_or(err(StatusCode::FORBIDDEN,"task not leased by this worker"))
}
async fn worker_event(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(e):Json<EventIn>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    own(&s,&id,&e.node_id).await?;
    if !["stdout","stderr","system"].contains(&e.stream.as_str()) || e.content.len()>16000{return Err(err(StatusCode::BAD_REQUEST,"invalid event"))}
    sqlx::query("INSERT INTO task_events(task_id,seq,stream,content,created_at) VALUES(?,(SELECT COALESCE(MAX(seq),0)+1 FROM task_events WHERE task_id=?),?,?,?)")
        .bind(&id).bind(&id).bind(&e.stream).bind(&e.content).bind(now()).execute(&s.pool).await.map_err(internal)?;
    Ok(Json(json!({"ok":true})))
}

async fn worker_artifact(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(a):Json<ArtifactIn>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    own(&s,&id,&a.node_id).await?;
    if a.name.is_empty() || a.name.len()>150 || a.name.contains('/') || a.name.contains('\\') || a.sha256.len()!=64 || !a.sha256.bytes().all(|b|b.is_ascii_hexdigit()) {
        return Err(err(StatusCode::BAD_REQUEST,"invalid artifact"))
    }
    if !s.data_dir.join("blobs").join(&a.sha256).is_file(){return Err(err(StatusCode::BAD_REQUEST,"artifact blob missing"))}
    let current:String=sqlx::query_scalar("SELECT artifacts_json FROM tasks WHERE id=?").bind(&id).fetch_one(&s.pool).await.map_err(internal)?;
    let mut files:Vec<Value>=serde_json::from_str(&current).unwrap_or_default();
    if files.len()>=20{return Err(err(StatusCode::BAD_REQUEST,"artifact limit reached"))}
    files.push(json!({"name":a.name,"sha256":a.sha256,"size":a.size}));
    sqlx::query("UPDATE tasks SET artifacts_json=? WHERE id=? AND assigned_node=?").bind(serde_json::to_string(&files).map_err(internal)?).bind(&id).bind(&a.node_id).execute(&s.pool).await.map_err(internal)?;
    Ok(Json(json!({"ok":true})))
}

async fn renew(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(n):Json<WorkerNode>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    let result=sqlx::query("UPDATE tasks SET lease_until=? WHERE id=? AND assigned_node=? AND state IN ('running','cancelling')")
        .bind(now()+90).bind(id).bind(n.node_id).execute(&s.pool).await.map_err(internal)?;
    if result.rows_affected()==0{return Err(err(StatusCode::CONFLICT,"lease cannot be renewed"))}
    Ok(Json(json!({"ok":true})))
}
async fn worker_state(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(n):Json<WorkerNode>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    let state=own(&s,&id,&n.node_id).await?;
    Ok(Json(json!({"state":state})))
}
async fn finish(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(f):Json<FinishIn>)->ApiResult<Value>{
    auth(&h,&s.worker_token)?;
    if !["completed","failed","dispatched","cancelled"].contains(&f.status.as_str()){return Err(err(StatusCode::BAD_REQUEST,"invalid completion status"))}
    let result=sqlx::query("UPDATE tasks SET state=CASE WHEN state='cancelling' THEN 'cancelled' ELSE ? END,output=?,error=?,finished_at=?,lease_until=NULL WHERE id=? AND assigned_node=? AND state IN ('running','cancelling')")
        .bind(&f.status).bind(f.output.chars().take(100_000).collect::<String>()).bind(&f.error).bind(now())
        .bind(&id).bind(f.node_id).execute(&s.pool).await.map_err(internal)?;
    if result.rows_affected()==0{return Err(err(StatusCode::CONFLICT,"task does not belong to worker"))}
    if matches!(f.status.as_str(), "completed"|"dispatched") {
        if let Err(e)=crate::rag::capture_task_if_opted(&s.pool,&id).await {
            if e!="RAG capture not opted in" {tracing::warn!(task_id=%id,error=%e,"RAG draft capture skipped")}
        }
    }
    Ok(Json(json!({"ok":true})))
}

/// A conscious safety choice: expired leases need manual operator review because agent tools
/// have external side effects and blind replay could create duplicate changes.
pub async fn expiration_scan(pool:&SqlitePool)->Result<u64,sqlx::Error>{
    Ok(sqlx::query("UPDATE tasks SET state='needs_review',error='worker lease expired; inspect workspace before retry',finished_at=? WHERE state IN ('running','cancelling') AND lease_until<?")
       .bind(now()).bind(now()).execute(pool).await?.rows_affected())
}

pub fn is_under(candidate:&FilePath,root:&FilePath)->bool{candidate.canonicalize().ok().zip(root.canonicalize().ok()).is_some_and(|(a,b)|a.starts_with(b))}
