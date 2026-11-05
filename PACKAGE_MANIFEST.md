# Argo v0.4.0 — 交付内容清单

- 交付形式：**Rust + Tauri 2 + TypeScript 源码 ZIP**，非已编译的 EXE/MSI。
- `src/*.ts`：前端唯一可维护源码，类型化七平台、机器与 RAG 接口；`web/*.js` 是 TypeScript 预编译产物。
- `src-tauri/`：Rust Tauri 2 主进程及 sidecar 管理，Bundler 已启用（需要 Rust 编译环境）。
- `crates/argo/`：Rust Controller、Worker、AI Agent、Git 跨机快照及 RAG/AI 功能。
- `design/`：可编辑 SVG / PNG 设计稿，以及 Argo 和 STAR ? 占位 Logo。
- `docs/`：要件定义、基本/详细设计、UT/IT/ST/E2E、运营与 v0.4 TS 专项设计。
- `examples/`：STAR SDK、通用接口/可选个人 RAG 网关示例。
- `scripts/`：开发预览服务器、TS watch、Rust sidecar 构建、CLI 验证和自动化测试。
- `tests/`：80 项 Node 契约及离线 Git 快照/补丁冒烟脚本、Rust/集群 E2E 测试脚本。
- `requirements_and_tests.xlsx`：需求追踪、测试矩阵、WBS、Agent 与 RAG 数据字典、v0.4 专项表。

## 当前验证状态

**通过**：`npm run typecheck`；`npm test`（80/80）；`python tests/smoke_snapshot_patch.py`。ZIP 完整性见 `SHA256SUMS.txt` 和压缩包 CRC 校验。

**未验证**：真实 Rust/Tauri 编译、Windows 安装包、七 CLI 全量真实兼容性、双机实际执行、第三方 RAG 联调。不得视为生产可用版本。
