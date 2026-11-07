# 详细设计：STAR 适配（预发布占位）

UI 七号选项卡：STAR，图标仅为 `?`，显示项目 ID、任务模式和可选任务卡 ID。由于 STAR 未推出，不假设它具有公开 CLI，也不虚构实体 ID 发现 API。

Worker 只有配置 `ARGO_STAR_DISPATCH_URL` (HTTPS) 和 `ARGO_STAR_SERVICE_TOKEN`（不少于 16 字符）才向平台注册 STAR 运行能力。请求 JSON：

```json
{
 "idempotency_key":"Argo-task-uuid", "source":"argo", "task_id":"Argo-task-uuid",
 "title":"任务摘要", "prompt":"任务输入", "project_id":"star-project", "ticket_id":"STAR-100", "mode":"existing"
}
```

成功标准：HTTP 2xx 且响应 JSON，形如 `{ "id":"STAR-remote-id" }`。Argo 标记为 `dispatched`；要判断最终完成，未来须增加远端事件回调、签名验证、轮询、超时、对账与 Worktree 安全写回，不得仅凭派发回执标记 `completed`。

设计细节：STAR 是独立平台 Adapter，不与 Rust Worker 的本机 CLI 路径混写，不能把 `?` 视为永久商标；待 STAR 发布后更新 logo、Schema 及真实 IT。
