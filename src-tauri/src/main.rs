#[tauri::command]
async fn argo_request(path:String,method:String,body:Option<serde_json::Value>,token:String)->Result<serde_json::Value,String>{
    if !path.starts_with("/api/v1/") || path.contains("..") || path.contains("//") {return Err("invalid API path".into())}
    if !matches!(method.as_str(),"GET"|"POST"|"PATCH"){return Err("unsupported method".into())}
    let url=std::env::var("ARGO_CONTROLLER_URL").unwrap_or_else(|_|"http://127.0.0.1:8788".into());
    if !(url.starts_with("https://") || url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:")) {return Err("controller URL must be HTTPS or localhost".into())}
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(if path=="/api/v1/ai/preview" {100}else{35})).redirect(reqwest::redirect::Policy::none()).build().map_err(|e|e.to_string())?;
    let mut req=client.request(method.parse().map_err(|_|"bad method")?,format!("{}{}",url.trim_end_matches('/'),path)).bearer_auth(token);
    if let Some(v)=body{req=req.json(&v)}
    let res=req.send().await.map_err(|e|e.to_string())?;
    if !res.status().is_success(){return Err(format!("API {}: {}",res.status(),res.text().await.unwrap_or_default().chars().take(200).collect::<String>()))}
    res.json::<serde_json::Value>().await.map_err(|e|e.to_string())
}
fn main(){tauri::Builder::default().invoke_handler(tauri::generate_handler![argo_request]).run(tauri::generate_context!()).expect("cannot run Argo");}
