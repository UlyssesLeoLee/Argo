/**
 * Star / Next.js 服务器专用 Argo REST adapter.
 * 不要在浏览器组件里导入；不要使用 NEXT_PUBLIC_ 暴露密钥。
 * 调用此模块前，必须在 Star 自身 API 上校验会话、租户、项目和节点使用权限。
 */
import 'server-only';

export type Provider = 'claude' | 'codex' | 'hermes' | 'orca' | 'minimax' | 'multica';
export type DispatchState = 'queued' | 'running' | 'cancelling' | 'cancelled' | 'completed' | 'failed' | 'dispatched' | 'needs_review';
export interface ArgoTaskInput {
  title: string;
  prompt: string;
  provider: Provider;
  model?: string | null;
  target_node?: string | null;
  repo_alias?: string | null;
  context?: Record<string, unknown>;
  attachments?: Array<{name: string; sha256: string}>;
}
export interface ArgoTask {
  id: string;
  title: string;
  provider: Provider;
  state: DispatchState;
  target_node?: string;
  assigned_node?: string;
  created_at: number;
  started_at?: number;
  finished_at?: number;
  output?: string;
  error?: string;
  artifacts_json?: string;
}
export interface ArgoNode {
  id: string;
  hostname: string;
  os: string;
  providers_json: string;
  repos_json: string;
  max_parallel: number;
  last_seen: number;
}
export interface ArgoEvent {
  seq: number;
  stream: 'system' | 'stdout' | 'stderr';
  content: string;
  created_at: number;
}

const getConfig = () => {
  const url = process.env.ARGO_SERVER_URL;
  const token = process.env.ARGO_ADMIN_TOKEN;
  if (!url || !token) throw new Error('Missing ARGO_SERVER_URL or ARGO_ADMIN_TOKEN on Star server');
  const base = new URL(url);
  if (base.protocol !== 'https:' && base.hostname !== 'localhost' && base.hostname !== '127.0.0.1') {
    throw new Error('Argo remote control plane must use HTTPS');
  }
  return {base: base.toString().replace(/\/$/, ''), token};
};

async function argo<T>(path: string, init?: RequestInit): Promise<T> {
  const {base, token} = getConfig();
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(init?.body != null ? {'Content-Type': 'application/json'} : {}),
      ...init?.headers,
    },
    cache: 'no-store',
    signal: init?.signal ?? AbortSignal.timeout(25_000),
  });
  if (!res.ok) {
    const reason = await res.text().catch(() => 'Unknown upstream error');
    throw new Error(`Argo HTTP ${res.status}: ${reason.slice(0, 200)}`);
  }
  return res.json() as Promise<T>;
}

export const listArgoNodes = () => argo<ArgoNode[]>('/api/v1/nodes');
export const listArgoTasks = (limit = 100) => argo<ArgoTask[]>(`/api/v1/tasks?limit=${Math.min(300, Math.max(1, limit))}`);
export const getArgoTask = (id: string) => argo<ArgoTask>(`/api/v1/tasks/${encodeURIComponent(id)}`);
export const listArgoEvents = (id: string, since = 0) => argo<ArgoEvent[]>(`/api/v1/tasks/${encodeURIComponent(id)}/events?since=${Math.max(0, since)}`);
export const submitArgoTask = (task: ArgoTaskInput) => argo<{id: string;state: 'queued'}>('/api/v1/tasks', {method: 'POST', body: JSON.stringify(task)});
export const cancelArgoTask = (id: string) => argo<{ok: true}>(`/api/v1/tasks/${encodeURIComponent(id)}/cancel`, {method: 'POST', body: '{}'});
