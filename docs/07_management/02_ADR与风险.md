# Argo ADR（架构决策记录）与风险登记

ID：ARGO-ADR-001 · 版本：1.0

## 决策记录

- **ADR-001：独立仓库，非 Star 子系统。** 原因：生命周期、部署、Agent 执行与 Star Canvas 解耦；Star 通过 API 接入。
- **ADR-002：Rust 作为 server/worker 核心。** 原因：多操作系统、低常驻资源、Task/Process 可控。
- **ADR-003：每平台独立 adapter。** 原因：Multica 的核心单位是 Issue、Orca 是 Worktree，不能伪装为普通 chat API。
- **ADR-004：首版 Worker 主动拉取。** 原因：跨网络/NAT 易部署、无需 Worker 入站服务；牺牲 2 秒轮询延迟。
- **ADR-005：SQLite WAL 起步。** 原因：独立、易部署；多控制面 HA 迁至 PostgreSQL。
- **ADR-006：内容寻址 Blob + Git patch。** 原因：避免整仓重复上传；不能作为透明同步盘。
- **ADR-007：租约失效不自动重新执行。** 原因：模型 Agent 和 Multica/Orca 具有不可回滚副作用。
- **ADR-008：`dispatched` 与 `completed` 分离。** 原因：CLI 的派发回执不等于后台 Agent 的最终工作结果。
- **ADR-009：MVP 不宣称 OS 级沙箱。** 原因：Git Worktree 不能阻止 CLI 访问 HOME 或系统路径，必须在生产部署前补强。

## 风险登记

| Risk | 可能性/影响 | 风险 | 缓解/负责人角色 |
|---|---|---|---|
| R-01 | 高/高 | Rust 未实际编译，潜在 API 兼容错误 | 开发负责人在目标环境跑 cargo fmt/check/test |
| R-02 | 高/高 | 第三方 CLI 升级变更参数 | Adapter 集成测试 + 版本锁定 |
| R-03 | 中/高 | Worker Token 横向扩权 | 每节点 mTLS/Scoped Token（P1） |
| R-04 | 中/高 | Agent 可访问宿主机其他文件 | OS 强沙箱、临时用户隔离（P1） |
| R-05 | 中/高 | Multica Issue 触发重复 Agent 或账单 | 明确派发副作用、人工确认、幂等外部 ID（P1） |
| R-06 | 中/中 | 任务租约超时产生孤儿进程 | needs_review + runbook + 节点状态回收 |
| R-07 | 中/中 | 输出文件未跟踪，diff 无法捕获 | 提示 output 目录、Artifacts 校验 |
| R-08 | 中/中 | 真实模型并发导致 OOM | 每节点限额，后续 GPU 资源感知 |
| R-09 | 中/中 | 本地 SQLite 容量上限 | Postgres/S3 迁移设计、清理策略 |
| R-10 | 中/高 | Star 误暴露 Admin Token | 服务端 Proxy 和 Star RBAC 保障 |
