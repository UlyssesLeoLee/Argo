# 详细设计：与部署商无关的 RAG Connector

## 管理

Controller 环境变量 `ARGO_RAG_CONNECTORS_JSON` 是只包含连接 ID、显示名称、HTTPS 端点和 token_env 的配置数组。Token 值必须作为单独的环境变量提供，**不通过 UI 或模型提示发送**。前端 `GET /api/v1/rag/connectors` 只能看到 `id/name/can_sync/can_search`。

## 同步与检索

- `POST /api/v1/rag/sync`：`{"ids":["rag-..."],"connector_id":"personal"}`。仅批准版，SHA256 幂等键；任意错误保留本地记录与失败审计。
- Connector 约定 `POST HTTPS upsert_url` JSON: `{id,source_key,source_kind,provider,project,title,content,summary,tags,fingerprint,revision}`，Bearer Token，预期 2xx JSON `{id:"REMOTE_ID"}`。
- Connector 约定 `POST HTTPS search_url` JSON: `{query,project,provider,limit}`，预期 JSON `{results:[{id,title,content,provider,project,tags}]}`。
- 各种 SaaS 或自托管 RAG 若 API 不兼容，应新增独立 Adapter；不得冒用这套协议能直接连接所有 RAG 数据库。

## 安全

只允许服务端明确配置的 HTTPS URL，禁止浏览器直接注入任意远端地址以规避 SSRF；禁止经过不可信重定向。生产应验证主机 DNS 与出口策略、证书及用户授权，实施租户隔离与删除同步。

## 与旧版兼容

旧变量 `ARGO_RAG_UPSERT_URL`、`ARGO_RAG_SEARCH_URL`、`ARGO_RAG_API_TOKEN` 将映射为 `default` 连接器。`examples/vercel-rag-gateway/` 是一个可以选择部署的实例，不是前端品牌或必须依赖。
