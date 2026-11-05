# Vercel 个人 RAG 接收端（独立部署参考）

该模块**不是你现有个人 RAG 的既有接口**，而是可选的、可部署的接收端参考实现。它能接收 Argo 人工批准的片段，在 Vercel 上通过 Next.js Route Handler 写入外部 PostgreSQL（Vercel Functions 本身不持久化数据库）。

1. 在 PostgreSQL 运行 `sql/001_create_rag.sql`。
2. 在 Vercel 配置 `DATABASE_URL`、`ARGO_RAG_API_TOKEN`、`ARGO_RAG_OWNER_ID`。
3. 部署本子项目，并在 Argo Rust Controller 配置 `ARGO_RAG_UPSERT_URL=https://<你的域名>/api/argo/rag/upsert`、相同的 `ARGO_RAG_API_TOKEN`。
4. 从 Argo 个人 RAG 页面审核片段并手动点击「同步到 Vercel RAG」。
5. 此实现提供 `POST /api/argo/rag/search` 的关键词搜索接口。**仅入库文本，不自动生成向量**，数据库的 `embedding_status` 默认 `pending`。接入你现有 embedding 管线后才能宣称语义检索；SQL 提供可选 pgvector 扩展注释。

生产环境需另行落实：用户身份 / tenant 隔离、上传频率限制、队列与重试、审计、凭据轮换、embedding worker、PITR 备份和已有 RAG 的字段映射。本示例未在线部署或连接真实数据库。
