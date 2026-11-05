# Argo WBS / 版本实施计划

ID：ARGO-PM-001 · 基线：2026-11-05

| WBS ID | 阶段 | 工作包 | 输出/结束条件 | 当前状态 |
|---|---|---|---|---|
| 1.1 | P0 | 六平台 UI、条件字段 | 表单/查询/验证 | 源码已交付，待浏览器验收 |
| 1.2 | P0 | Rust server + DB | `/api/v1` 契约与 SQLite | 源码已交付，待 cargo 编译 |
| 1.3 | P0 | 两节点 Worker | 注册/Claim/取消/租约 | 源码已交付，待双机实测 |
| 1.4 | P0 | 六平台 Adapter | argv 输入/返回映射 | 源码已交付，待真实 CLI 测试 |
| 1.5 | P0 | Blob 与 Git diff | SHA、changes.patch、Output | 源码已交付，待集成验证 |
| 1.6 | P0 | UT/IT/ST 测试 | 测试用例及脚本 | JS UT 已执行；其余待执行 |
| 2.1 | P1 | Rust 编译与跨 OS CI | Windows/Linux/macOS green | 未开始 |
| 2.2 | P1 | 平台认证健康检查 | CLI 版本、登录状态探测 | 未开始 |
| 2.3 | P1 | Star 服务端集成 | SSO/RBAC/Task ID Mapping | 仅示例代码 |
| 2.4 | P1 | 安全沙箱 | Linux 容器、Windows Job Object/VM | 未开始 |
| 2.5 | P1 | Multica/Orca 下游追踪 | platform_run_id、真实完成状态 | 未开始 |
| 3.1 | P2 | 多租户授权 | Node/Task/Blob scoped credential | 未开始 |
| 3.2 | P2 | Postgres + S3 | 多控制面 HA/海量 Blob | 未开始 |
| 3.3 | P2 | 资源感知调度 | GPU/VRAM/CPU/内存余量 | 未开始 |
| 3.4 | P2 | Agent 会话协议 | ACP/MCP、交互审批、续聊 | 未开始 |
| 3.5 | P2 | 事件推送 | SSE/WebSocket、观测追踪 | 未开始 |
| 4.1 | P3 | GPU 联合推理 | vLLM 等专用推理后端 | 未开始，非核心需求 |

## Release Gate

v0.1 只是代码与规范交付，不代表生产发布。发布候选版 RC 的必要条件：`cargo fmt/check/clippy/test` 通过、真实 Windows/Linux 双节点 IT 通过、各选择的第三方 CLI 至少一套可用、权限路径穿越修复、安全扫描通过、UI 视觉和 E2E 现场验收完成。
