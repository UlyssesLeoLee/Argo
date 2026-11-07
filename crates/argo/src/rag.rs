//! Personal RAG: local review queue + generic server-side remote connector profiles.
//! No embedding or remote DB is silently assumed. Source captures are opt-in.
use crate::server::ServerState;
use axum::{extract::{Path,Query,State},http::{HeaderMap,header,StatusCode},routing::{get,post},Json,Router};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use sqlx::{Row,SqlitePool,FromRow};
use std::{sync::Arc,time::{SystemTime,UNIX_EPOCH}};

type E=(StatusCode,String);
type ResultJson=Result<Json<Value>,E>;
fn bad(e:impl ToString)->E{(StatusCode::BAD_REQUEST,e.to_string())}
fn fault(e:impl ToString)->E{(StatusCode::INTERNAL_SERVER_ERROR,e.to_string())}
fn auth(h:&HeaderMap,s:&ServerState)->Result<(),E>{
    let token=h.get(header::AUTHORIZATION).and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer "));
    if token==Some(s.admin_token.as_str()){Ok(())} else {Err((StatusCode::UNAUTHORIZED,"invalid administrator token".into()))}
}
fn now()->i64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64}
fn hash(data:&str)->String{hex::encode(Sha256::digest(data.as_bytes()))}
fn clean_tags(v:&[String])->Vec<String>{v.iter().map(|t|t.trim()).filter(|t|!t.is_empty()&&t.chars().count()<=32).take(12).map(|t|t.to_string()).collect()}
pub(crate) fn redact(src:&str)->String{
    // First redact Bearer credentials, then named secrets; review remains mandatory.
    let bearer=regex::Regex::new(r"(?i)bearer\s+\S+").expect("valid regex");
    let masked=bearer.replace_all(src,"[REDACTED]");
    let named=regex::Regex::new(r"(?i)(?:(?:sk|ghp|gho|ghu|ghs|github_pat|AIza)[-_][A-Za-z0-9_-]{12,}|(?:api[_ -]?key|authorization|password|secret)\s*[:=]\s*\S+)").expect("valid regex");
    named.replace_all(&masked,"[REDACTED]").into_owned()
}

/// Deterministic Unicode-safe segmentation, 850 chars / 110 overlap.
pub fn chunks(input:&str,max_chars:usize,overlap:usize)->Vec<String>{
    let text=input.trim();if text.is_empty()||max_chars<=overlap{return vec![]}
    let v:Vec<char>=text.chars().collect();let mut start=0;let mut result=vec![];
    while start<v.len()&&result.len()<128 {
        let end=(start+max_chars).min(v.len());
        let mut cut=end;
        if end<v.len(){
            let begin=(start+max_chars/2).min(end);
            if let Some(i)=(begin..end).rev().find(|&i|v[i]=='\n'||v[i]=='。'||v[i]=='.'||v[i]=='！'||v[i]=='？') {cut=i+1;}
        }
        if cut<=start {cut=end;}
        let piece:String=v[start..cut].iter().collect();
        if !piece.trim().is_empty(){result.push(piece.trim().to_string());}
        if cut==v.len(){break}
        start=cut.saturating_sub(overlap).max(start+1);
    }
    result
}
#[derive(Debug,Serialize,FromRow)]
struct Chunk{ id:String,source_key:String,source_kind:String,provider:String,project:String,task_id:Option<String>,title:String,content:String,summary:String,tags_json:String,status:String,remote_id:Option<String>,fingerprint:String,created_at:i64,updated_at:i64,revision:i64 }
#[derive(Debug,Deserialize)]
pub struct Ingest{pub source_key:String,pub source_kind:String,pub provider:String,pub project:String,pub task_id:Option<String>,pub title:String,pub content:String,#[serde(default)]pub tags:Vec<String>}
#[derive(Debug,Deserialize)]struct Browse{query:Option<String>,provider:Option<String>,project:Option<String>,status:Option<String>,limit:Option<i64>,offset:Option<i64>}
#[derive(Debug,Deserialize)]struct Patch{status:Option<String>,tags:Option<Vec<String>>,title:Option<String>,content:Option<String>,revision:i64}
#[derive(Debug,Deserialize)]struct Link{target_id:String,relation:String}
#[derive(Debug,Deserialize)]struct SyncRequest{ids:Vec<String>,connector_id:Option<String>}
#[derive(Debug,Deserialize)]struct Retrieve{query:String,project:Option<String>,provider:Option<String>,limit:Option<usize>,connector_id:Option<String>}

pub fn routes()->Router<Arc<ServerState>>{
    Router::new()
        .route("/api/v1/rag/status",get(status))
        .route("/api/v1/rag/chunks",get(list).post(ingest))
        .route("/api/v1/rag/chunks/{id}",get(detail).patch(patch))
        .route("/api/v1/rag/chunks/{id}/links",get(links).post(link))
        .route("/api/v1/rag/sync",post(sync))
        .route("/api/v1/rag/retrieve",post(retrieve))
        .route("/api/v1/rag/remote-search",post(remote_search))
        .route("/api/v1/rag/connectors",get(connectors))
        .route("/api/v1/rag/task/{id}/capture",post(capture))
}
pub async fn migrate(pool:&SqlitePool)->Result<(),sqlx::Error>{
    for stmt in [
      "CREATE TABLE IF NOT EXISTS rag_chunks(id TEXT PRIMARY KEY,source_key TEXT NOT NULL,source_kind TEXT NOT NULL,provider TEXT NOT NULL,project TEXT NOT NULL,task_id TEXT,title TEXT NOT NULL,content TEXT NOT NULL,tags_json TEXT NOT NULL,status TEXT NOT NULL,remote_id TEXT,fingerprint TEXT NOT NULL UNIQUE,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,revision INTEGER NOT NULL DEFAULT 1)",
      "CREATE INDEX IF NOT EXISTS ix_rag_status ON rag_chunks(status,project,provider,created_at)",
      "CREATE INDEX IF NOT EXISTS ix_rag_task ON rag_chunks(task_id)",
      "CREATE TABLE IF NOT EXISTS rag_links(source_id TEXT NOT NULL,target_id TEXT NOT NULL,relation TEXT NOT NULL,PRIMARY KEY(source_id,target_id,relation),FOREIGN KEY(source_id) REFERENCES rag_chunks(id),FOREIGN KEY(target_id) REFERENCES rag_chunks(id))",
      "CREATE TABLE IF NOT EXISTS rag_sync_audit(chunk_id TEXT NOT NULL,event TEXT NOT NULL,message TEXT NOT NULL,created_at INTEGER NOT NULL)",
      "CREATE TABLE IF NOT EXISTS rag_ai_revisions(id TEXT NOT NULL, revision INTEGER NOT NULL, snapshot TEXT NOT NULL, operation TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(id, revision))",
      "CREATE TABLE IF NOT EXISTS rag_ai_previews(id TEXT PRIMARY KEY, operation TEXT NOT NULL, payload_json TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed_at INTEGER)"
    ] {sqlx::query(stmt).execute(pool).await?;}
    let info=sqlx::query("PRAGMA table_info(rag_chunks)").fetch_all(pool).await?;
    if !info.iter().any(|r|r.get::<String,_>("name")=="summary") {
        sqlx::query("ALTER TABLE rag_chunks ADD COLUMN summary TEXT NOT NULL DEFAULT ''").execute(pool).await?;
    }
    Ok(())
}
/// Creates review-only records; redacted, fingerprinted and idempotent across retries.
pub async fn ingest_to_db(pool:&SqlitePool,v:&Ingest)->Result<Vec<String>,String>{
    if v.source_key.trim().is_empty()||v.source_key.len()>240{return Err("source_key required (max 240)".into())}
    if v.title.trim().is_empty()||v.title.len()>240{return Err("title required (max 240)".into())}
    if v.provider.trim().is_empty()||v.provider.len()>40||v.project.len()>100||v.source_kind.len()>60{return Err("invalid provider/project/source_kind".into())}
    if v.content.trim().is_empty()||v.content.chars().count()>80000{return Err("content required (max 80000 chars)".into())}
    let text=redact(&v.content);
    let items=chunks(&text,850,110);
    let mut ids=Vec::new();
    let tags=serde_json::to_string(&clean_tags(&v.tags)).map_err(|e|e.to_string())?;
    for (i,c) in items.iter().enumerate() {
        let fp=hash(&format!("{}\0{}\0{}\0{}",v.provider,v.project,v.source_key,c));
        let id=format!("rag-{}",&fp[..24]);
        sqlx::query("INSERT OR IGNORE INTO rag_chunks(id,source_key,source_kind,provider,project,task_id,title,content,tags_json,status,fingerprint,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,'draft',?,?,?)")
            .bind(&id).bind(&v.source_key).bind(&v.source_kind).bind(&v.provider).bind(&v.project).bind(&v.task_id)
            .bind(format!("{} · {}",v.title,i+1)).bind(c).bind(&tags).bind(&fp).bind(now()).bind(now()).execute(pool).await.map_err(|e|e.to_string())?;
        ids.push(id);
    }
    Ok(ids)
}
async fn status(State(s):State<Arc<ServerState>>,h:HeaderMap)->ResultJson{
    auth(&h,&s)?;
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM rag_chunks").fetch_one(&s.pool).await.map_err(fault)?;
    let drafts:i64=sqlx::query_scalar("SELECT count(*) FROM rag_chunks WHERE status='draft'").fetch_one(&s.pool).await.map_err(fault)?;
    let queued:i64=sqlx::query_scalar("SELECT count(*) FROM rag_chunks WHERE status='approved'").fetch_one(&s.pool).await.map_err(fault)?;
    Ok(Json(json!({"total":count,"draft":drafts,"approved":queued,"remote_configured":available_connectors().iter().any(|x|x.upsert_url.is_some()&&std::env::var(&x.token_env).ok().is_some_and(|v|v.len()>=16)),"remote_search_configured":available_connectors().iter().any(|x|x.search_url.is_some()&&std::env::var(&x.token_env).ok().is_some_and(|v|v.len()>=16))})))
}
async fn list(State(s):State<Arc<ServerState>>,h:HeaderMap,Query(q):Query<Browse>)->ResultJson{
    auth(&h,&s)?;
    let limit=q.limit.unwrap_or(100).clamp(1,200);
    let offset=q.offset.unwrap_or(0).clamp(0,1000000);
    let search=q.query.as_ref().filter(|q|!q.trim().is_empty()).map(|q|format!("%{}%",q.trim().to_lowercase()));
    let rows=sqlx::query_as::<_,Chunk>("SELECT * FROM rag_chunks WHERE (?1 IS NULL OR provider=?1) AND (?2 IS NULL OR project=?2) AND (?3 IS NULL OR status=?3) AND (?4 IS NULL OR lower(title) LIKE ?4 OR lower(content) LIKE ?4 OR lower(tags_json) LIKE ?4) ORDER BY updated_at DESC,id DESC LIMIT ?5 OFFSET ?6")
      .bind(q.provider.as_deref()).bind(q.project.as_deref()).bind(q.status.as_deref()).bind(search).bind(limit).bind(offset)
      .fetch_all(&s.pool).await.map_err(fault)?;
    Ok(Json(json!(rows)))
}
async fn ingest(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(v):Json<Ingest>)->ResultJson{
    auth(&h,&s)?;
    let ids=ingest_to_db(&s.pool,&v).await.map_err(bad)?;
    Ok(Json(json!({"chunk_ids":ids,"status":"draft","auto_sync":false})))
}
async fn detail(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>)->ResultJson{
    auth(&h,&s)?;
    let row=sqlx::query_as::<_,Chunk>("SELECT * FROM rag_chunks WHERE id=?").bind(id).fetch_optional(&s.pool).await.map_err(fault)?;
    row.map(|x|Json(json!(x))).ok_or((StatusCode::NOT_FOUND,"chunk missing".into()))
}
async fn patch(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(v):Json<Patch>)->ResultJson{
    auth(&h,&s)?;
    let current=sqlx::query_as::<_,Chunk>("SELECT * FROM rag_chunks WHERE id=?").bind(&id).fetch_optional(&s.pool).await.map_err(fault)?.ok_or((StatusCode::NOT_FOUND,"chunk missing".into()))?;
    if let Some(status)=&v.status{if !["draft","approved","archived"].contains(&status.as_str()){return Err(bad("invalid review status"))}}
    if v.content.as_ref().is_some_and(|x|x.chars().count()>5000||x.trim().is_empty()){return Err(bad("invalid chunk content"))}
    if v.title.as_ref().is_some_and(|x|x.len()>240||x.trim().is_empty()){return Err(bad("invalid title"))}
    let edited=v.content.is_some()||v.tags.is_some()||v.title.is_some();
    let new_status=v.status.unwrap_or_else(||if edited {"draft".into()}else{current.status.clone()});
    if current.status=="synced"&&new_status=="approved" {return Err(bad("synced items need a new review revision"))}
    let new_content=v.content.map(|x|redact(&x)).unwrap_or(current.content);
    // The chunk keeps its stable ID, but the content fingerprint changes with edits.
    let new_fingerprint=hash(&format!("{}\0{}\0{}\0{}",current.provider,current.project,current.source_key,new_content));
    let changed=sqlx::query("UPDATE rag_chunks SET status=?,tags_json=?,title=?,content=?,fingerprint=?,remote_id=CASE WHEN ? THEN NULL ELSE remote_id END,revision=revision+1,updated_at=? WHERE id=? AND revision=?")
      .bind(new_status).bind(v.tags.as_ref().map(|x|serde_json::to_string(&clean_tags(x)).unwrap_or_default()).unwrap_or(current.tags_json))
      .bind(v.title.unwrap_or(current.title)).bind(new_content).bind(new_fingerprint).bind(edited)
      .bind(now()).bind(&id).bind(v.revision).execute(&s.pool).await.map_err(fault)?;
    if changed.rows_affected()==0{return Err((StatusCode::CONFLICT,"record changed; refresh before editing".into()))}
    Ok(Json(json!({"ok":true,"status": "updated"})))
}
async fn links(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>)->ResultJson{
    auth(&h,&s)?;
    let rows=sqlx::query("SELECT target_id,relation FROM rag_links WHERE source_id=? ORDER BY relation,target_id").bind(id).fetch_all(&s.pool).await.map_err(fault)?;
    Ok(Json(json!(rows.into_iter().map(|r|json!({"target_id":r.get::<String,_>(0),"relation":r.get::<String,_>(1)})).collect::<Vec<_>>())))
}
async fn link(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>,Json(v):Json<Link>)->ResultJson{
    auth(&h,&s)?;
    if id==v.target_id||!["related","supersedes","depends_on","derived_from"].contains(&v.relation.as_str()){return Err(bad("invalid link"))}
    for key in [&id,&v.target_id] {
        let exists:i64=sqlx::query_scalar("SELECT count(*) FROM rag_chunks WHERE id=?").bind(key).fetch_one(&s.pool).await.map_err(fault)?;
        if exists==0{return Err(bad("missing source or target"))}
    }
    sqlx::query("INSERT OR IGNORE INTO rag_links(source_id,target_id,relation) VALUES(?,?,?)").bind(id).bind(v.target_id).bind(v.relation).execute(&s.pool).await.map_err(fault)?;
    Ok(Json(json!({"ok":true})))
}
async fn retrieve(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(v):Json<Retrieve>)->ResultJson{
    auth(&h,&s)?;if v.query.trim().is_empty()||v.query.len()>400{return Err(bad("invalid query"))}
    let parts=v.query.to_lowercase().split_whitespace().map(str::to_string).collect::<Vec<_>>();
    let terms=if parts.is_empty(){vec![v.query.to_lowercase()]}else{parts};
    let rows=sqlx::query_as::<_,Chunk>("SELECT * FROM rag_chunks WHERE status IN ('approved','synced') ORDER BY updated_at DESC LIMIT 2000").fetch_all(&s.pool).await.map_err(fault)?;
    let mut scored=rows.into_iter().filter(|r|v.project.as_ref().is_none_or(|x|x==&r.project)&&v.provider.as_ref().is_none_or(|x|x==&r.provider))
      .filter_map(|r|{let text=format!("{} {}",r.title,r.content).to_lowercase();let score=terms.iter().filter(|t|text.contains(t.as_str())).count();if score>0{Some((score,r))}else{None}}).collect::<Vec<_>>();
    scored.sort_by(|a,b|b.0.cmp(&a.0).then(b.1.updated_at.cmp(&a.1.updated_at)));
    Ok(Json(json!(scored.into_iter().take(v.limit.unwrap_or(6).clamp(1,20)).map(|(score,r)|json!({"score":score,"chunk":r})).collect::<Vec<_>>())))
}
pub async fn capture_task_if_opted(pool:&SqlitePool,id:&str)->Result<Vec<String>,String>{
    let t=sqlx::query("SELECT title,prompt,provider,repo_alias,context_json,output,state FROM tasks WHERE id=?").bind(id).fetch_optional(pool).await.map_err(|e|e.to_string())?.ok_or("task not found")?;
    let state:String=t.get("state");if !["completed","dispatched"].contains(&state.as_str()){return Err("task not finished".into())}
    let context:String=t.get("context_json");
    let policy:Value=serde_json::from_str(&context).unwrap_or(Value::Null);
    if !policy.get("rag_capture").and_then(Value::as_bool).unwrap_or(false){return Err("RAG capture not opted in".into())}
    let prompt:String=t.get("prompt");let output:Option<String>=t.get("output");
    let provider:String=t.get("provider");let project:Option<String>=t.get("repo_alias");let title:String=t.get("title");
    // Selected RAG excerpts are only references, never re-imported as new evidence.
    let has_refs=policy.get("rag_refs").and_then(Value::as_array).is_some_and(|x|!x.is_empty());
    let original_prompt=if has_refs {prompt.split("\n\n### 已审核的个人 RAG 参考").next().unwrap_or(&prompt).to_string()}else{prompt};
    let mut ids=Vec::new();
    for (kind,role,content) in [
        ("agent_input","input",original_prompt),
        (if state=="dispatched"{"dispatch_receipt"}else{"agent_output"},"output",output.unwrap_or_default()),
    ]{
        // ≤40k chars per ingestion avoids silently dropping long source conversations.
        let chars:Vec<char>=content.chars().collect();
        for (i,window) in chars.chunks(40_000).enumerate(){
            let part:String=window.iter().collect();if part.trim().is_empty(){continue}
            let v=Ingest{source_key:format!("task:{id}:{role}:{i}"),source_kind:kind.into(),provider:provider.clone(),project:project.clone().unwrap_or_else(||"unassigned".into()),task_id:Some(id.into()),title:format!("{} · {} {}",title,role,i+1),content:part,tags:vec!["agent-context".into(),role.into()]};
            ids.extend(ingest_to_db(pool,&v).await?);
        }
    }
    Ok(ids)
}
async fn capture(State(s):State<Arc<ServerState>>,h:HeaderMap,Path(id):Path<String>)->ResultJson{
    auth(&h,&s)?;
    let ids=capture_task_if_opted(&s.pool,&id).await.map_err(bad)?;
    Ok(Json(json!({"chunk_ids":ids,"status":"draft","auto_sync":false})))
}
#[derive(Debug,Clone,Deserialize)]
struct ConnectorProfile { id:String, name:String, upsert_url:Option<String>, search_url:Option<String>, token_env:String }
fn available_connectors()->Vec<ConnectorProfile>{
    let mut profiles:Vec<ConnectorProfile>=std::env::var("ARGO_RAG_CONNECTORS_JSON").ok()
       .and_then(|v|serde_json::from_str(&v).ok()).unwrap_or_default();
    if profiles.is_empty(){
        let upsert=std::env::var("ARGO_RAG_UPSERT_URL").ok();
        let search=std::env::var("ARGO_RAG_SEARCH_URL").ok();
        if upsert.is_some()||search.is_some(){profiles.push(ConnectorProfile{id:"default".into(),name:"默认知识库".into(),upsert_url:upsert,search_url:search,token_env:"ARGO_RAG_API_TOKEN".into()});}
    }
    profiles.retain(|c|{
        !c.id.is_empty()&&c.id.len()<=60&&c.id.chars().all(|x|x.is_ascii_alphanumeric()||x=='-'||x=='_')
        && c.token_env.len()<=100&&c.token_env.chars().all(|x|x.is_ascii_alphanumeric()||x=='_')
        && c.upsert_url.as_deref().is_none_or(|s|s.starts_with("https://")&&!s.contains('@'))
        && c.search_url.as_deref().is_none_or(|s|s.starts_with("https://")&&!s.contains('@'))
    });
    profiles.truncate(25);profiles
}
fn select_connector(requested:Option<&str>,needs_search:bool)->Result<(ConnectorProfile,String,String),E>{
    let profiles=available_connectors();
    let chosen=profiles.iter().find(|p|p.id==requested.unwrap_or("default"))
        .or_else(||if requested.is_none(){profiles.first()}else{None})
        .ok_or((StatusCode::PRECONDITION_FAILED,"RAG connector missing or not configured".into()))?.clone();
    let url=(if needs_search{&chosen.search_url}else{&chosen.upsert_url}).clone()
        .ok_or((StatusCode::PRECONDITION_FAILED,"connector endpoint not configured".into()))?;
    let token=std::env::var(&chosen.token_env).map_err(|_|(StatusCode::PRECONDITION_FAILED,"connector secret missing".into()))?;
    if token.len()<16{return Err((StatusCode::PRECONDITION_FAILED,"connector secret too short".into()))}
    Ok((chosen,url,token))
}
async fn connectors(State(s):State<Arc<ServerState>>,h:HeaderMap)->ResultJson{
    auth(&h,&s)?;
    let all=available_connectors().iter().map(|c|json!({"id":c.id,"name":c.name,"can_sync":c.upsert_url.is_some(),"can_search":c.search_url.is_some()})).collect::<Vec<_>>();
    Ok(Json(json!(all)))
}
async fn remote_search(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(q):Json<Retrieve>)->ResultJson{
    auth(&h,&s)?;

    if q.query.trim().is_empty()||q.query.len()>400{return Err(bad("invalid query"))}
    let (chosen,url,token)=select_connector(q.connector_id.as_deref(),true)?;
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(16)).redirect(reqwest::redirect::Policy::none()).build().map_err(fault)?;
    let response=client.post(url).bearer_auth(token).json(&json!({"query":q.query,"project":q.project,"provider":q.provider,"limit":q.limit.unwrap_or(8).clamp(1,20)})).send().await
        .map_err(|_|(StatusCode::BAD_GATEWAY,"remote RAG transport error".into()))?;
    if !response.status().is_success(){return Err((StatusCode::BAD_GATEWAY,format!("remote RAG returned {}",response.status().as_u16())))}
    let data:Value=response.json().await.map_err(fault)?;
    let records=data.get("results").and_then(Value::as_array).ok_or((StatusCode::BAD_GATEWAY,"remote RAG results missing".into()))?;
    let safe=records.iter().take(20).map(|r|json!({
        "id":r.get("id").and_then(Value::as_str).unwrap_or(""),
        "title":r.get("title").and_then(Value::as_str).unwrap_or("").chars().take(240).collect::<String>(),
        "content":r.get("content").and_then(Value::as_str).unwrap_or("").chars().take(6000).collect::<String>(),
        "provider":r.get("provider").and_then(Value::as_str).unwrap_or("remote"),
        "project":r.get("project").and_then(Value::as_str).unwrap_or("Personal"),
        "tags":r.get("tags").cloned().unwrap_or(json!([]))
    })).collect::<Vec<_>>();
    Ok(Json(json!({"results":safe,"mode":data.get("mode").and_then(Value::as_str).unwrap_or("remote"),"source":chosen.id})))
}
async fn sync(State(s):State<Arc<ServerState>>,h:HeaderMap,Json(req):Json<SyncRequest>)->ResultJson{
    auth(&h,&s)?;
    if req.ids.len()>30||req.ids.is_empty(){return Err(bad("select between 1 and 30 reviewed chunks"))}

    let (_chosen,url,token)=select_connector(req.connector_id.as_deref(),false)?;
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(16)).redirect(reqwest::redirect::Policy::none()).build().map_err(fault)?;
    let mut completed=vec![];let mut failed=vec![];
    for id in &req.ids{
        let row=sqlx::query_as::<_,Chunk>("SELECT * FROM rag_chunks WHERE id=?").bind(id).fetch_optional(&s.pool).await.map_err(fault)?;
        let Some(r)=row else {failed.push(json!({"id":id,"error":"not_found"}));continue};
        if r.status!="approved"{failed.push(json!({"id":id,"error":"not_approved"}));continue}
        let payload=json!({"id":r.id,"source_key":r.source_key,"source_kind":r.source_kind,"provider":r.provider,"project":r.project,"title":r.title,"content":r.content,"summary":r.summary,"tags":serde_json::from_str::<Vec<String>>(&r.tags_json).unwrap_or_default(),"fingerprint":r.fingerprint,"revision":r.revision});
        let response=client.post(&url).bearer_auth(&token).header("Idempotency-Key",&r.fingerprint).json(&payload).send().await;
        match response{
          Ok(resp) if resp.status().is_success()=>{
             let body:Value=resp.json().await.unwrap_or(Value::Null);
             let remote=body.get("id").and_then(Value::as_str).unwrap_or(&r.id).to_string();
             let changed=sqlx::query("UPDATE rag_chunks SET status='synced',remote_id=?,updated_at=?,revision=revision+1 WHERE id=? AND status='approved' AND revision=?")
                  .bind(remote).bind(now()).bind(&r.id).bind(r.revision).execute(&s.pool).await.map_err(fault)?;
             if changed.rows_affected()==1{completed.push(r.id)}else{failed.push(json!({"id":r.id,"error":"concurrent_change_after_remote_commit"}))}
          },
          Ok(resp)=>failed.push(json!({"id":r.id,"error":format!("remote_http_{}",resp.status().as_u16())})),
          Err(_)=>failed.push(json!({"id":r.id,"error":"transport_error"})),
        }
    }
    for id in &completed {
        let _=sqlx::query("INSERT INTO rag_sync_audit(chunk_id,event,message,created_at) VALUES(?,'synced','remote accepted',?)")
           .bind(id).bind(now()).execute(&s.pool).await;
    }
    for item in &failed {
        let id=item.get("id").and_then(Value::as_str).unwrap_or("");
        let why=item.get("error").and_then(Value::as_str).unwrap_or("unknown");
        let _=sqlx::query("INSERT INTO rag_sync_audit(chunk_id,event,message,created_at) VALUES(?,'failed',?,?)")
           .bind(id).bind(why).bind(now()).execute(&s.pool).await;
    }
    Ok(Json(json!({"synced":completed,"failed":failed})))
}
#[cfg(test)] mod tests{
    use super::*;
    #[test]fn unicode_segmentation_is_safe(){let a="汉字😀中。".repeat(700);let c=chunks(&a,850,110);assert!(c.len()>1);assert!(c.iter().all(|v|v.chars().count()<=850));}
    #[test]fn secrets_are_redacted(){assert!(!redact("Authorization: Bearer mysecretvalue00001").contains("mysecretvalue00001"));}
    #[test]fn stable_fingerprint(){assert_eq!(hash("a"),hash("a"));}
}
