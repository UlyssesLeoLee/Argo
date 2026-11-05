/** Argo platform-facing task contracts, validated before Rust dispatch. */
import type {AgentId,Attachment,InputContext,TaskCreate,TaskState} from './contracts.js';

export interface ProviderMeta {id:AgentId;name:string;symbol:string;subtitle:string;hint:string}
export const PROVIDERS:ReadonlyArray<ProviderMeta> = [
  {id:'claude',name:'Claude Code',symbol:'✳',subtitle:'Anthropic CLI',hint:'claude -p · 非交互流式执行'},
  {id:'codex',name:'Codex',symbol:'⌘',subtitle:'OpenAI CLI',hint:'codex exec · JSONL 事件输出'},
  {id:'hermes',name:'Hermes',symbol:'H',subtitle:'NousResearch CLI',hint:'hermes chat --query-file --oneshot'},
  {id:'orca',name:'Orca',symbol:'◫',subtitle:'Worktree CLI',hint:'orca worktree create · 创建并启动 Agent'},
  {id:'minimax',name:'MiniMax',symbol:'M',subtitle:'MiniMax 模型',hint:'通过执行节点的 Hermes MiniMax 模型配置运行'},
  {id:'multica',name:'Multica',symbol:'◇',subtitle:'Workspace / Issue',hint:'multica issue · 项目与任务卡绑定'},
  {id:'star',name:'STAR',symbol:'?',subtitle:'自有项目平台（待公开）',hint:'通过配置的 STAR HTTPS 接口派发任务，异步回执'}
];
export const terminalStates:ReadonlyArray<TaskState>=['completed','failed','dispatched','cancelled','needs_review'];
export const stateLabels:Record<TaskState,string>={
  queued:'排队中',running:'执行中',cancelling:'正在取消',completed:'已完成',failed:'失败',
  dispatched:'已派发',cancelled:'已取消',needs_review:'需要检查'
};
export function escapeHtml(value:unknown):string{
  const lookup:Record<string,string>={'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'};
  return String(value??'').replace(/[&<>"']/g,char=>lookup[char]);
}
export function validateTask(p:TaskCreate):string|null{
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
export interface DraftTask {
  provider:AgentId;title:string;prompt:string;model?:string|null;
  repo?:string|null;node?:string|null;context?:InputContext;attachments?:Attachment[];
}
export function makeTask({provider,title,prompt,model,repo,node,context,attachments}:DraftTask):TaskCreate{
  return {provider,title:title.trim(),prompt:prompt.trim(),model:model?.trim()||null,
    target_node:node||null,repo_alias:repo||null,context:context||{},attachments:attachments||[]};
}
export type CatalogKind='project'|'issue'|'agent'|'repo';
export interface CatalogItem {id:string;label:string}
export function normalizeCatalog(output:string,kind:CatalogKind):CatalogItem[]{
  let parsed:unknown;try{parsed=JSON.parse(output)}catch{return []}
  const keys:Record<CatalogKind,string[]>={project:['projects','items','data','results'],issue:['issues','items','data','results'],agent:['agents','items','data','results'],repo:['repositories','repos','items','data','results']};
  let items:unknown=Array.isArray(parsed)?parsed:null;
  if(items===null&&parsed&&typeof parsed==='object'){
    const record=parsed as Record<string,unknown>;
    for(const key of keys[kind])if(Array.isArray(record[key])){items=record[key];break}
  }
  if(!Array.isArray(items))return [];
  return items.map((entry:unknown)=>{
    const item=(entry && typeof entry==='object'?entry:{}) as Record<string,unknown>;
    return {id:String(item.id??item.uuid??item.repositoryId??item.identifier??''),
      label:String(item.name??item.title??item.identifier??item.slug??item.id??'unknown')};
  }).filter(item=>item.id);
}
export function ago(timestamp:number):string{
  const seconds=Math.max(0,Math.floor(Date.now()/1000)-(timestamp||0));
  return seconds<60?'刚刚':seconds<3600?`${Math.floor(seconds/60)} 分钟前`:seconds<86400?`${Math.floor(seconds/3600)} 小时前`:`${Math.floor(seconds/86400)} 天前`;
}
export function icon(id:string):string{return PROVIDERS.find(p=>p.id===id)?.symbol||'✦'}
