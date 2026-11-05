# Argo 0.3.0 — 多机 Agent Runtime + 跨平台个人记忆

独立项目：**Rust Controller / Worker + Tauri 2 桌面客户端 + 通用 RAG 连接器 + 用户自带 AI Key 的知识预处理**。与 Star 通过可配置 API 集成，不侵入 Star 源仓库。MIT 许可（第三方 CLI 各自许可单独核查）。

## 0.3 功能边界

| 模块 | 当前交付 | 限制 |
|---|---|---|
| 桌面 UI | 七平台单 Agent 选项卡、专属输入、机器/路径隶属展示、执行节点独立选择 | UI 已作 JS 静态检查与契约测试；Tauri 真机待验证 |
| 机器集群 | Rust Worker 主动注册、任务领取、租约、心跳、结果、Git patch 与 SHA256 附件 | 写仓库的执行机器必须为文件归属机器；尚无跨节点安全写回 |
| CLI | Claude、Codex、Hermes、Orca、Multica；MiniMax 经 Hermes MiniMax provider | 第三方实际 CLI 版本和认证需现场 IT；Orca/Multica 派发不等于完成 |
| STAR | 第七平台、问号占位 Logo、项目/任务卡、带 Token 的 HTTPS Webhook 适配器 | STAR 尚未推出；须用户自行实现约定回执 API |
| 本地 RAG | SQLite 记忆分段、去重、来源追踪、审核、链接、归档、检索、版本回滚 | 当前检索为关键词匹配；独立向量索引由外部 RAG 提供 |
| RAG 连接 | 支持多个任意 HTTPS JSON 兼容连接器，通过环境变量管理服务密钥 | 必须适配对方 API 的 JSON 契约；不自动猜测不同服务结构 |
| AI 预处理 | 使用请求临时提供的个人 API Key；标签、摘要、结构提取、改写；先预览后按批提交与审计恢复 | 依赖 OpenAI 兼容 chat/completions；不保证模型输出正确，需要人工审核 |
| 全库批处理 | 查询全库 ID，再每 8 条预览、按确认规则处理，支持中止与进度 | 客户端窗口保持打开；尚非服务器后台持久作业，失败时需手动续跑 |

## 快速预览（无真实外部操作）

```powershell
cd Argo
python -m http.server 8899 --directory web
# 浏览器打开 http://127.0.0.1:8899/?demo=1
```

默认进入 Multica，顶端另有 STAR `?` 选项卡；左侧切到个人 RAG 后展开「AI 智能整理」演示流程。**?demo=1 不会调用 CLI、AI 或上传 RAG**。

## Rust Controller / Worker

安装 Rust stable、Git 和所选 Agent CLI。示例变量见 `examples/.env.example`，**请先设置不同的强管理员和 Worker Token**。

```powershell
cargo run -p argo-runtime -- server
# 在本机或其他受信计算机上启动 Worker（使用同一 ARGO_WORKER_TOKEN）
cargo run -p argo-runtime -- worker
```

服务端默认 `127.0.0.1:8788`，采用 Bearer 身份验证。不同电脑联网使用 TLS 反向代理或者 VPN，不要将 HTTP 控制端口直接暴露公网。`src-tauri/` 需要安装 Tauri CLI、Rust 及相应桌面构建依赖。

## AI 预处理

1. 在个人 RAG 页选择片段、筛选结果或全库所有记忆；选择标签、摘要、结构提取或按统一要求改写。
2. 输入 AI 服务、模型、**个人 API Key**、统一规则；API Key 不保存在数据库和浏览器存储中，仅本次请求传给本机/受信控制面并由控制面转发至模型服务。
3. 点击「预览 AI 修改」。最多预览第一批 8 条；应用时明确确认所选全部数量，后续每批采用同一规则。用户可中止后续批次。
4. 成功写入本地版本快照，**状态重置为待审核**；重新审核通过后才能同步至选择的外部知识库。
5. 片段详情的「版本历史」可恢复之前的内容（恢复后同样待审核）。

提醒：**模型看到的知识片段是来自用户的待处理数据**。当前实施脱敏正则，但不能覆盖一切个人或企业秘密；有机密的知识不要发送不受信模型。不存在跨平台自动收集全部其他产品历史聊天的权限，采集需通过本项目明确授权或导入。

## 通用 RAG 连接

服务端变量示例（可连接自己部署在 Vercel、Cloudflare、家庭 NAS、其他云的 RAG，只要对接相同 JSON 接口）：

```dotenv
ARGO_RAG_CONNECTORS_JSON=[{"id":"personal","name":"个人知识库","upsert_url":"https://rag.example.com/api/upsert","search_url":"https://rag.example.com/api/search","token_env":"ARGO_RAG_PERSONAL_TOKEN"}]
ARGO_RAG_PERSONAL_TOKEN=your-long-secret
```

旧配置 `ARGO_RAG_UPSERT_URL`、`ARGO_RAG_SEARCH_URL`、`ARGO_RAG_API_TOKEN` 仍被支持作为默认连接器。前端不展示任何部署供应商标识。

服务端 `POST /api/v1/rag/sync` 带 `connector_id` 选择目标。参考网关示例仍放在 `examples/vercel-rag-gateway/`，它只是**众多可选部署方式之一**。

## STAR 平台（占位适配）

该平台尚未公开。前端提供 `?` 临时标志，项目和任务卡输入；Worker 只有在配置下述环境变量并成功注册后才执行派发：

```dotenv
ARGO_STAR_DISPATCH_URL=https://star.example.com/api/argo/dispatch
ARGO_STAR_SERVICE_TOKEN=your-separate-secret
```

发送 JSON `idempotency_key`、`source`、`task_id`、`title`、`prompt`、`project_id`、`ticket_id`、`mode`；返回 JSON 至少含 `id`。状态只标记为 `dispatched`，不能据此认定 Star 后续任务已完成。

## 项目目录

- `crates/argo/`：Rust Controller、Worker、REST/Webhook/CLI 适配、RAG 和 AI 预处理。
- `src-tauri/`：Tauri 2 桌面壳与请求桥。
- `web/`：无构建工具前端，可直接预览；七 Agent Tab、个人 RAG、AI 批处理。
- `design/`：可编辑 UI SVG、Argo Logo、STAR 问号占位 Logo 及 PNG 预览。
- `docs/`：要件定义、基本设计、详细设计、UT/IT/ST/E2E、运维、风险与 WBS。
- `examples/`：通用连接器配置、STAR Webhook 契约、可选的 Next.js RAG 网关。
- `scripts/`、`tests/`：测试、预检、辅助运维脚本。
- `requirements_and_tests.xlsx`：完整需求、测试、WBS 与接口追踪。

## 验证

```powershell
npm test
./scripts/run-tests.ps1
```

当前交付环境没有 Cargo/Rustc，无法声称 Rust/Tauri 已编译通过；真实模型 API、STAR 服务和双机执行也未联调。交付的 E2E、IT/ST 文档提供了实际验收步骤。生产部署前必须完成权限隔离、TLS、节点身份、Secret 管理、配置备份和审核。
