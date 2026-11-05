# ADR 与风险决策

- **ADR-001**：Rust Controller/Worker 独立于 Star；理由：独立生命周期、部署与 CPU/GPU 调度。
- **ADR-002**：Tauri 2 轻量 HTML ES Modules；理由：无需复杂前端状态库即可保持桌面运行性能；后续可替换为框架。
- **ADR-003**：六平台 Tab + 同屏只配置一个 Agent；理由：降低 Hick 选择成本，保持空间锚点。
- **ADR-004**：文件归属与执行位置两实体；理由：防止“同名路径错误机器”导致破坏性修改。
- **ADR-005**：RAG 本地 draft→approved→synced，上传 opt-in；理由：控制机密泄漏与未经审核内容污染。
- **ADR-006**：Vercel 只提供 API + 持久化 Postgres；重计算/embedding 不放在长寿命 Vercel 请求内。
- **ADR-007**：本版禁用异机写回；理由：缺跨机文件快照、冲突检测、审批，不可伪称完成。
- **ADR-008**：只保证词法检索；已创建远端 schema 不等于模型 Embedding 或向量召回正常。

## 风险矩阵

| 风险 | 严重性 | 缓解 |
|---|---|---|
| API Token 被浏览器泄漏 | 高 | Tauri 走 IPC；生产 Star 走 server-only；密钥不进 bundle |
| Prompt/RAG 含敏感内容 | 高 | 采集默认关闭、脱敏、人工审核、最小上传 |
| CLI 运行权限越界 | 高 | Repo 白名单/Worktree 仅基础保护；未来增 OS 沙箱 |
| 源节点与执行节点不一致 | 高 | 跨机写入拒绝 |
| 异步 Multica 误判完成 | 中 | 独立 dispatched 状态 |
| Vercel schema 与现有 RAG 不符 | 高 | 先契约映射与模拟测试，禁止直接迁移用户数据 |
| SQLite 大规模检索缓慢 | 中 | 索引、限制及后续分页/远端检索 |
| 同步回执后本地更新冲突 | 中 | revision + Idempotency-Key；未来 Outbox 对账 |
