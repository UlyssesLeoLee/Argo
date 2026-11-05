# Vercel 个人 RAG Connector 详细设计

## 连接方式

在**Rust Controller** 设置：

```dotenv
ARGO_RAG_UPSERT_URL=https://your-rag.vercel.app/api/argo/rag/upsert
ARGO_RAG_SEARCH_URL=https://your-rag.vercel.app/api/argo/rag/search
ARGO_RAG_API_TOKEN=至少20字符随机机密
```

Tauri 前端不直接连接 PostgreSQL、Vercel 数据库或者保存该 Token。仅当管理员手动批准片段后，Controller 才以 HTTPS Bearer、`Idempotency-Key` 指纹发起 POST；禁用 HTTP 远端和重定向。

## 接口契约

`POST /api/argo/rag/upsert`：提交单片知识；字段包含 `id/title/content/source_key/source_kind/provider/project/tags/fingerprint/revision`。期望 HTTP 2xx + `{"id":"...","status":"stored"}`。本地接收后更新状态为 `synced`，失败保持 `approved`。

`POST /api/argo/rag/search`：接收 `{query,project?,provider?,limit?}`，返回 `{results:[{id,title,content,provider,project,tags}],mode:"keyword"}`；Controller 对结果数量、内容长度截断。用户可将远端命中拷贝到本地草稿再整理。

## 数据库实现

`examples/vercel-rag-gateway/` 内有 Next.js 参考实现和 PostgreSQL DDL。该参考网关使用 `postgres` 包进行有参数化的 SQL 写入，按 `owner_id` 隔离。**默认只保证文本入库与关键词搜索；没有部署任何 embedding 模型或向量索引。** 接入你现有个人 RAG 需要按真实 schema 改映射器和认证策略。

## 安全/一致性

- Token 两端独立与 Tauri Admin Token；为降低泄漏风险，HTTP 错误不返回远端正文。
- 相同 ID 的同步使用 revision 检查；建议生产增补 Outbox、回执对账、重试退避、限速和安全删除。
- 向量数据库应按 tenant_id/owner_id 过滤；远端生产必须验证请求身份，不能依靠浏览器提交的 owner_id。
- Vercel Functions 是有生命周期限制的请求运行环境，长耗时重索引应放队列/Worker，不在页面请求里持续执行。
