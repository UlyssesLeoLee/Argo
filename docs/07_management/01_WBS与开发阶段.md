# Argo v0.2 WBS（按成果而非过程组织）

| ID | 工作包 | 交付物 | 验收出口 | 阶段 |
|---|---|---|---|---|
| W01 | 统一视觉与导航 | Comic UI、6 Tab、单 Agent 输入 | UI 审查通过 | P1 |
| W02 | 文件目标机器层级 | owner→repo→scope + Runner | 无歧义，不跨机误写 | P1 |
| W03 | Rust RAG 本地库 | SQL schema、ingest、review、links | UT/IT 通过 | P1 |
| W04 | 语义片段优化 | token-aware/结构化章节、质量筛选 | 人工抽检准确率 | P2 |
| W05 | Vercel Connector | HTTPS upsert/search、幂等 | 真实 DB 查询一致 | P1 |
| W06 | Tauri 2 包装 | Windows 安装程序、原生 IPC | Windows 构建及权限审计 | P1 |
| W07 | 平台适配器联调 | 6 CLI 参数预检与日志回调 | 每个平台真实 UT/IT | P2 |
| W08 | 异机编辑审批 | 快照、patch、冲突检测、源机回写 | 故障注入无数据损坏 | P3 |
| W09 | 云端向量化与检索 | Embedding Worker、pgvector | 语义召回指标 | P2 |
| W10 | Star 集成 | 服务端 SDK + RBAC + 无限画布入口 | E2E | P2 |
| W11 | 运维与安全 | TLS、隔离、备份、删除、审计 | 安全评审 | P2 |

P1 是原型开发基线；P2 是首次生产候选；P3 才开放跨机器回写。不能仅因为页面已有“执行节点”就认为 P3 完成。
