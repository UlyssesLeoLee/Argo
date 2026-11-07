/** Argo public UI contracts. These align to Rust serde payload names. */
export const AGENT_IDS = ['claude','codex','hermes','orca','minimax','multica','star'] as const;
export type AgentId = typeof AGENT_IDS[number];
export type TaskState = 'queued'|'running'|'cancelling'|'completed'|'failed'|'dispatched'|'cancelled'|'needs_review';
export type InputContext = {
  argo_file_owner?:string;
  argo_file_scope?:string;
  rag_capture?:boolean;
  rag_refs?:string[];
  orca_repo_id?:string;
  orca_agent?:string;
  multica_workspace_id?:string;
  multica_project_id?:string;
  multica_issue_id?:string;
  multica_assignee?:string;
  multica_mode?:'new'|'existing';
  star_project_id?:string;
  star_ticket_id?:string;
  star_mode?:'new'|'existing';
};
export interface Attachment {name:string;sha256:string}
export interface TaskCreate {
  provider:AgentId;title:string;prompt:string;model:string|null;
  target_node:string|null;repo_alias:string|null;context:InputContext;
  attachments:Attachment[];
}
export interface NodeSummary {
  id:string;hostname:string;os:string;last_seen:number;
  repos_json:string;providers_json:string;max_parallel:number;
}
export type RagState = 'draft'|'approved'|'synced'|'archived';
export interface RagChunk {
  id:string;title:string;content:string;provider:AgentId|string;project:string;
  source_kind:string;source_key:string;tags_json:string;status:RagState;
  revision:number;created_at:number;updated_at:number;remote_id:string|null;
  summary?:string|null;
}
export type ApiMethod = 'GET'|'POST'|'PATCH';
export interface NativeInvoke {
  <T=unknown>(command:string,args?:Record<string,unknown>):Promise<T>;
}
export interface NativeDaemonStatus {
  controller_started:boolean;worker_started:boolean;node_id:string;
  repositories:Record<string,string>;connectors:Array<{id:string;name:string;has_upsert:boolean;has_search:boolean}>;
  last_error:string|null;
}
export type RagBatchOperation = 'tags'|'summary'|'structured'|'rewrite';
export interface RagRevision {id:string;revision:number;title:string;content:string;summary:string;tags:string[]}
