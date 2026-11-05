use crate::model::*;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest,Sha256};
use std::{collections::HashMap, path::{Path,PathBuf}, process::Stdio, sync::Arc, time::Duration};
use tokio::{io::{AsyncBufReadExt,BufReader},process::Command,sync::{mpsc,Semaphore}};

#[derive(Clone)]struct Worker {
    base:String,token:String,id:String,client:Client,repos:HashMap<String,PathBuf>,root:PathBuf,
    demo:bool,
}
fn paths()->HashMap<String,PathBuf>{
    std::env::var("ARGO_REPOS").unwrap_or_default().split(';').filter_map(|s|s.split_once('='))
        .filter_map(|(name,path)|{
            let p=PathBuf::from(path);
            if !name.is_empty() && p.is_absolute() && p.is_dir(){Some((name.to_string(),p))}else{None}
        }).collect()
}
fn has_command(name:&str)->bool{
    std::env::var_os("PATH").into_iter().flat_map(|p|std::env::split_paths(&p).collect::<Vec<_>>())
        .any(|folder| ["", ".exe", ".cmd", ".bat"].iter().any(|ext|folder.join(format!("{name}{ext}")).is_file()))
}
impl Worker {
    async fn post(&self,path:&str,val:Value)->Result<Value,String>{
        let res=self.client.post(format!("{}{}",self.base,path)).bearer_auth(&self.token).json(&val)
            .send().await.map_err(|e|e.to_string())?;
        let st=res.status();let txt=res.text().await.map_err(|e|e.to_string())?;
        if !st.is_success(){return Err(format!("HTTP {st}: {txt}"))}
        serde_json::from_str(&txt).map_err(|e|format!("invalid response: {e}"))
    }
    async fn register(&self,max:usize)->Result<(),String>{
        let providers:Vec<&str>=SUPPORTED.iter().copied().filter(|p|self.demo || (if *p=="star" {std::env::var("ARGO_STAR_DISPATCH_URL").ok().is_some_and(|v|v.starts_with("https://")) && std::env::var("ARGO_STAR_SERVICE_TOKEN").ok().is_some_and(|v|v.len()>=16)} else {has_command(match *p {"minimax"=>"hermes",x=>x})})).collect();
        let val=json!({"id":self.id,"hostname":std::env::var("COMPUTERNAME").or_else(|_|std::env::var("HOSTNAME")).unwrap_or_else(|_|"worker".into()),
        "os":std::env::consts::OS,"providers":providers,"repos":self.repos.keys().collect::<Vec<_>>(),"max_parallel":max});
        self.post("/api/v1/worker/register",val).await.map(|_|())
    }
    async fn report(&self,task:&str,stream:&str,content:&str){
        let limited: String=content.chars().take(4000).collect();
        let _=self.post(&format!("/api/v1/worker/tasks/{task}/event"),json!({"node_id":self.id,"stream":stream,"content":limited})).await;
    }
    async fn finish(&self,task:&str,state:&str,output:String,error:Option<String>){
        if let Err(e)=self.post(&format!("/api/v1/worker/tasks/{task}/finish"),json!({"node_id":self.id,"status":state,"output":output,"error":error})).await{
            tracing::warn!(task=%task,error=%e,"failed to report completion");
        }
    }
    async fn save_artifact(&self,task:&str,name:&str,bytes:Vec<u8>)->Result<(),String>{
        if bytes.is_empty()||bytes.len()>10*1024*1024{return Err("artifact must be 1..10 MiB".into())}
        let response=self.client.post(format!("{}/api/v1/blobs",self.base)).bearer_auth(&self.token).body(bytes.clone()).send().await.map_err(|e|e.to_string())?;
        let ok=response.error_for_status().map_err(|e|e.to_string())?;
        let blob:Value=ok.json().await.map_err(|e|e.to_string())?;
        self.post(&format!("/api/v1/worker/tasks/{task}/artifact"),json!({"node_id":self.id,"name":name,"sha256":blob["sha256"],"size":bytes.len()})).await.map(|_|())
    }
    async fn blob(&self,sha:&str)->Result<Vec<u8>,String>{
        let res=self.client.get(format!("{}/api/v1/blobs/{}",self.base,sha)).bearer_auth(&self.token).send().await.map_err(|e|e.to_string())?;
        if !res.status().is_success(){return Err(format!("failed fetching blob {sha}: {}",res.status()))}
        let bytes=res.bytes().await.map_err(|e|e.to_string())?;
        let digest=hex::encode(Sha256::digest(&bytes));
        if digest!=sha{return Err("blob hash mismatch".into())}
        Ok(bytes.to_vec())
    }
}
pub async fn run(server:String,id:String,token:String,max_parallel:usize)->Result<(),Box<dyn std::error::Error>>{
    if token.len()<24 || id.trim().is_empty(){return Err("worker token and id required".into())}
    let worker=Worker {
        base:server.trim_end_matches('/').into(),id,token,client:Client::builder().timeout(Duration::from_secs(100)).build()?,
        repos:paths(),root:PathBuf::from(std::env::var("ARGO_RUNS_DIR").unwrap_or_else(|_|"./data/runs".into())),
        demo:std::env::var("ARGO_DEMO_MODE").as_deref()==Ok("1"),
    };
    tokio::fs::create_dir_all(&worker.root).await?;
    let parallel=max_parallel.clamp(1,64);
    let semaphore=Arc::new(Semaphore::new(parallel));
    let mut last_register=std::time::Instant::now()-Duration::from_secs(30);
    tracing::info!(node=%worker.id,server=%worker.base,parallel,"Argo worker started");
    loop {
        if last_register.elapsed()>Duration::from_secs(12){
            if let Err(e)=worker.register(parallel).await{tracing::warn!(error=%e,"register failed");}
            last_register=std::time::Instant::now();
        }
        // File owners perform snapshot export and explicitly approved patch application.
        // These are separate from ordinary agent task execution.
        if let Ok(v)=worker.post("/api/v1/worker/source/claim",json!({"node_id":worker.id})).await {
            if !v.is_null(){if let Ok(task)=serde_json::from_value::<TaskRow>(v){
                let w=worker.clone();tokio::spawn(async move{prepare_source(w,task).await;});
            }}
        }
        if let Ok(v)=worker.post("/api/v1/worker/apply/claim",json!({"node_id":worker.id})).await {
            if !v.is_null(){if let Ok(task)=serde_json::from_value::<TaskRow>(v){
                let w=worker.clone();tokio::spawn(async move{apply_approved(w,task).await;});
            }}
        }
        if let Ok(permit)=semaphore.clone().try_acquire_owned(){
            match worker.post("/api/v1/worker/claim",json!({"node_id":worker.id})).await {
                Ok(v) if !v.is_null()=>{
                    match serde_json::from_value::<TaskRow>(v){
                        Ok(task)=>{
                            let w=worker.clone();
                            tokio::spawn(async move{
                                let result=execute(w.clone(),task.clone()).await;
                                if let Err(e)=result{
                                    w.report(&task.id,"system",&format!("Execution error: {e}")).await;
                                    w.finish(&task.id,"failed",String::new(),Some(e)).await;
                                }
                                drop(permit);
                            });
                        },
                        Err(e)=>tracing::warn!(error=%e,"bad task payload")
                    }
                },
                Ok(_)=>{},
                Err(e)=>tracing::warn!(error=%e,"claim failed"),
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn execute(w:Worker,t:TaskRow)->Result<(),String>{
    let base=w.root.join(&w.id).join(&t.id);
    tokio::fs::create_dir_all(&base).await.map_err(|e|e.to_string())?;
    let input_dir=base.join("input");tokio::fs::create_dir_all(&input_dir).await.map_err(|e|e.to_string())?;
    let attachments:Vec<Attachment>=serde_json::from_str(&t.attachments_json).map_err(|e|e.to_string())?;
    let mut names=Vec::new();
    for a in attachments {
        if a.name.contains('/') || a.name.contains('\\') || a.name=="." || a.name==".." {return Err("invalid file name".into())}
        let bytes=w.blob(&a.sha256).await?;
        let dest=input_dir.join(&a.name);
        tokio::fs::write(&dest,bytes).await.map_err(|e|e.to_string())?;
        names.push(dest);
    }
    let output_dir=base.join("output");tokio::fs::create_dir_all(&output_dir).await.map_err(|e|e.to_string())?;
    let cwd=base.join("workspace");
    if t.source_state=="ready" {
        // Remote executor receives a Git-archive snapshot produced by the file owner's worker.
        let sha=t.source_sha.as_deref().ok_or("remote source hash missing")?;
        let bundle=w.blob(sha).await?;
        tokio::fs::create_dir_all(&cwd).await.map_err(|e|e.to_string())?;
        unpack_archive(&cwd,bundle).await?;
        git(&cwd,&["init","-q"]).await?;
        git(&cwd,&["add","-A"]).await?;
        git(&cwd,&["-c","user.name=Argo Snapshot","-c","user.email=argo@localhost","commit","--allow-empty","-q","-m","Owner source snapshot"]).await?;
    } else if let Some(alias)=&t.repo_alias {
        let root=w.repos.get(alias).ok_or_else(||format!("repo alias not available on this node: {alias}"))?;
        // This detached Git worktree is the task's write boundary. Never run agents in the base checkout.
        let output=Command::new("git").args(["-C",root.to_str().ok_or("invalid repo path")?,"worktree","add","--detach",cwd.to_str().ok_or("invalid worktree path")?,"HEAD"]).output().await.map_err(|e|e.to_string())?;
        if !output.status.success(){return Err(format!("git worktree creation failed: {}",String::from_utf8_lossy(&output.stderr)))}
    } else {tokio::fs::create_dir_all(&cwd).await.map_err(|e|e.to_string())?}
    // Scope belongs to the selected repository on this worker. This changes the
    // process working directory; it is NOT a full OS-level filesystem sandbox.
    let ctx:Value=serde_json::from_str(&t.context_json).map_err(|e|e.to_string())?;
    let mut execution_cwd=cwd.clone();
    if let Some(scope)=ctx.get("argo_file_scope").and_then(Value::as_str).filter(|s|!s.trim().is_empty()) {
        if t.repo_alias.is_none(){return Err("a repository is required for a file scope".into())}
        if scope.starts_with('/') || scope.contains('\\') || scope.split('/').any(|x|x==".."||x==".") {return Err("invalid file scope".into())}
        let requested=cwd.join(scope);
        let root=tokio::fs::canonicalize(&cwd).await.map_err(|e|e.to_string())?;
        let resolved=tokio::fs::canonicalize(&requested).await.map_err(|e|format!("file scope must exist inside repository: {e}"))?;
        if !resolved.starts_with(&root){return Err("file scope escapes task worktree".into())}
        execution_cwd=if resolved.is_file(){resolved.parent().ok_or("invalid file target")?.to_path_buf()}else{resolved};
    }
    let mut note=if names.is_empty(){String::new()}else{format!("\n\nFiles attached to this task (local read-only references; do not delete):\n{}",names.iter().map(|x|x.display().to_string()).collect::<Vec<_>>().join("\n"))};
    if !["multica","orca"].contains(&t.provider.as_str()) {
        note.push_str(&format!("\n\nTo return non-Git generated files to the controller, save them in this output directory (max 20 files, 10MiB each): {}",output_dir.display()));
    }
    let mut normalized=t.clone();normalized.prompt.push_str(&note);
    let prompt_file=base.join("prompt.md");
    tokio::fs::write(&prompt_file,&normalized.prompt).await.map_err(|e|e.to_string())?;
    let prompt_path=prompt_file.to_str().ok_or("invalid prompt path")?;
    // STAR is not yet released as a public CLI. Dispatch only to the explicit owner-defined HTTPS webhook.
    if t.provider=="star" {
        if w.demo {w.report(&t.id,"system","DEMO · STAR connector simulated; no HTTP sent").await;w.finish(&t.id,"dispatched","DEMO STAR dispatch".into(),None).await;return Ok(())}
        let url=std::env::var("ARGO_STAR_DISPATCH_URL").map_err(|_|"STAR HTTPS dispatch endpoint is not configured")?;
        if !url.starts_with("https://") || url.contains('@') {return Err("STAR dispatch URL must be HTTPS".into())}
        let token=std::env::var("ARGO_STAR_SERVICE_TOKEN").map_err(|_|"STAR service token is not configured")?;
        if token.len()<16{return Err("STAR service token too short".into())}
        let payload=json!({"idempotency_key":t.id,"source":"argo","task_id":t.id,"title":t.title,"prompt":normalized.prompt,
          "project_id":ctx.get("star_project_id"),"ticket_id":ctx.get("star_ticket_id"),"mode":ctx.get("star_mode")});
        let res=w.client.post(url).bearer_auth(token).json(&payload).send().await.map_err(|_|"STAR dispatch transport failed")?;
        if !res.status().is_success(){return Err(format!("STAR endpoint rejected: HTTP {}",res.status().as_u16()))}
        let response:Value=res.json().await.map_err(|_|"STAR JSON acknowledgement missing")?;
        let remote_id=response.get("id").and_then(Value::as_str).unwrap_or("unverified");
        w.report(&t.id,"system",&format!("STAR dispatch acknowledged: {remote_id}; delivery does not indicate task completion")).await;
        w.finish(&t.id,"dispatched",format!("STAR dispatched remote ID: {remote_id}"),None).await;
        return Ok(())
    }
    let mut plan=invocation(&normalized,prompt_path)?;
    if t.provider=="multica" && !names.is_empty() && !normalized.context_json.contains("discover_") {
        for attachment in &names {plan.args.extend(["--attachment".to_string(),attachment.to_string_lossy().into_owned()]);}
    }
    w.report(&t.id,"system",&format!("Node: {} | Adapter: {} | Workspace: {} | Mode: {}",w.id,plan.program,execution_cwd.display(),if plan.acceptance_only{"dispatch"}else{"execute"})).await;
    if w.demo {
        for txt in ["Argo demo mode: request validated", "Node received prompt & file manifest", "Adapter would be invoked on the selected machine"] {
            w.report(&t.id,"stdout",txt).await;
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
        w.finish(&t.id,if plan.acceptance_only{"dispatched"}else{"completed"},"DEMO: no third-party CLI invoked".into(),None).await;
        return Ok(())
    }
    let mut cmd=Command::new(plan.program);
    cmd.args(&plan.args).current_dir(&execution_cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // No shell interpolation, no secret forwarding. CLI credentials stay on this worker.
    let mut child=cmd.spawn().map_err(|e|format!("{} launch failed: {e}",plan.program))?;
    let (tx,mut rx)=mpsc::channel::<(String,String)>(128);
    if let Some(out)=child.stdout.take(){let tx=tx.clone();tokio::spawn(async move{
        let mut lines=BufReader::new(out).lines();
        while let Ok(Some(line))=lines.next_line().await{if tx.send(("stdout".into(),line)).await.is_err(){break}}
    });}
    if let Some(out)=child.stderr.take(){let tx=tx.clone();tokio::spawn(async move{
        let mut lines=BufReader::new(out).lines();
        while let Ok(Some(line))=lines.next_line().await{if tx.send(("stderr".into(),line)).await.is_err(){break}}
    });}
    drop(tx);
    let mut combined=String::new();let mut ticker=tokio::time::interval(Duration::from_secs(10));
    let mut cancelled=false;
    let status=loop {
        tokio::select! {
            maybe=rx.recv()=>{
                if let Some((stream,line))=maybe {
                    w.report(&t.id,&stream,&line).await;
                    if combined.len()<95_000 {combined.push_str(&line.chars().take(4000).collect::<String>());combined.push('\n');}
                }
            }
            result=child.wait()=>{break result.map_err(|e|e.to_string())?;}
            _=ticker.tick()=>{
                let _=w.post(&format!("/api/v1/worker/tasks/{}/renew",t.id),json!({"node_id":w.id})).await;
                if let Ok(v)=w.post(&format!("/api/v1/worker/tasks/{}/state",t.id),json!({"node_id":w.id})).await {
                    if v.get("state").and_then(Value::as_str)==Some("cancelling"){
                        cancelled=true;let _=child.kill().await;break child.wait().await.map_err(|e|e.to_string())?;
                    }
                }
            }
        }
    };
    while let Ok((stream,line))=rx.try_recv(){w.report(&t.id,&stream,&line).await;if combined.len()<95_000{combined.push_str(&line);combined.push('\n')}}
    // Synchronize tracked AND newly created files (intent-to-add) as reviewable Git patch.
    if !cancelled && !["multica","orca"].contains(&t.provider.as_str()) {
        if t.repo_alias.is_some(){
            let _=Command::new("git").arg("-C").arg(&cwd).args(["add","-N","."]).output().await;
            if let Ok(o)=Command::new("git").arg("-C").arg(&cwd).args(["diff","HEAD","--binary"]).output().await {
                if o.status.success() && !o.stdout.is_empty() && o.stdout.len()<=10*1024*1024 {
                    if let Err(e)=w.save_artifact(&t.id,"changes.patch",o.stdout).await {w.report(&t.id,"system",&format!("artifact upload failed: {e}")).await}
                }
            }
        }
        if let Ok(mut entries)=tokio::fs::read_dir(&output_dir).await {
            let mut count=0;
            while count<19 {
                match entries.next_entry().await {
                    Ok(Some(entry))=>{
                        if let Ok(meta)=entry.metadata().await{
                            if !meta.is_file() || meta.len()>10*1024*1024 || meta.len()==0 {continue}
                            let entry_name=entry.file_name();
                            if let Some(name)=entry_name.to_str(){
                                let name=name.to_string();
                                if let Ok(bytes)=tokio::fs::read(entry.path()).await {
                                    if let Err(e)=w.save_artifact(&t.id,&name,bytes).await{w.report(&t.id,"system",&format!("artifact upload failed: {e}")).await}
                                    count+=1;
                                }
                            }
                        }
                    },
                    _=>break
                }
            }
        }
    }
    let mut followup_error=None;
    if status.success() && !cancelled && t.provider=="multica" {
        let ctx:Value=serde_json::from_str(&t.context_json).map_err(|e|e.to_string())?;
        if let (Some(issue),Some(assignee),Some(workspace))=(ctx.get("multica_issue_id").and_then(Value::as_str).filter(|x|!x.is_empty()),ctx.get("multica_assignee").and_then(Value::as_str),ctx.get("multica_workspace_id").and_then(Value::as_str)) {
            // Existing Issue: comment is data transfer; explicit assignment requests an actual agent run.
            let second=Command::new("multica").args(["--workspace-id",workspace,"issue","assign",issue,"--to",assignee]).current_dir(&cwd).output().await;
            match second {
                Ok(o) if o.status.success()=>{
                    let msg=String::from_utf8_lossy(&o.stdout).to_string();
                    w.report(&t.id,"system",&format!("Multica assignment submitted: {}",msg.chars().take(2000).collect::<String>())).await;
                    combined.push_str(&msg);
                },
                Ok(o)=>followup_error=Some(format!("Multica comment posted, but assignment failed: {}",String::from_utf8_lossy(&o.stderr))),
                Err(e)=>followup_error=Some(format!("Multica comment posted, but assignment could not start: {e}")),
            }
        }
    }
    let result=if cancelled{"cancelled"} else if !status.success()||followup_error.is_some(){"failed"} else if plan.acceptance_only{"dispatched"}else{"completed"};
    let err=if let Some(e)=followup_error{Some(e)}else if !status.success()&&!cancelled{Some(format!("exit status: {status}"))}else{None};
    w.finish(&t.id,result,combined,err).await;
    Ok(())
}

pub fn repo_summary()->Vec<String>{ paths().into_keys().collect() }
pub fn repo_path_for(alias:&str)->Option<PathBuf>{paths().remove(alias)}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn paths_empty_is_supported(){let _=paths();}
 #[test]fn missing_command_is_false(){assert!(!has_command("argo-impossible-command-12345"));}
 #[test]fn works_with_file_names(){assert!(!Path::new("safe.txt").is_absolute());}
}

// A Git archive exports HEAD contents only, not the owner's Git history or working tree.
async fn git(root:&Path,args:&[&str])->Result<String,String>{
    let output=Command::new("git").arg("-C").arg(root).args(args).output().await.map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(format!("git {:?}: {}",args,String::from_utf8_lossy(&output.stderr)))}
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn scope_allowed(path:&str,scope:&str)->bool {
    if path.starts_with('/')||path.contains('\')||path.split('/').any(|v|v==".."||v=="."){return false}
    scope.is_empty()||path==scope||path.starts_with(&format!("{scope}/"))
}
async fn unpack_archive(root:&Path,bytes:Vec<u8>)->Result<(),String>{
    let dir=root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut archive=tar::Archive::new(std::io::Cursor::new(bytes));
        for result in archive.entries().map_err(|e|e.to_string())? {
            let mut entry=result.map_err(|e|e.to_string())?;
            let kind=entry.header().entry_type();
            if !kind.is_file()&&!kind.is_dir(){return Err("snapshot contains a symlink or unsupported entry".into())}
            let path=entry.path().map_err(|e|e.to_string())?;
            if path.is_absolute()||path.components().any(|c|matches!(c,std::path::Component::ParentDir)){
                return Err("snapshot contains unsafe archive path".into())
            }
            if !entry.unpack_in(&dir).map_err(|e|e.to_string())?{return Err("snapshot path escaped sandbox".into())}
        }
        Ok::<(),String>(())
    }).await.map_err(|e|e.to_string())?
}
async fn prepare_source(w:Worker,t:TaskRow){
    let result=async {
        let alias=t.repo_alias.as_deref().ok_or("source repo alias missing")?;
        let root=w.repos.get(alias).ok_or("source repo missing from file owner node")?;
        if !git(root,&["status","--porcelain"]).await?.is_empty(){return Err("source repo has uncommitted changes; commit or stash before export".to_string())}
        let commit=git(root,&["rev-parse","HEAD"]).await?;
        let ctx:Value=serde_json::from_str(&t.context_json).map_err(|e|e.to_string())?;
        let scope=ctx.get("argo_file_scope").and_then(Value::as_str).unwrap_or("");
        let path=w.root.join(&w.id).join(&t.id);tokio::fs::create_dir_all(&path).await.map_err(|e|e.to_string())?;
        let bundle=path.join("source.tar");
        let file=std::fs::File::create(&bundle).map_err(|e|e.to_string())?;
        let mut cmd=Command::new("git");cmd.arg("-C").arg(root).args(["archive","--format=tar","HEAD"]);
        if !scope.is_empty(){cmd.arg("--").arg(scope);}
        let mut proc=cmd.stdout(Stdio::from(file)).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
        let status=tokio::time::timeout(Duration::from_secs(90),proc.wait()).await.map_err(|_|"git archive timeout".to_string())?.map_err(|e|e.to_string())?;
        if !status.success(){return Err("git archive could not export selected scope".to_string())}
        let meta=tokio::fs::metadata(&bundle).await.map_err(|e|e.to_string())?;
        if meta.len()==0||meta.len()>40*1024*1024{return Err("source archive must be 1..40 MiB; narrow the file scope".into())}
        let bytes=tokio::fs::read(&bundle).await.map_err(|e|e.to_string())?;
        let response=w.client.post(format!("{}/api/v1/blobs",w.base)).bearer_auth(&w.token)
          .body(bytes).send().await.map_err(|e|e.to_string())?;
        if !response.status().is_success(){return Err("blob upload rejected".into())}
        let reply:Value=response.json().await.map_err(|e|e.to_string())?;
        let sha=reply["sha256"].as_str().ok_or("blob receipt malformed")?;
        Ok::<(String,String),String>((sha.to_string(),commit))
    }.await;
    let receipt=match result {Ok((sha,commit))=>json!({"node_id":w.id,"sha256":sha,"base_commit":commit,"error":null}),Err(e)=>json!({"node_id":w.id,"sha256":null,"base_commit":null,"error":e})};
    if let Err(e)=w.post(&format!("/api/v1/worker/source/{}/complete",t.id),receipt).await{tracing::error!(task=%t.id,error=%e,"source receipt failed")}
}
async fn apply_approved(w:Worker,t:TaskRow){
    let result=async {
        let alias=t.repo_alias.as_deref().ok_or("apply requires repository")?;
        let root=w.repos.get(alias).ok_or("file owner repository is no longer registered")?;
        let original=t.source_commit.as_deref().ok_or("original commit missing")?;
        let current=git(root,&["rev-parse","HEAD"]).await?;
        if current!=original{return Err("source HEAD has moved; manual reconciliation required".into())}
        let sha=t.apply_sha.as_deref().ok_or("approved patch missing")?;
        let patch=w.blob(sha).await?;
        if patch.len()>10*1024*1024||patch.is_empty(){return Err("patch exceeds review limits".into())}
        if String::from_utf8_lossy(&patch).contains("new file mode 120000") {return Err("patch creates a symlink; blocked".into())}
        let repo_run=w.root.join(&w.id).join(&t.id);
        tokio::fs::create_dir_all(&repo_run).await.map_err(|e|e.to_string())?;
        let approved=repo_run.join("reviewed-worktree");
        let patch_file=repo_run.join("approved.patch");
        tokio::fs::write(&patch_file,&patch).await.map_err(|e|e.to_string())?;
        let branch=format!("argo/review-{}",t.id.replace('-',''));
        let r=Command::new("git").arg("-C").arg(root).args(["worktree","add","-b",&branch])
          .arg(&approved).arg("HEAD").output().await.map_err(|e|e.to_string())?;
        if !r.status.success(){return Err(format!("could not create owner review branch: {}",String::from_utf8_lossy(&r.stderr)))}
        let check=Command::new("git").arg("-C").arg(&approved).args(["apply","--check","--binary"]).arg(&patch_file).output().await.map_err(|e|e.to_string())?;
        if !check.status.success(){return Err("patch cannot apply cleanly; isolated review worktree was preserved".into())}
        git(&approved,&["apply","--binary",patch_file.to_str().ok_or("patch path encoding")?]).await?;
        let ctx:Value=serde_json::from_str(&t.context_json).map_err(|e|e.to_string())?;
        let scope=ctx.get("argo_file_scope").and_then(Value::as_str).unwrap_or("");
        // Inspect all changed paths including untracked files, with NUL separators.
        let status=Command::new("git").arg("-C").arg(&approved).args(["status","--porcelain","-z"]).output().await.map_err(|e|e.to_string())?;
        if !status.status.success(){return Err("cannot audit reviewed worktree".into())}
        for entry in status.stdout.split(|b|*b==0).filter(|r|!r.is_empty()) {
            if entry.len()<4{return Err("unrecognized changed path".into())}
            let name=std::str::from_utf8(&entry[3..]).map_err(|_|"non UTF-8 changed path needs manual review")?;
            if !scope_allowed(name,scope){return Err(format!("change outside authorized scope: {name}; isolated worktree preserved"))}
        }
        Ok::<String,String>(format!("review_branch={branch};worktree={}",approved.display()))
    }.await;
    let report=match result {Ok(value)=>json!({"node_id":w.id,"success":true,"result":value}),Err(error)=>json!({"node_id":w.id,"success":false,"result":error})};
    if let Err(e)=w.post(&format!("/api/v1/worker/apply/{}/complete",t.id),report).await{tracing::error!(task=%t.id,error=%e,"apply receipt failed")}
}

#[cfg(test)]mod cross_node_tests {
  use super::*;
  #[test]fn source_scope_is_path_boundary(){assert!(scope_allowed("src/main.rs","src"));assert!(!scope_allowed("src2/main.rs","src"));assert!(!scope_allowed("../secrets",""));}
}
