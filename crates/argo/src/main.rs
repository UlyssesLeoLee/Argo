mod model;
mod rag;
mod ai;
mod server;
mod worker;
use std::env;
#[tokio::main]
async fn main()->Result<(),Box<dyn std::error::Error>>{
 tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_|"argo_runtime=info".into())).init();
 let args:Vec<String>=env::args().collect();
 match args.get(1).map(String::as_str){
  Some("server")=>{
    let admin=env::var("ARGO_ADMIN_TOKEN").map_err(|_|"ARGO_ADMIN_TOKEN missing")?;
    let worker=env::var("ARGO_WORKER_TOKEN").map_err(|_|"ARGO_WORKER_TOKEN missing")?;
    let bind=env::var("ARGO_BIND").unwrap_or_else(|_|"127.0.0.1:8788".into());
    let data=env::var("ARGO_DATA_DIR").unwrap_or_else(|_|"./data".into());
    let web=env::var("ARGO_WEB_DIR").unwrap_or_else(|_|"./web".into());
    server::serve(&bind,&data,&web,admin,worker).await?;
  },
  Some("worker")=>{
    let endpoint=env::var("ARGO_SERVER").unwrap_or_else(|_|"http://127.0.0.1:8788".into());
    let token=env::var("ARGO_WORKER_TOKEN").map_err(|_|"ARGO_WORKER_TOKEN missing")?;
    let node=env::var("ARGO_NODE_ID").unwrap_or_else(|_|"node-1".into());
    let parallel=env::var("ARGO_MAX_PARALLEL").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(2);
    worker::run(endpoint,node,token,parallel).await?;
  },
  _=>{eprintln!("Argo 0.3.0\n  cargo run -p argo-runtime -- server\n  cargo run -p argo-runtime -- worker\nSee README.md for environment variables.");}
 }
 Ok(())
}
