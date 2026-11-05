# v0.3 UT / IT / ST / E2E 测试设计（IPA 形式）

| 案例 ID | 类型 | 前提/操作 | 预期结果 | 执行状态 |
|---|---|---|---|---|
| V03-UT-01 | UT | 读取 PROVIDERS | 七个唯一 id，含 star；对应 ? | JS 自动化 |
| V03-UT-02 | UT | STAR 关联已有任务缺票据 | 校验拒绝 | JS 自动化 |
| V03-UT-03 | UT | STAR 新任务具备项目 | 客户端校验成功 | JS 自动化 |
| V03-UT-04 | UT | AI 预览返回多余 ID | 服务端 502，禁止写 DB | 待 Rust IT |
| V03-UT-05 | UT | AI 预处理结果字符超过限制 | 服务端拒绝，原记录未变 | 待 Rust IT |
| V03-UT-06 | UT | AI Key 缺失或过短 | 返回 400；DB 与日志不出现 Key | 待 Rust IT |
| V03-UT-07 | UT | 片段 CAS revision 不一致 | 返回 409 | 待 Rust IT |
| V03-UT-08 | UT | 事务中多个片段任一失败 | 整批回滚，无部分保存 | 待 Rust IT |
| V03-IT-01 | IT | 2 台计算机分别运行 Worker | Node 注册、心跳、选择机器/路径正确 | 待真机 |
| V03-IT-02 | IT | 不同 FileOwner 与 Executor 写仓库 | 拒绝危险写回 | 待真机 |
| V03-IT-03 | IT | Mock STAR webhook | 接收 idempotency_key、Bearer，回执状态 dispatched | 待真机 |
| V03-IT-04 | IT | 配置 2 个 RAG 连接器 | 列出并可按 ID 选择、令牌不下发 | 待真机 |
| V03-IT-05 | IT | 连接 AI Chat completions 兼容模型 | 真实生成 tags、summary，人工预览再提交 | 待真机 |
| V03-ST-01 | ST | 全库 1,000 条混合状态 | 总数确认、进度可见、停止后已处理留历史 | 待性能环境 |
| V03-ST-02 | ST | 断网/429/429 重试模拟 | 不误标入库，不丢旧版本 | 待性能环境 |
| V03-ST-03 | ST | 有敏感数据的模型调用 | 阻止出站高风险信息/最小化，必要时人工审查 | 待安全审查 |
| V03-E2E-01 | E2E | 点击 7 Agent Tab | 专属配置区切换，主机/路径区固定 | 浏览器自动化脚本待环境可运行 |
| V03-E2E-02 | E2E | RAG 选中、标签、摘要、改写与版本恢复 | 草稿/审批/同步状态正确 | 部分 JS 契约自动化，其余待环境 |
| V03-E2E-03 | E2E | 关闭窗口再打开 | 密钥不保存在 localStorage/sessionStorage | 待 Tauri 环境 |

**不能以 JavaScript 关键词合同测试替代真实 Rust 集成测试**；本交付只对实测项目报告通过。旧版 IT/ST 文档仍适用于任务调度与文件传输。
