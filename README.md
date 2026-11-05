# Argo v0.4 — Rust + Tauri 多机 AI Agent 桌面控制台

**独立 Rust 项目，可接入 STAR。** 原生桌面端采用 Tauri 2（Rust 主进程 + TypeScript 编译的轻量 HTML/CSS WebView）；Controller、Worker、任务调度、Git 快照、人工审核写回、知识库管理与 AI 预处理在 Rust 中实现。UI 支持 Claude、Codex、Hermes、Orca、MiniMax、Multica 和尚未正式发布的 STAR（`?` 占位图标）。

> **交付类别：可构建源代码 / 开发基线。** 当前交付环境没有 `cargo` / `rustc`，未完成 Rust/Tauri 编译、双机真机联调、第三方 CLI / 外部 RAG 的现场测试；不包含已构建 EXE/MSI，也不应将其误称为通过生产验收的软件。TypeScript 前端/JavaScript 契约测试和离线 Git 快照/补丁测试的结果见 `docs/08_v04/04_验证报告与限制.md`。

## 0. TypeScript 与 Rust 的职责

Argo 的桌面 GUI 由 **Tauri 2 + TypeScript + CSS** 构成，前端源码统一位于 `src/*.ts`；`npm run build:web` 只把 TypeScript 编译为 WebView 执行的 ES2022 JS。**业务执行、Agent 调度、文件同步、RAG 数据库、机密凭据均在 Rust 端**。采用 `tsc` 直接输出 ES 模块（无 Vite / React 运行时依赖），降低安装链复杂度；生产打包后不需要运行 Node.js 服务。

更多见 `docs/08_v04/05_TypeScript前端和Tauri集成详细设计.md`。

## 1. Windows 首次启动（桌面版）

**前置条件**：Windows 11；Rust stable（按 Tauri 2 当前依赖要求建议 Rust ≥1.90）；Node.js 22+、npm、Git；Microsoft C++ Build Tools、WebView2，参考 [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)。所选 Agent CLI 需另行安装并登录。依赖版本以 `Cargo.toml`、`package.json` 为准。

```powershell
cd Argo
npm install
rustc --version
cargo --version
npm run dev
```

`npm run dev` 会先构建 Rust `argo-runtime`，将可执行文件复制为 Tauri sidecar 所需的 `argo-runtime-<目标三元组>.exe`，随后打开 Tauri。首次打开将建立本机 SQLite 数据库，生成私有 Controller/Worker token，自动启动并停止本机 Controller/Worker。

打包桌面安装程序：

```powershell
npm run build
```

输出位于 `target/release/bundle/`（具体按目标平台与 Tauri 构建器而定）。**请勿直接手动运行 `tauri build`，否则可能缺少 sidecar；统一使用 `npm run build`。**

在 **设置 → 本机 Daemon / 仓库** 填写：仓库别名如 `project`、本机 Git 仓库的**根目录绝对路径**（例如 `D:\projects\Star`）。Worker 随后重启并注册 Git 仓库。文件归属机器、仓库、路径属于同一组；Agent 执行节点是独立选择项。

## 2. 无 Rust / 无 CLI 前端预览

```powershell
npm run preview
# 浏览器访问 http://127.0.0.1:8899/?demo=1
```

演示不会执行第三方 Agent、不会调用个人 AI API、不会上传外部 RAG。

## 3. 单台 / 多台机器的运行模型

```text
Tauri 2 桌面 UI ── Tauri Rust IPC ── 本机 Rust Controller (SQLite + HTTPS 代理边界)
                                           │
                   ┌───────────────────────┼──────────────────────┐
                   │                       │                      │
              Owner Worker A          Executor Worker B      RAG Connector
              源文件机器                AI 运行机器              任意兼容 HTTPS
              Git 快照/写回            临时工作区 / 输出         upsert/search
```

- **同机模式**：Agent 运行在持有仓库的 Worker；为每个任务创建独立 Git Worktree。
- **跨机模式**：文件归属机器 A 仅导出经授权的 `HEAD` 文件范围为 Git tar 快照（≤40 MiB）；执行机器 B 下载并在独立目录运行 Agent；B 上传 SHA-256 校验的 `changes.patch`；管理员先在 UI 查看 Diff，再明确批准，A 验证原始 HEAD 并在**新的隔离 Git Worktree / 分支**中应用。**不会自动合并到原始工作目录。**
- **任务状态**：API 心跳、节点可用性、任务租约、完成事件和审批状态分别保存；租约异常需人工检查后恢复，不自动重放可能有副作用的 Agent 操作。
- **运行限制**：跨机快照流程用于直接编辑代码的 Claude / Codex / Hermes / MiniMax。Orca / Multica / STAR 属于上层任务编排平台，使用各自接口，不能将其当成同一种“远程命令执行”。

### 多机 Controller 与 Worker（手工/服务器）

见 `examples/.env.example` 和 `docs/08_v04/02_双机部署与执行手顺.md`。在一台可信机器运行中心 Controller，其他机器设置 `ARGO_SERVER=https://...`（推荐 WireGuard/Tailscale/VPN + TLS 反向代理）并运行 Worker。Bearer Token 由用户管理，不要把未经 TLS 保护的 Controller 暴露公网；现阶段**未实现每节点独立 mTLS 身份**。

## 4. 七平台入口与输入适配

- Claude / Codex / Hermes：特有参数转换后调用机器上实际安装的 CLI，权限、模型参数和登录状态依 CLI 版本调整。
- MiniMax：使用配置了 MiniMax 模型的 Hermes 兼容调用，不假设固定的 `mcode` 命令。
- Orca：以 Worktree/分支/Agent 等参数驱动其 CLI，属于编排器调用，派发成功不等同完成。
- Multica：需 Workspace / Project / Issue / Action 等结构化上下文，不只是一段 Prompt。
- STAR：目前仅保留问号 Logo，按用户显式配置的 HTTPS webhook 派发；**未实现未公开 STAR 服务端**。回执表示接受任务，不表示 STAR 执行完成。

可参照 `docs/03_detailed_design/05_平台CLI适配器.md` 核对具体适配参数。外部平台更新 CLI 后，应先运行 `scripts/verify-clis.ps1`，再在本机集成测试后启用。

## 5. 个人 RAG 与个人 AI Key

个人 RAG 页面包含采集 / 分段 / 检索 / 审核 / 关联 / 归档 / 版本恢复；只有审核通过的记忆允许主动同步。外部 RAG 接口由**用户自行配置**，不锁定 Vercel 或其他部署商。桌面设置可保存 JSON HTTP 连接器的 upsert/search HTTPS URL 与独立凭据，**前端不显示服务商名称或后台密钥**。

AI 前处理支持用户填写 OpenAI 兼容 Chat Completions 的服务地址、模型与 **本次请求** API Key，提供智能标签、摘要、结构提取和统一规则批量改写。批量操作按 8 条预览/分批确认，保留版本历史；更改后回到“待审核”状态。请自行确认第三方模型的数据处理政策，机密数据不应发送给不受信模型。

API 与安全契约：`docs/08_v04/03_RAG与AI处理设计.md`；旧版示例的 Vercel 网关仍在 `examples/` 仅供可选部署，**产品 UI 与核心协议均不依赖 Vercel**。

## 6. 测试与构建

```powershell
npm run typecheck                      # TypeScript 编译与核心严格类型检查
npm test                              # 编译 TS 并运行 80 项契约与 UI 逻辑检查
python tests/smoke_snapshot_patch.py # 在临时 Git 仓库测试范围快照/补丁/审阅工作树
cargo test -p argo-runtime           # Rust 单元测试（需要 Rust 环境）
cargo check -p argo-desktop          # Tauri 编译检查（需要 Rust/Tauri 环境）
python tests/integration_cluster.py  # 真实 Controller + 2 Worker E2E（需要 Rust 环境）
```

或者执行 `scripts/test-all.ps1` / `.sh`。其中 Rust/Tauri 和双机 E2E 尚未在本交付环境真实执行；安装前应在 Windows/Linux 上进行预检。

## 7. 项目文件

- `crates/argo/src/`：Rust Controller / Worker / seven adapters / RAG / AI 预处理。
- `src-tauri/`：Tauri 2 项目、本机 Daemon sidecar 管理、仓库/连接器设置、安全 IPC、应用图标。
- `src/`：前端 TypeScript 源码和接口契约；`web/`：编译产物 + HTML/CSS（不用 React、Next.js 或 Node.js 后端）。
- `design/`：三张可维护 SVG 设计稿、PNG 预览、Argo Logo、STAR `?` 占位 Logo。
- `docs/`：要件定义、基本设计、详细设计、UT/IT/ST/E2E、运维、STAR SDK 与 v0.4 增量规范。
- `examples/`：环境变量、可选 RAG 网关和 STAR webhook 契约。
- `scripts/`：sidecar 自动构建、CLI 预检与测试脚本。
- `tests/`：80 项 JS 契约、Rust 单测、临时 Git smoke 与真实双 Worker E2E 脚本。
- `requirements_and_tests.xlsx`：原需求 / 测试 / WBS 矩阵，v0.4 增量以 `docs/08_v04/` 为准。

许可：本仓库 MIT；所有外部 AI CLI、SDK、模型和云服务的商业许可需分别确认。

## 8. Windows 一键入口

```powershell
powershell -ExecutionPolicy Bypass -File scripts/windows-first-run.ps1
# 编译安装包：
powershell -ExecutionPolicy Bypass -File scripts/windows-build.ps1
```

前端编译产物无需 Node.js 后台驻留；Node 只用于开发/打包。当前源码中原生 Rust sidecar 安装包仍需在 Windows 编译并验证。
