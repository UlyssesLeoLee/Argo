# Argo v0.4 — TypeScript / Rust / Tauri 专项测试设计

| ID | 层级 | 输入/前提 | 操作 | 预期结果 | 本次验证 |
|---|---|---|---|---|---|
| TS-01 | UT | 源码树 | 检查 `src/*.ts` 和 `web/` | 前端唯一手写源码为 TS，产物由 tsc 生成 | PASS |
| TS-02 | UT | TypeScript 5.x | `npm run typecheck` | 无编译错误；关键契约 strict | PASS |
| TS-03 | UT | 七平台数据 | 运行输入校验 | 缺失 Orca/Multica/STAR 必需参数时拒绝 | PASS |
| TS-04 | UT | RAG 字符串 | 分段/脱敏/路径检查 | 中文、emoji 保留；敏感凭据移除预览 | PASS |
| TS-05 | IT | Tauri 配置文件 | 检查 Build Command | 打包前必须 `npm run build:web` | PASS（静态） |
| TS-06 | IT | Rust sidecar | 启动桌面 → 关闭窗口 | Controller/Worker 自动启动和停止，无残留 | 待 Windows 验收 |
| TS-07 | IT | 本机 Agent | 选 Agent → 配置任务 → 执行 | Rust 返回任务号，CLI 输出和退出码正确 | 待实机验收 |
| TS-08 | IT | 两台机器 + Git 仓库 | 指定源机器 A + 执行机器 B | B 修改隔离快照；A 审核后独立 Worktree 写回 | Git smoke PASS，跨机待验收 |
| TS-09 | ST | Windows 11 构建机 | `npm run build` | 生成可安装包，运行不依赖 Node.js 后台 | 待构建 |
| TS-10 | ST | RAG 服务/用户 AI Key | 预览标签、摘要、统一改写 | 审核前不远端同步；版本恢复可用 | 静态合同 PASS，真机待验收 |
| TS-11 | ST | 无 Agent、无 Key | `npm run preview` | 能查看完整 demo，绝不调用外部真实模型 | PASS（构建） |
| TS-12 | Security | Tauri UI | 检查 IPC/日志/密钥 | 前端无法读取本机管理员 Token；临时 AI Key 不落库 | 静态 PASS，渗透测试待实施 |

**本次执行记录（2026-11-05）**：`npm run typecheck` 成功；`npm test` 的 80 项 Node 契约测试全部通过；`python tests/smoke_snapshot_patch.py` 通过。当前执行环境无 Cargo/Rustc，Rust/Tauri 编译与真实双机 E2E 未执行。不能把静态代码断言视为可运行集成测试。
