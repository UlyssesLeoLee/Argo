//! Argo Desktop: local Rust daemon lifecycle + narrow IPC to the UI.
//! The browser never receives the controller's administrator/worker tokens.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::{Arc,Mutex}, time::Duration};
use tauri::Manager;
use tauri_plugin_shell::{process::{CommandChild,CommandEvent},ShellExt};

const LOCAL_API: &str = "http://127.0.0.1:18788";
#[derive(Clone,Serialize,Deserialize)]
struct RagConnector {
    id:String,name:String,upsert_url:Option<String>,search_url:Option<String>,token:String,
}
#[derive(Clone,Serialize,Deserialize)]
struct DesktopConfig {
    admin_token:String,worker_token:String,node_id:String,
    #[serde(default)] repositories:BTreeMap<String,String>,
    #[serde(default)] connectors:Vec<RagConnector>,
}
struct DesktopState {
    config_path:PathBuf,data_dir:PathBuf,config:Mutex<DesktopConfig>,
    controller:Mutex<Option<CommandChild>>,worker:Mutex<Option<CommandChild>>,
    last_error:Mutex<Option<String>>,
}
fn secret()->String{format!("{}{}{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple())}
fn persist(path:&PathBuf,c:&DesktopConfig)->Result<(),String>{
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let tmp=path.with_extension("json.tmp");
    fs::write(&tmp,serde_json::to_vec_pretty(c).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    #[cfg(unix)]{
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?;
    }
    // On Windows, std::fs::rename cannot replace an existing destination.
    // Preserve a recoverable copy before updating credentials/configuration.
    #[cfg(windows)] {
        if path.exists() {
            let backup=path.with_extension("json.previous");
            fs::copy(path,&backup).map_err(|e|e.to_string())?;
            if let Err(e)=fs::copy(&tmp,path) {
                let _=fs::copy(&backup,path);
                return Err(e.to_string())
            }
            fs::remove_file(&tmp).map_err(|e|e.to_string())?;
            return Ok(())
        }
    }
    fs::rename(&tmp,path).map_err(|e|e.to_string())
}
fn init(path:&PathBuf)->Result<DesktopConfig,String>{
    if path.is_file(){let bytes=fs::read(path).map_err(|e|e.to_string())?;
        return serde_json::from_slice(&bytes).map_err(|e|format!("invalid desktop settings: {e}"));
    }
    let cfg=DesktopConfig {admin_token:secret(),worker_token:secret(),node_id:format!("desktop-{}",uuid::Uuid::new_v4().simple()),repositories:BTreeMap::new(),connectors:vec![]};
    persist(path,&cfg)?;Ok(cfg)
}
fn report_err(s:&DesktopState,e:String){if let Ok(mut v)=s.last_error.lock(){*v=Some(e);}}
fn drain(mut rx:tauri::async_runtime::Receiver<CommandEvent>){
    tauri::async_runtime::spawn(async move{while let Some(msg)=rx.recv().await{
        match msg {CommandEvent::Error(e)=>eprintln!("Argo runtime: {e}"),CommandEvent::Stderr(bytes)=>eprintln!("Argo runtime: {}",String::from_utf8_lossy(&bytes)),_=>{}}
    }});
}
fn start_controller(app:&tauri::AppHandle,state:&Arc<DesktopState>)->Result<(),String>{
    if state.controller.lock().map_err(|e|e.to_string())?.is_some(){return Ok(())}
    let cfg=state.config.lock().map_err(|e|e.to_string())?.clone();
    let cmd=app.shell().sidecar("argo-runtime").map_err(|e|e.to_string())?
      .arg("server").env("ARGO_ADMIN_TOKEN",cfg.admin_token).env("ARGO_WORKER_TOKEN",cfg.worker_token)
      .env("ARGO_BIND","127.0.0.1:18788")
      .env("ARGO_DATA_DIR",state.data_dir.to_string_lossy().to_string())
      .env("ARGO_WEB_DIR",state.data_dir.to_string_lossy().to_string())
      .env("ARGO_RAG_CONNECTORS_JSON",serde_json::to_string(&cfg.connectors.iter().enumerate().map(|(i,c)|json!({"id":c.id,"name":c.name,"upsert_url":c.upsert_url,"search_url":c.search_url,"token_env":format!("ARGO_RAG_TOKEN_{i}")})).collect::<Vec<_>>()).map_err(|e|e.to_string())?);
    let cmd=cfg.connectors.iter().enumerate().fold(cmd,|acc,(i,c)|acc.env(format!("ARGO_RAG_TOKEN_{i}"),&c.token));
    let (rx,child)=cmd.set_process_group(true).spawn().map_err(|e|e.to_string())?;
    drain(rx);*state.controller.lock().map_err(|e|e.to_string())?=Some(child);Ok(())
}
fn start_worker(app:&tauri::AppHandle,state:&Arc<DesktopState>)->Result<(),String>{
    if state.worker.lock().map_err(|e|e.to_string())?.is_some(){return Ok(())}
    let cfg=state.config.lock().map_err(|e|e.to_string())?.clone();
    let repo_map=cfg.repositories.iter().map(|(name,path)|format!("{name}={path}")).collect::<Vec<_>>().join(";");
    let cmd=app.shell().sidecar("argo-runtime").map_err(|e|e.to_string())?
      .arg("worker").env("ARGO_SERVER",LOCAL_API).env("ARGO_WORKER_TOKEN",cfg.worker_token)
      .env("ARGO_NODE_ID",cfg.node_id).env("ARGO_MAX_PARALLEL","2")
      .env("ARGO_REPOS",repo_map).env("ARGO_RUNS_DIR",state.data_dir.join("runs").to_string_lossy().to_string());
    let (rx,child)=cmd.set_process_group(true).spawn().map_err(|e|e.to_string())?;
    drain(rx);*state.worker.lock().map_err(|e|e.to_string())?=Some(child);Ok(())
}
fn stop(child:&Mutex<Option<CommandChild>>){if let Ok(mut v)=child.lock(){if let Some(c)=v.take(){let _=c.kill();}}}
async fn readiness(state:&Arc<DesktopState>)->bool{
    let token=match state.config.lock(){Ok(s)=>s.admin_token.clone(),Err(_)=>return false};
    let client=reqwest::Client::new();
    for _ in 0..40{
        if let Ok(resp)=client.get(format!("{LOCAL_API}/api/v1/providers")).bearer_auth(&token).timeout(Duration::from_millis(650)).send().await {
            if resp.status().is_success(){return true}
        }
        tokio::time::sleep(Duration::from_millis(180)).await;
    }
    false
}
async fn boot(app:tauri::AppHandle,state:Arc<DesktopState>){
    if !readiness(&state).await {
       if let Err(e)=start_controller(&app,&state){report_err(&state,format!("Controller could not start: {e}"));return}
       if !readiness(&state).await{report_err(&state,"Controller did not become ready; verify sidecar bundle and port 18788".into());return}
    }
    let active_worker={
       let cfg=match state.config.lock(){Ok(s)=>s.clone(),Err(_)=>return};
       let resp=reqwest::Client::new().get(format!("{LOCAL_API}/api/v1/nodes")).bearer_auth(cfg.admin_token).send().await;
       if let Ok(res)=resp {
         if let Ok(nodes)=res.json::<Value>().await {
           let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
           nodes.as_array().is_some_and(|list|list.iter().any(|n|n["id"]==cfg.node_id && n["last_seen"].as_i64().is_some_and(|t|now-t<30)))
         }else{false}
       }else{false}
    };
    if active_worker{return}
    if let Err(e)=start_worker(&app,&state){report_err(&state,format!("Worker could not start: {e}"))}
    else if let Ok(mut v)=state.last_error.lock(){*v=None;}
}
#[tauri::command]
async fn argo_request(app:tauri::AppHandle,path:String,method:String,body:Option<Value>)->Result<Value,String>{
    if !path.starts_with("/api/v1/")||path.contains("..")||path.contains("//")||path.contains('#')||path.contains('\\') {return Err("invalid API path".into())}
    if !matches!(method.as_str(),"GET"|"POST"|"PATCH"){return Err("unsupported method".into())}
    let token={let s=app.state::<Arc<DesktopState>>();let cfg=s.config.lock().map_err(|e|e.to_string())?;cfg.admin_token.clone()};
    let client=reqwest::Client::builder().timeout(Duration::from_secs(if path=="/api/v1/ai/preview"{100}else{45}))
       .redirect(reqwest::redirect::Policy::none()).build().map_err(|e|e.to_string())?;
    let mut request=client.request(method.parse().map_err(|_|"bad method")?,format!("{LOCAL_API}{path}")).bearer_auth(token);
    if let Some(value)=body{request=request.json(&value)}
    let response=request.send().await.map_err(|e|e.to_string())?;
    let status=response.status();let txt=response.text().await.map_err(|e|e.to_string())?;
    if !status.is_success(){return Err(format!("API {status}: {}",txt.chars().take(400).collect::<String>()))}
    serde_json::from_str(&txt).map_err(|e|e.to_string())
}
#[tauri::command]
async fn argo_upload_attachment(app:tauri::AppHandle,name:String,base64:String)->Result<Value,String>{
    use base64::Engine;
    if name.is_empty()||name.len()>120||name.contains('/')||name.contains('\\')||name=="."||name==".."{
        return Err("invalid attachment name".into())
    }
    if base64.len()>14_000_000{return Err("attachment too large".into())}
    let bytes=base64::engine::general_purpose::STANDARD.decode(base64).map_err(|_|"invalid file encoding")?;
    if bytes.is_empty()||bytes.len()>10*1024*1024{return Err("file must be 1..10 MiB".into())}
    let token={let s=app.state::<Arc<DesktopState>>();let c=s.config.lock().map_err(|e|e.to_string())?;c.admin_token.clone()};
    let response=reqwest::Client::new().post(format!("{LOCAL_API}/api/v1/blobs")).bearer_auth(token)
        .body(bytes).timeout(Duration::from_secs(45)).send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(format!("file upload returned {}",response.status()))}
    let receipt:Value=response.json().await.map_err(|e|e.to_string())?;
    Ok(json!({"name":name,"sha256":receipt["sha256"],"size":receipt["size"]}))
}
#[tauri::command]
async fn argo_daemon_status(app:tauri::AppHandle)->Result<Value,String>{
    let s=app.state::<Arc<DesktopState>>();
    let cfg=s.config.lock().map_err(|e|e.to_string())?.clone();
    Ok(json!({"controller_started":s.controller.lock().map_err(|e|e.to_string())?.is_some(),
      "worker_started":s.worker.lock().map_err(|e|e.to_string())?.is_some(),
      "node_id":cfg.node_id,"repositories":cfg.repositories,"connectors":cfg.connectors.iter().map(|c|json!({"id":c.id,"name":c.name,"has_upsert":c.upsert_url.is_some(),"has_search":c.search_url.is_some()})).collect::<Vec<_>>(),
      "last_error":s.last_error.lock().map_err(|e|e.to_string())?.clone()}))
}
fn repo_name_ok(name:&str)->bool{!name.is_empty()&&name.len()<=50&&name.chars().all(|x|x.is_ascii_alphanumeric()||x=='-'||x=='_')}
#[tauri::command]
async fn argo_register_repo(app:tauri::AppHandle,alias:String,path:String)->Result<Value,String>{
    if !repo_name_ok(&alias)||path.contains(';')||path.contains('='){return Err("invalid repository alias/path".into())}
    let real=fs::canonicalize(&path).map_err(|e|e.to_string())?;
    if !real.is_dir()||!real.join(".git").exists(){return Err("choose a local Git checkout root".into())}
    let s=app.state::<Arc<DesktopState>>().inner().clone();
    {let mut cfg=s.config.lock().map_err(|e|e.to_string())?;
      cfg.repositories.insert(alias.clone(),real.to_string_lossy().to_string());persist(&s.config_path,&cfg)?;}
    stop(&s.worker);start_worker(&app,&s)?;
    Ok(json!({"ok":true,"alias":alias,"path":real.to_string_lossy()}))
}
#[tauri::command]
async fn argo_save_connector(app:tauri::AppHandle,id:String,name:String,upsert_url:Option<String>,search_url:Option<String>,token:String)->Result<Value,String>{
    if !repo_name_ok(&id)||name.trim().is_empty()||name.chars().count()>80||token.len()<16{return Err("invalid connector id/name/token".into())}
    for url in [&upsert_url,&search_url]{if url.as_deref().is_some_and(|s|!s.starts_with("https://")||s.contains('@')){return Err("connector URLs must be HTTPS".into())}}
    if upsert_url.is_none()&&search_url.is_none(){return Err("configure at least one RAG endpoint".into())}
    let s=app.state::<Arc<DesktopState>>().inner().clone();
    {let mut cfg=s.config.lock().map_err(|e|e.to_string())?;
      cfg.connectors.retain(|c|c.id!=id);
      cfg.connectors.push(RagConnector{id:id.clone(),name,upsert_url,search_url,token});
      if cfg.connectors.len()>25{return Err("maximum 25 connectors".into())}
      persist(&s.config_path,&cfg)?;}
    stop(&s.worker);stop(&s.controller);
    boot(app,s).await;
    Ok(json!({"ok":true,"id":id}))
}
fn main(){
    let app=tauri::Builder::default()
       .plugin(tauri_plugin_shell::init())
       .setup(|app| {
          let data_dir=app.path().app_data_dir()?;
          fs::create_dir_all(&data_dir)?;
          let settings=data_dir.join("desktop.json");
          let config=init(&settings).map_err(std::io::Error::other)?;
          let state=Arc::new(DesktopState{config_path:settings,data_dir,config:Mutex::new(config),controller:Mutex::new(None),worker:Mutex::new(None),last_error:Mutex::new(None)});
          app.manage(state.clone());
          let handle=app.handle().clone();
          tauri::async_runtime::spawn(async move{boot(handle,state).await;});
          Ok(())
       })
       .invoke_handler(tauri::generate_handler![argo_request,argo_upload_attachment,argo_daemon_status,argo_register_repo,argo_save_connector])
       .build(tauri::generate_context!()).expect("cannot build Argo Desktop");
    app.run(|app,event|{if let tauri::RunEvent::Exit=event {
      let state=app.state::<Arc<DesktopState>>();stop(&state.worker);stop(&state.controller);
    }});
}
