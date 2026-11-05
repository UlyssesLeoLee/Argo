# Argo v0.3.0 — 交付文件清单

- 交付日期：2026-11-05
- 范围：源码、界面 SVG/PNG、Logo、接口配置、Markdown 设计与测试文档、Excel 需求/测试/WBS、脚本。
- 验证：49/49 JavaScript 契约测试通过；JS/JSON/YAML/SVG/Python 静态解析通过；ZIP CRC 验证通过。
- 未验证：Rust/Tauri 编译，第三方 CLI 调用真机，STAR 真实 API，双机联网与任意外部 RAG 联调。

## 内容模块

- **前端及 Tauri 应用**：`web/ + src-tauri/`
- **Rust Controller、Worker、CLI 适配与 AI 处理**：`crates/argo/`
- **SVG UI 原稿、PNG 预览、品牌图标**：`design/`
- **要件、基本、详细、UT/IT/ST/E2E、运维与 WBS 设计**：`docs/`
- **配置、OpenAPI、Star 与通用 RAG 对接示例**：`examples/`
- **自动化与预检脚本**：`scripts/ + tests/`
- **追踪矩阵**：`requirements_and_tests.xlsx`

## 安全与功能约束

- RAG 用户密钥仅为当前请求传输，不内置硬编码密钥；生产环境仍须配置机密管理和 TLS。
- 全库 AI 整理先预览与确认，进行受控批量变更，保留历史版本，改写后进入待审核。
- 所有外部 RAG 提供商通过通用 HTTPS JSON 契约接入，不在客户端品牌化。
- STAR 是尚未公开的平台占位适配，使用问号图标，需提供真实 API 方可运行。
- 跨机派发与跨机文件写回不是同一件事：目前写入执行仍要求执行机器与文件归属机器一致，跨节点安全写回留作后续。

## 入门

1. 浏览 `README.md` 与 `docs/`。
2. 无真实操作演示：`python -m http.server 8899 --directory web`，然后访问 `http://localhost:8899/?demo=1`。
3. 运行契约测试：`npm test`。
4. Rust、Tauri 及集群测试参阅 `scripts/` 与 `docs/06_operations/`。
