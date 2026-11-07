# v0.5 开发 WBS（测试驱动，Release Gate）

| ID | 级别 | 工作包 | 交付验收条件 | 状态 |
|---|---|---|---|---|
| P0-01 | 最高 | Windows 11 Rust/Tauri 编译打包 | `cargo test`、`cargo check`、EXE/MSI 真实安装成功 | 未验证 |
| P0-02 | 最高 | 无信任 Agent 的 OS 级沙箱 | Windows 受限 Token/Job 或 Linux namespace/容器，测试无法越界读写 | 未实现 |
| P0-03 | 最高 | Blob / 文件访问按任务授权 | Node token 仅能访问已授权任务的 Blob，过期后拒绝 | 未实现 |
| P0-04 | 最高 | 双机 E2E 及恢复测试 | 分别模拟 Worker 失联、租约超时、重复 claim、Git HEAD 变化 | 未验证 |
| P1-01 | 高 | 集群节点密钥轮换、吊销和证书策略 | 独立身份、吊销后 401、审计记录 | 未实现自动化 |
| P1-02 | 高 | RAG 全库批次任务管理 | 中断恢复、幂等 batch_id、跨批审核与审计 | 部分实现 |
| P1-03 | 高 | 安全存储 Secret | Windows DPAPI / Credential Manager，文件迁移与凭据轮换 | 未实现 |
| P1-04 | 高 | RAG 连接器能力发现 | 匹配不同 RAG 平台 upsert/search schema，错误处理与回滚 | 参考实现 |
| P1-05 | 高 | CLI 版本适配矩阵 | 七平台实机输入/输出契约 + Feature Flags | 待联调 |
| P2-01 | 中 | 前端简洁性与无障碍检查 | 仅当前 Agent 的设置可见，目标机器与路径归一，键盘操作完整 | 静态布局已提供，待 UX |

**放行原则**：P0 全部验收前不得面向不可信组织开放跨机写入功能。开发包中的 `red/green` 是已观察到的代码测试状态，不代表生产安全认证。
