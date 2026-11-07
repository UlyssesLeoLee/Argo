# Argo v0.5.0 — Red/Green 安全修订版

本次以 v0.4 的源码为基线；不删除任何七平台、RAG、Tauri TypeScript、SVG、Logo 与 Star 集成内容。

- 修复 Worker 路径作用域 Rust 字符转义错误，收紧 node_id 目录命名规则。
- Remote Cluster 改为可配置的 **每节点独立 token**；本机共享 token 需明确在 loopback 模式开启。
- 过期任务不能继续上报事件、附件、租约或结果；选择执行节点时检查在线与 Agent 能力。
- 防止批准未完整呈现的 >200kB Git Patch，同时检查 SHA-256。
- 从 `ARGO_REPOS` 分隔符转为 `ARGO_REPOS_JSON`，避免合法 Windows 路径含 `;` 或 `=` 的歧义。
- RAG AI 预览增加服务端 10 分钟有效、单次消费的 `preview_id`；应用前比较每条变更与预览快照。
- TypeScript 和 Tauri IPC 同步预览凭证；RAG 修改仍先进入待审核，保留修订恢复。
- 新增 12 项 ADV 回归（v0.4 首轮 7 红 / 3 绿，v0.5 全部通过）；保留 80 项原有检查。
- 新增真实临时 Git 仓库 5 项正反向行为测试，Windows / Linux 红绿一键脚本、IPA 风格对抗式审查资料、矩阵与结果日志。

**明确限制**：Rust/Tauri 未在此交付环境完成构建；不包含 EXE/MSI；OS 沙箱、任务级 Blob ACL 和真实双机 / 七 CLI / RAG 联调仍是发布阻断项。
