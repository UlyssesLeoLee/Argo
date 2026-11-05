import {PROVIDERS,stateLabels,terminalStates,escapeHtml,validateTask,makeTask,normalizeCatalog,ago,icon} from './lib.mjs';
const $=id=>document.getElementById(id);
const bySel=s=>document.querySelector(s);
const demo=new URLSearchParams(location.search).get('demo')==='1';
const store={provider:'claude',nodes:[],tasks:[],selectedTask:null,tab:'compose',files:[],token:sessionStorage.getItem('argo_admin_token')||'',catalog:{project:[],issue:[],agent:[],repo:[]},lastSeq:0,events:[],multicaMode:'new'};
const initial=Date.now()/1000|0;
const demoNodes=[
  {id:'studio-win',hostname:'STUDIO-WIN11',os:'windows',providers_json:JSON.stringify(['claude','codex','hermes','orca','minimax','multica']),repos_json:JSON.stringify(['star','ide']),last_seen:initial,max_parallel:3},
  {id:'gpu-linux',hostname:'GPU-SERVER',os:'linux',providers_json:JSON.stringify(['claude','codex','hermes','minimax']),repos_json:JSON.stringify(['star']),last_seen:initial,max_parallel:4}
];
const demoTasks=[{id:'a1-demo',title:'检查 Star 的任务管理接口',prompt:'审查接口...',provider:'codex',model:null,repo_alias:'star',target_node:null,assigned_node:'gpu-linux',state:'completed',created_at:initial-400,started_at:initial-393,finished_at:initial-320,output:'已检查目标接口。\nDEMO: 示例内容（非实际 Agent 输出）',error:null},
{id:'b2-demo',title:'设计 Daemon 节点监控',prompt:'设计...',provider:'claude',model:null,repo_alias:'star',target_node:null,assigned_node:'studio-win',state:'running',created_at:initial-70,started_at:initial-60,finished_at:null,output:null,error:null}];
let fakeEvents=[{seq:1,stream:'system',content:'DEMO · 任务已分配给 studio-win',created_at:initial-70},{seq:2,stream:'stdout',content:'正在分析需求与代码结构（模拟输出）',created_at:initial-60}];
let timerToast;
function toast(message){$('toast').textContent=message;$('toast').classList.add('show');clearTimeout(timerToast);timerToast=setTimeout(()=>$('toast').classList.remove('show'),3500)}
function showError(s){$('formError').hidden=!s;$('formError').textContent=s||'';if(s)toast(s)}
function endpoint(){return location.origin}
async function api(path,{method='GET',body,blob=false}={}){
  if(demo)return demoAPI(path,{method,body,blob});
  const headers={'Authorization':`Bearer ${store.token}`};
  if(body!==undefined&&!blob)headers['Content-Type']='application/json';
  const result=await fetch(endpoint()+path,{method,headers,body:blob?body:body===undefined?undefined:JSON.stringify(body)});
  if(!result.ok){let message=await result.text().catch(()=>'');throw new Error(`HTTP ${result.status}: ${message.slice(0,260)}`)}
  return result.json();
}
async function demoAPI(path,{method,body,blob}={}){
  if(path==='/api/v1/providers')return PROVIDERS;
  if(path==='/api/v1/nodes')return demoNodes;
  if(path.startsWith('/api/v1/tasks?'))return [...demoTasks].sort((a,b)=>b.created_at-a.created_at);
  if(path==='/api/v1/tasks'&&method==='POST'){
    const id='demo-'+crypto.randomUUID().slice(0,8),t={...body,id,created_at:Date.now()/1000|0,started_at:null,finished_at:null,assigned_node:null,state:'queued',output:null,error:null};
    demoTasks.unshift(t);
    setTimeout(()=>{if(t.state==='queued'){t.state='running';t.assigned_node=t.target_node||'studio-win';t.started_at=Date.now()/1000|0;renderAll()}},700);
    setTimeout(()=>{if(t.state==='running'){
      t.state=body.context?.argo_action?'completed':(['orca','multica'].includes(body.provider)?'dispatched':'completed');
      t.finished_at=Date.now()/1000|0;
      const sample={discover_repos:JSON.stringify([{id:'repo-demo',name:'Star'}]),discover_projects:JSON.stringify([{id:'project-demo',title:'Star Sprint'}]),discover_agents:JSON.stringify([{id:'agent-demo',name:'Builder'}]),discover_issues:JSON.stringify([{id:'issue-demo',identifier:'MUL-123',title:'示例任务卡'}])};
      t.output=sample[body.context?.argo_action]||'DEMO: 已收到任务。不会执行真实 CLI，也不会产生模型调用费用。';renderAll();}},1500);
    return{id,state:'queued'};
  }
  if(path.startsWith('/api/v1/tasks/')&&path.endsWith('/cancel')&&method==='POST'){
    const id=path.split('/')[4],t=demoTasks.find(t=>t.id===id);if(t)t.state='cancelled';renderAll();return{ok:true};
  }
  if(path.startsWith('/api/v1/tasks/')&&path.includes('/events'))return store.selectedTask==='b2-demo'?fakeEvents:[];
  if(path.startsWith('/api/v1/tasks/'))return demoTasks.find(t=>path.includes(t.id))||null;
  if(path==='/api/v1/blobs'&&blob)return{sha256:'a'.repeat(64),size:body.byteLength};
  throw Error('Unknown demo route: '+path);
}
function renderProviderOptions(){
 $('providers').innerHTML=PROVIDERS.map(x=>`<button type="button" role="radio" aria-checked="${store.provider===x.id}" class="provider-option ${store.provider===x.id?'active':''}" data-provider="${x.id}"><span class="provider-logo">${x.symbol}</span><span><span class="name">${x.name}</span><span class="desc">${x.subtitle}</span></span></button>`).join('');
 for(const el of document.querySelectorAll('[data-provider]'))el.addEventListener('click',()=>{
   store.provider=el.dataset.provider;renderProviderOptions();renderContext();$('providerHint').textContent=PROVIDERS.find(x=>x.id===store.provider).hint;
   $('modelSelect').disabled=['multica','orca'].includes(store.provider);
   $('modelSelect').placeholder=store.provider==='orca'?'由 Orca 内部 Agent 决定模型':store.provider==='multica'?'由 Multica Agent 配置决定模型':'使用平台默认模型';
 });
}
function options(kind){return `<datalist id="list-${kind}">${store.catalog[kind].map(x=>`<option value="${escapeHtml(x.id)}">${escapeHtml(x.label)}</option>`).join('')}</datalist>`}
function contextInput(label,id,placeholder,kind='',value=''){
 return `<div><label class="field-label" for="${id}">${label}</label><input id="${id}" type="text" value="${escapeHtml(value)}" placeholder="${escapeHtml(placeholder)}" ${kind?`list="list-${kind}"`:''}/>${kind?options(kind):''}</div>`;
}
function renderContext(){
 const root=$('contextFields');
 const external=['orca','multica'].includes(store.provider);
 $('repoAlias').disabled=external;
 if(external)$('repoAlias').value='';
 $('modelSelect').disabled=external;
 if(external)$('modelSelect').value='';
 if(!['orca','multica'].includes(store.provider)){root.classList.remove('visible');root.innerHTML='';return}
 root.classList.add('visible');
 if(store.provider==='orca'){
   root.innerHTML=`<div class="context-title"><span>◫ Orca · Worktree 运行上下文</span><span class="tiny-badge">CLI ADAPTER</span></div>
   <div class="form-grid">${contextInput('Orca Repository ID','orcaRepo','例如：仓库 UUID','repo')}<div><label class="field-label" for="orcaAgent">启动 Agent</label><select id="orcaAgent"><option value="codex">Codex</option><option value="claude">Claude</option><option value="opencode">OpenCode</option><option value="pi">Pi</option></select></div></div>
   <div class="context-toolbar"><button type="button" class="context-button" id="queryRepos">↻ 从目标节点查询仓库</button></div>
   <p class="context-help">Orca 通过 <code>orca worktree create --repo id:… --agent … --prompt … --json</code> 创建工作空间并启动 Agent；命令返回表示成功派发，不代表 Agent 已执行完毕。</p>`;
   $('queryRepos').onclick=()=>lookup('repo','discover_repos');
 }else{
   root.innerHTML=`<div class="context-title"><span>◇ Multica · Workspace / Project / Issue</span><span class="tiny-badge">ISSUE ADAPTER</span></div>
   ${contextInput('Workspace ID','multiWorkspace','Workspace UUID 或当前工作区标识')}
   <div class="context-toolbar"><button type="button" id="queryProjects" class="context-button">↻ 查询 Project</button><button type="button" id="queryAgents" class="context-button">↻ 查询 Agent</button><button type="button" id="queryIssues" class="context-button">↻ 查询任务卡</button></div>
   <div style="margin:6px 0 16px"><label class="field-label" for="multiMode">任务卡处理方式</label><select id="multiMode"><option value="new" ${store.multicaMode==='new'?'selected':''}>新建任务卡并分配给 Agent</option><option value="existing" ${store.multicaMode==='existing'?'selected':''}>使用已有任务卡，追加消息并重新指派</option></select></div>
   <div class="form-grid">${contextInput('Project ID','multiProject','选择已有项目 ID','project')}${contextInput('目标 Agent','multiAssignee','Agent 名称（例如 Builder）','agent')}</div>
   <div id="multiExistingWrap" style="margin-top:13px;${store.multicaMode==='new'?'display:none':''}">${contextInput('Issue / Task Card ID','multiIssue','例如：MUL-123','issue')}</div>
   <p class="context-help">Project 属于 Multica Workspace；新建卡使用 <code>issue create</code>。已有卡先通过 <code>issue comment add</code> 追加内容，再通过 <code>issue assign</code> 触发分派；真实执行由 Multica 自身的 Daemon 完成。</p>`;
   $('multiMode').onchange=e=>{store.multicaMode=e.target.value;$('multiExistingWrap').style.display=e.target.value==='existing'?'block':'none'};
   $('queryProjects').onclick=()=>lookup('project','discover_projects');$('queryAgents').onclick=()=>lookup('agent','discover_agents');$('queryIssues').onclick=()=>lookup('issue','discover_issues');
 }
}
function getContext(){
 if(store.provider==='orca')return {orca_repo_id:$('orcaRepo')?.value.trim()||'',orca_agent:$('orcaAgent')?.value||'codex'};
 if(store.provider==='multica')return {multica_mode:store.multicaMode,multica_workspace_id:$('multiWorkspace')?.value.trim()||'',multica_project_id:$('multiProject')?.value.trim()||'',multica_assignee:$('multiAssignee')?.value.trim()||'',...(store.multicaMode==='existing'?{multica_issue_id:$('multiIssue')?.value.trim()||''}:{})};
 return{};
}
function collect(){return makeTask({provider:store.provider,title:$('taskTitle').value,prompt:$('taskPrompt').value,model:$('modelSelect').disabled?'':$('modelSelect').value,repo:$('repoAlias').disabled?'':$('repoAlias').value,node:$('nodeSelect').value,context:getContext(),attachments:[]})}
async function lookup(kind,action){
  const context=getContext();
  if(!demo&&!store.token){toast('请先在设置页连接 Argo 控制面');return}
  if(kind!=='repo'&&!context.multica_workspace_id){toast('请先填写 Workspace ID');return}
  if(kind==='issue'&&!context.multica_project_id){toast('请先填写 Project ID');return}
  const button=kind==='repo'?$('queryRepos'):kind==='project'?$('queryProjects'):kind==='issue'?$('queryIssues'):$('queryAgents');
  if(button)button.disabled=true;
  try{
   const payload=makeTask({provider:store.provider,title:`Catalog · ${kind}`,prompt:`Read-only catalog discovery: ${kind}`,node:$('nodeSelect').value,context:{...context,argo_action:action}});
   const job=await api('/api/v1/tasks',{method:'POST',body:payload});toast(`正在目标节点查询 ${kind}…`);
   let output=null;for(let i=0;i<28;i++){
      await new Promise(r=>setTimeout(r,650));const t=await api('/api/v1/tasks/'+encodeURIComponent(job.id));
      if(t?.state==='completed'){output=t.output;break}
      if(['failed','cancelled','needs_review'].includes(t?.state)){throw Error(`查询失败：${t.error||t.state}`)}
   }
   if(output===null){toast('查询已排队，可在“执行记录”查看结果');return}
   const normalized=normalizeCatalog(output,kind);store.catalog[kind]=normalized;
   const original={workspace:$('multiWorkspace')?.value,project:$('multiProject')?.value,assignee:$('multiAssignee')?.value,issue:$('multiIssue')?.value,repo:$('orcaRepo')?.value,agent:$('orcaAgent')?.value};
   renderContext();
   for(const [key,id] of Object.entries({workspace:'multiWorkspace',project:'multiProject',assignee:'multiAssignee',issue:'multiIssue',repo:'orcaRepo',agent:'orcaAgent'}))if($(id)&&original[key])$(id).value=original[key];
   toast(normalized.length?`已获取 ${normalized.length} 条 ${kind} 记录`:'未解析到记录，请到执行记录检查原始 JSON');
  }catch(e){toast(e.message)}finally{if(button)button.disabled=false}
}
function switchTab(name){store.tab=name;for(const el of document.querySelectorAll('.tab-screen'))el.classList.toggle('active',el.id===`tab-${name}`);for(const el of document.querySelectorAll('.rail-button'))el.classList.toggle('selected',el.dataset.tab===name);if(name==='runs')renderRuns();if(name==='nodes')renderNodes();if(name==='settings')renderSettings();}
function online(n){return Date.now()/1000 - n.last_seen < 45}
function currentNodes(){return store.nodes.filter(n=>online(n))}
function renderComposeSummary(){
 const active=currentNodes();$('nodeCount').textContent=active.length;$('runningCount').textContent=store.tasks.filter(t=>t.state==='running').length;$('queueCount').textContent=store.tasks.filter(t=>t.state==='queued').length;
 $('miniNodes').innerHTML=active.length?active.slice(0,4).map(n=>`<div class="mini-node"><span class="node-chip">⬡</span><div><strong>${escapeHtml(n.hostname)}</strong><span>${escapeHtml(n.os)} · ${JSON.parse(n.providers_json||'[]').length} 个运行时</span></div><span class="status-dot"></span></div>`).join(''):'<div class="empty-muted">当前没有在线节点。请启动 Rust Worker Daemon。</div>';
 $('recentRuns').innerHTML=store.tasks.length?store.tasks.slice(0,5).map(t=>`<div class="recent-row" data-open-run="${escapeHtml(t.id)}"><span class="mini-provider">${icon(t.provider)}</span><div class="run-copy"><strong>${escapeHtml(t.title)}</strong><span>${escapeHtml(stateLabels[t.state]||t.state)} · ${ago(t.created_at)}</span></div><span class="state-tag ${escapeHtml(t.state)}">${escapeHtml(stateLabels[t.state]||t.state)}</span></div>`).join(''):'<div class="empty-muted">还没有任务，提交第一条指令开始。</div>';
 for(const el of document.querySelectorAll('[data-open-run]'))el.onclick=()=>openRun(el.dataset.openRun);
 const previous=$('nodeSelect').value;$('nodeSelect').innerHTML='<option value="">自动调度 · Auto</option>'+active.map(n=>`<option value="${escapeHtml(n.id)}">${escapeHtml(n.hostname)} · ${escapeHtml(n.id)}</option>`).join('');$('nodeSelect').value=previous;
 const oldRepo=$('repoAlias').value;const aliases=new Set(active.flatMap(n=>{try{return JSON.parse(n.repos_json||'[]')}catch{return[]}}));$('repoAlias').innerHTML='<option value="">独立工作空间 / 无仓库</option>'+[...aliases].sort().map(name=>`<option value="${escapeHtml(name)}">${escapeHtml(name)} · Git Worktree</option>`).join('');$('repoAlias').value=oldRepo;
}
function renderRuns(){
 const filter=$('runFilter').value;const filtered=store.tasks.filter(t=>filter==='all'||t.state===filter);
 $('runsList').innerHTML=filtered.length?filtered.map(t=>`<button type="button" class="run-item ${store.selectedTask===t.id?'selected':''}" data-run="${escapeHtml(t.id)}"><span class="run-top"><strong>${escapeHtml(t.title)}</strong><span class="state-tag ${escapeHtml(t.state)}">${escapeHtml(stateLabels[t.state]||t.state)}</span></span><small>${icon(t.provider)} ${escapeHtml(t.provider)} · ${ago(t.created_at)} · ${escapeHtml(t.assigned_node||'调度中')}</small></button>`).join(''):'<div class="empty-muted">没有符合条件的任务。</div>';
 for(const el of document.querySelectorAll('[data-run]'))el.onclick=()=>openRun(el.dataset.run);
 renderRunDetail();
}
function parseArtifacts(t){try{return JSON.parse(t?.artifacts_json||'[]')}catch{return []}}
async function downloadArtifact(sha,name){
 if(demo){toast('演示模式没有真实文件');return}
 try{const res=await fetch(endpoint()+'/api/v1/blobs/'+encodeURIComponent(sha),{headers:{Authorization:`Bearer ${store.token}`}});if(!res.ok)throw Error(`HTTP ${res.status}`);
 const data=await res.blob(),uri=URL.createObjectURL(data),a=document.createElement('a');a.href=uri;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(uri),1500);
 }catch(e){toast(`文件下载失败：${e.message}`)}
}
function renderRunDetail(){const t=store.tasks.find(t=>t.id===store.selectedTask);if(!t){$('runDetail').innerHTML='<div class="empty-illustration">↗</div><p class="empty-center">选择左侧任务，查看输出与事件日志</p>';$('cancelTask').hidden=true;return}
 $('cancelTask').hidden=!['queued','running'].includes(t.state);
 $('runDetail').innerHTML=`<div class="detail-heading"><h3>${escapeHtml(t.title)}</h3><span class="state-tag ${escapeHtml(t.state)}">${escapeHtml(stateLabels[t.state]||t.state)}</span></div>
 <div class="run-meta"><div><span>平台</span><strong>${escapeHtml(t.provider)}</strong></div><div><span>执行节点</span><strong>${escapeHtml(t.assigned_node||'等待分配')}</strong></div><div><span>工作仓库</span><strong>${escapeHtml(t.repo_alias||'独立工作空间')}</strong></div><div><span>任务 ID</span><strong>${escapeHtml(t.id)}</strong></div></div>
 <div class="field-label">结果文件 / Artifacts</div><div class="attachment-list">${parseArtifacts(t).length?parseArtifacts(t).map(a=>`<button class="file-tag" type="button" data-artifact="${escapeHtml(a.sha256)}" data-filename="${escapeHtml(a.name)}">⬇ ${escapeHtml(a.name)} · ${Math.ceil(a.size/1024)} KB</button>`).join(''):'<span class="tiny-note">尚无回传文件；Git 变更会作为 changes.patch 上传。</span>'}</div>
 <div style="height:20px"></div><div class="field-label">实时事件 / Stdout / Stderr</div><div id="eventConsole" class="log-console" aria-live="polite"></div>`;
 for(const el of document.querySelectorAll('[data-artifact]'))el.onclick=()=>void downloadArtifact(el.dataset.artifact,el.dataset.filename);
 renderEvents(t);
}
function renderEvents(t){const out=$('eventConsole');if(!out)return;out.innerHTML='';let logs=store.events.map(e=>({stream:e.stream,content:e.content}));if(!logs.length&&t.output)logs=[{stream:'stdout',content:t.output}];if(t.error)logs.push({stream:'stderr',content:t.error});if(!logs.length)logs=[{stream:'system',content:'等待节点输出…'}];for(const l of logs){const line=document.createElement('div');line.className='log-line '+l.stream;line.textContent=`[${l.stream}] ${l.content}`;out.append(line)}out.scrollTop=out.scrollHeight;}
function openRun(id){store.selectedTask=id;store.lastSeq=0;store.events=[];switchTab('runs');void loadEvents()}
function renderNodes(){const active=currentNodes();$('allNodeCount').textContent=store.nodes.length;$('allOnlineCount').textContent=active.length;$('allAdapterCount').textContent=active.reduce((a,n)=>a+JSON.parse(n.providers_json||'[]').length,0);
 $('nodesGrid').innerHTML=store.nodes.length?store.nodes.map(n=>`<div class="node-card"><div class="node-head"><div><h3>⬡ &nbsp;${escapeHtml(n.hostname)}</h3><p>${escapeHtml(n.id)} · ${escapeHtml(n.os)}</p></div><span class="state-tag ${online(n)?'completed':'failed'}">${online(n)?'在线':'离线'}</span></div><div class="thin-divider"></div><div class="node-details">${JSON.parse(n.providers_json||'[]').map(p=>`<span>${icon(p)} ${escapeHtml(p)}</span>`).join('')||'<span>无可用 CLI</span>'}</div><div class="node-footer"><span>并发槽 ${n.max_parallel}</span><span>最近心跳 ${ago(n.last_seen)}</span></div><div class="node-footer">仓库：${escapeHtml(JSON.parse(n.repos_json||'[]').join('、')||'未注册')}</div></div>`).join(''):'<div class="surface"><div class="empty-muted">暂无节点。请启动 Argo Worker 以自动注册。</div></div>';
}
function renderSettings(){
 $('apiEndpoint').textContent=endpoint();$('demoIndicator').textContent=demo?'开启（不执行真实任务）':'关闭';$('accessToken').value=store.token;
 const chip=bySel('.env-chip');chip.classList.toggle('connected',demo||!!store.token);$('connectionStatus').textContent=demo?'DEMO MODE':store.token?'控制面已配置':'未连接';
}
function renderAll(){renderComposeSummary();renderRuns();renderNodes();renderSettings()}
async function refresh(){
 try{
  if(!demo&&!store.token){renderSettings();return}
  const [nodes,tasks]=await Promise.all([api('/api/v1/nodes'),api('/api/v1/tasks?limit=100')]);store.nodes=nodes;store.tasks=tasks;
  renderAll();if(store.selectedTask)void loadEvents();
 }catch(e){$('connectionStatus').textContent='连接失败';toast(`Argo API：${e.message}`)}
}
async function loadEvents(){if(!store.selectedTask)return;try{const events=await api(`/api/v1/tasks/${encodeURIComponent(store.selectedTask)}/events?since=${store.lastSeq}`);if(events?.length){store.events.push(...events);store.lastSeq=events.at(-1).seq;const t=store.tasks.find(t=>t.id===store.selectedTask);if(t)renderEvents(t)}}catch(e){console.warn('events',e)}}
async function submit(){showError(null);let payload=collect();const issue=validateTask(payload);if(issue){showError(issue);return}
 if(!demo&&!store.token){showError('请先在「连接设置」中输入 Argo Admin Token');return}
 if(!demo&&!currentNodes().length){showError('没有在线节点，请先启动至少一台 Argo Worker');return}
 const candidates=currentNodes().filter(n=>{const arr=JSON.parse(n.providers_json||'[]');const repos=JSON.parse(n.repos_json||'[]');return arr.includes(payload.provider)&&(!payload.target_node||n.id===payload.target_node)&&(!payload.repo_alias||repos.includes(payload.repo_alias))});
 if(!candidates.length){showError('没有满足平台、指定机器及仓库条件的在线节点');return}
 const btn=$('submitTask');btn.disabled=true;btn.querySelector('span').textContent='正在提交…';
 try{
  for(const file of store.files){
    if(file.size>10*1024*1024)throw Error(`${file.name} 超过 10MB`);
    const bytes=await file.arrayBuffer();const blob=await api('/api/v1/blobs',{method:'POST',body:bytes,blob:true});
    payload.attachments.push({name:file.name,sha256:blob.sha256});
  }
  const result=await api('/api/v1/tasks',{method:'POST',body:payload});
  $('taskTitle').value='';$('taskPrompt').value='';$('charCount').textContent='0 字';store.files=[];renderFiles();await refresh();toast('任务提交成功，已进入调度队列');openRun(result.id);
 }catch(e){showError(e.message)}finally{btn.disabled=false;btn.querySelector('span').textContent='发送到集群'}
}
function renderFiles(){$('attachmentList').innerHTML=store.files.map((f,i)=>`<span class="file-tag">▣ ${escapeHtml(f.name)} <button type="button" data-remove-file="${i}" aria-label="移除 ${escapeHtml(f.name)}">×</button></span>`).join('');for(const el of document.querySelectorAll('[data-remove-file]'))el.onclick=()=>{store.files.splice(Number(el.dataset.removeFile),1);renderFiles()}}
function addFiles(files){const next=[...store.files];for(const f of files){if(next.length>=20){toast('最多允许 20 个文件');break}if(f.size>10*1024*1024){toast(`文件超出 10 MB：${f.name}`);continue}if(next.some(x=>x.name===f.name)){toast(`附件重名：${f.name}`);continue}if(/[/\\]/.test(f.name)){toast('非法文件名');continue}next.push(f)}store.files=next;renderFiles()}
function bootstrap(){
 renderProviderOptions();renderContext();renderSettings();
 for(const el of document.querySelectorAll('[data-tab]'))el.onclick=()=>switchTab(el.dataset.tab);
 for(const el of document.querySelectorAll('[data-switch]'))el.onclick=()=>switchTab(el.dataset.switch);
 $('themeButton').onclick=()=>{document.body.classList.toggle('light');sessionStorage.setItem('argo_theme',document.body.classList.contains('light')?'light':'dark')};
 if(sessionStorage.getItem('argo_theme')==='light')document.body.classList.add('light');
 $('taskPrompt').oninput=()=>{$('charCount').textContent=$('taskPrompt').value.length+' 字'};
 $('taskPrompt').onkeydown=e=>{if(e.key==='Enter'&&(e.ctrlKey||e.metaKey)){e.preventDefault();void submit()}};
 $('submitTask').onclick=()=>void submit();$('refreshAll').onclick=()=>void refresh();$('refreshRuns').onclick=()=>void refresh();$('refreshNodes').onclick=()=>void refresh();
 $('runFilter').onchange=renderRuns;
 $('openDocs').onclick=()=>window.open('/help.html','_blank','noopener');
 $('cancelTask').onclick=async()=>{if(!store.selectedTask||!confirm('取消这项任务？运行中的进程将在下次节点检查时终止。'))return;try{await api(`/api/v1/tasks/${store.selectedTask}/cancel`,{method:'POST',body:{}});await refresh();toast('取消请求已提交')}catch(e){toast(e.message)}};
 $('connectButton').onclick=async()=>{const token=$('accessToken').value.trim();if(token.length<24&&!demo){toast('令牌至少需要 24 个字符');return}const prev=store.token;store.token=token;try{await api('/api/v1/providers');sessionStorage.setItem('argo_admin_token',token);await refresh();toast('已连接 Argo 控制面')}catch(e){store.token=prev;toast(e.message)}};
 $('disconnectButton').onclick=()=>{store.token='';sessionStorage.removeItem('argo_admin_token');store.nodes=[];store.tasks=[];store.selectedTask=null;renderAll();toast('本次会话令牌已清除')};
 $('dropArea').onclick=()=>$('attachmentPicker').click();$('dropArea').onkeydown=e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();$('attachmentPicker').click()}};
 $('attachmentPicker').onchange=e=>{addFiles([...e.target.files]);e.target.value=''};
 $('dropArea').ondragover=e=>{e.preventDefault();$('dropArea').classList.add('over')};$('dropArea').ondragleave=()=>$('dropArea').classList.remove('over');$('dropArea').ondrop=e=>{e.preventDefault();$('dropArea').classList.remove('over');addFiles([...e.dataTransfer.files])};
 if(demo){store.nodes=demoNodes;store.tasks=demoTasks;renderAll();toast('DEMO MODE：所有任务与输出均为模拟数据')}else if(store.token){void refresh()}
 setInterval(()=>{if(demo||store.token)void refresh()},3500);
}
bootstrap();
