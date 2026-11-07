# Argo v0.5.0 Source Package

完整 Rust Controller + Worker / Tauri 2 + TypeScript GUI / 7 Agent adapters / RAG / Logos / IPA 风格资料。

**当前已验证**：`npm run check:all` 92/92 通过；Python Git 快照冒烟通过；Git 安全正反向测试 5/5 通过；ZIP 完整性通过。

**未验证**：Rust/Tauri 编译、Windows 安装包、双机 HTTP E2E、第三方 CLI/STAR/外部 RAG 真机。应用还没有 OS 沙箱和任务级 Blob ACL，不可直接向不可信节点部署。

新增资料在 `docs/09_v05_security_audit/`；红绿日志在 `reports/`；Windows 一键复验 `scripts/red-green.ps1`；矩阵 `Argo_v05_red_green_matrix.xlsx`。
