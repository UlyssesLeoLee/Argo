/** Shared browser-side Argo UI contract. No network calls and no side effects. */
export const PROVIDERS = [
  {id:'claude',name:'Claude Code',symbol:'✳',subtitle:'Anthropic CLI',hint:'claude -p · 非交互流式执行'},
  {id:'codex',name:'Codex',symbol:'⌘',subtitle:'OpenAI CLI',hint:'codex exec · JSONL 事件输出'},
  {id:'hermes',name:'Hermes',symbol:'H',subtitle:'NousResearch CLI',hint:'hermes chat --query-file --oneshot'},
  {id:'orca',name:'Orca',symbol:'◫',subtitle:'Worktree CLI',hint:'orca worktree create · 创建并启动 Agent'},
  {id:'minimax',name:'MiniMax',symbol:'M',subtitle:'MiniMax Code CLI',hint:'mcode exec · 独立 Agent Runtime'},
  {id:'multica',name:'Multica',symbol:'◇',subtitle:'Workspace / Issue',hint:'multica issue · 项目与任务卡绑定'},
  {id:'star',name:'STAR',symbol:'?',subtitle:'自有项目平台（待公开）',hint:'通过配置的 STAR HTTPS 接口派发任务，异步回执'}
];
export const terminalStates = ['completed','failed','dispatched','cancelled','needs_review'];
export const stateLabels={queued:'排队中',running:'执行中',cancelling:'正在取消',completed:'已完成',failed:'失败',dispatched:'已派发',cancelled:'已取消',needs_review:'需要检查'};
export function escapeHtml(s){return String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
export function validateTask(p){
  if(!PROVIDERS.some(x=>x.id===p.provider))return '请选择有效的 Agent 平台';
  if(!p.title?.trim() || p.title.length>200)return '任务标题不能为空且不能超过 200 字符';
  if(!p.prompt?.trim() || p.prompt.length>120000)return '请输入任务目标（不超过 120000 字符）';
  if(p.provider==='orca' && (!p.context?.orca_repo_id || !p.context?.orca_agent))return 'Orca 必须指定仓库与 Agent';
  if(p.provider==='multica'){
    if(!p.context?.multica_workspace_id)return 'Multica 必须选择 Workspace';
    if(!p.context?.multica_assignee)return 'Multica 必须填写需要执行的 Agent';
    if(p.context?.multica_mode==='existing' && !p.context?.multica_issue_id)return '使用已有任务卡时必须填写 Issue ID';
    if(!p.context?.multica_issue_id && !p.context?.multica_project_id)return 'Multica 创建任务卡时必须选择 Project';
  }
  if(p.provider==='star' && (!p.context?.star_project_id || (p.context?.star_mode==='existing' && !p.context?.star_ticket_id)))return 'STAR 必须填写项目，引用已有任务时还需要任务卡 ID';
  if((p.attachments?.length??0)>20)return '最多允许 20 个附件';
  return null;
}
export function makeTask({provider,title,prompt,model,repo,node,context,attachments}){
  const p={provider,title:title.trim(),prompt:prompt.trim(),model:model?.trim()||null,
    target_node:node||null,repo_alias:repo||null,context:context||{},attachments:attachments||[]};
  return p;
}
export function normalizeCatalog(output,kind){
  let parsed;try{parsed=JSON.parse(output);}catch{return []}
  const keys={project:['projects','items','data','results'],issue:['issues','items','data','results'],agent:['agents','items','data','results'],repo:['repositories','repos','items','data','results']}[kind]||[];
  let values=Array.isArray(parsed)?parsed:null;
  if(!values)for(const key of keys){if(Array.isArray(parsed?.[key])){values=parsed[key];break}}
  if(!Array.isArray(values))return [];
  return values.map(item=>({
    id:String(item?.id??item?.uuid??item?.repositoryId??item?.identifier??''),
    label:String(item?.name??item?.title??item?.identifier??item?.slug??item?.id??'unknown')
  })).filter(x=>x.id);
}
export function ago(t){const delta=Math.max(0,Math.floor(Date.now()/1000)-(t||0));return delta<60?'刚刚':delta<3600?`${Math.floor(delta/60)} 分钟前`:delta<86400?`${Math.floor(delta/3600)} 小时前`:`${Math.floor(delta/86400)} 天前`;}
export function icon(id){return PROVIDERS.find(x=>x.id===id)?.symbol||'✦'}
