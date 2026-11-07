# Rust 详细设计：Controller / Worker / API

## HTTP 鉴权

Admin Token 控制任务及 RAG API；Worker Token 仅节点注册/领取/报告及 Blob；两个 Token 独立且要求 ≥24 字符。正式运行应绑定 loopback 或放到 TLS 反向代理后端。禁止把 Worker Token 放在前端构建变量中。

## 核心 API

| 方法 | 路径 | 输入 | 返回 |
|---|---|---|---|
| GET | `/api/v1/providers` | — | 6 平台元信息 |
| GET | `/api/v1/nodes` | — | 注册节点与仓库别名 |
| POST | `/api/v1/tasks` | NewTask | ID、queued |
| GET | `/api/v1/tasks` | limit | TaskRow[] |
| GET | `/api/v1/tasks/{id}/events` | since | 增量事件 |
| POST | `/api/v1/tasks/{id}/cancel` | — | ok |
| POST | `/api/v1/worker/register` | node capabilities | ok |
| POST | `/api/v1/worker/claim` | node_id | TaskRow 或 null |
| POST | `/api/v1/blobs` | Binary | sha256、size |
| POST | `/api/v1/rag/chunks` | Ingest | chunk_ids、draft |
| GET | `/api/v1/rag/chunks` | query/provider/project/status | Chunk[] |
| PATCH | `/api/v1/rag/chunks/{id}` | status/tags/content/revision | ok 或 409 |
| GET/POST | `/api/v1/rag/chunks/{id}/links` | relation/target_id | links/ok |
| POST | `/api/v1/rag/task/{id}/capture` | — | 经许可创建草稿 |
| POST | `/api/v1/rag/retrieve` | query/project/provider/limit | 本地片段排名（关键词） |
| POST | `/api/v1/rag/sync` | ids[] | synced/failed |
| POST | `/api/v1/rag/remote-search` | query/project/provider/limit | Vercel results |

## 关键判定

- 如果 `repo_alias` 非空且 `context.argo_file_owner` 非空，必须有 `target_node==argo_file_owner`，否则 400。
- `argo_file_scope` 不得包含绝对路径、反斜杠、`..`、`\0`。Worker 还需要 canonicalize 并验证落在 detached Worktree 内；工作目录变更不是 OS 级隔离。
- `rag_capture` 默认 false；仅 completed / dispatched 且主动勾选的任务会采集。dispatched 仅采集派发回执；真实远程 Agent 结果必须额外对接平台回调。
- 任务租约过期标记 needs_review，避免盲重试产生外部副作用。

## 故障与观测

Blob 限 10 MiB、最多20附件；不自动转发本地密钥；RAG 入库失败保留 approved；`remote_http_xxx`/`transport_error` 不包含敏感响应；SQLite WAL 应在停机时一致性备份。
