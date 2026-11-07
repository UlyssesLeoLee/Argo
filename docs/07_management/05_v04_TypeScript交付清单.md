# v0.4.0 TypeScript 迁移与交付状态

## 产品状态

- [x] 七平台统一入口：Claude / Codex / Hermes / Orca / MiniMax / Multica / STAR
- [x] 文件所属机器、仓库、路径组装为一个上下文；执行节点独立选择
- [x] 个人 RAG 页面和通用连接器设置（不写死任何云提供商）
- [x] AI Key 驱动的预处理、全库分批预览、人工确认和版本恢复基础流程
- [x] Rust Controller / Worker / Tauri 2 的源代码及 Sidecar 生命周期管理
- [x] TypeScript 的前端源码与编译产物，核心契约 strict 检查
- [x] 个人 RAG / 执行任务 / 批量 AI 处理 SVG + Logo
- [x] 设计、需求追踪 Excel、UT/IT/ST/E2E 测试设计与命令脚本
- [ ] 目标 Windows 机器 Rust 编译、Tauri 安装包生成
- [ ] 七平台 CLI 的实际安装/授权和输入兼容性联调
- [ ] 双机文件同步 E2E 和网络故障注入
- [ ] Windows 密钥加密与安全审计

## 验证证据与方法

```text
npm run typecheck                     PASS
npm test                              80 passed / 0 failed
python tests/smoke_snapshot_patch.py  PASS
cargo check -p argo-desktop           NOT RUN (toolchain unavailable)
cargo test -p argo-runtime            NOT RUN (toolchain unavailable)
python tests/integration_cluster.py   NOT RUN (depends on built runtime)
```

保留原有 01～07 阶段文档，并增加 v0.4 TypeScript 专项详细设计及测试设计。正式上线前，应在目标 Windows 机器完成 Rust 依赖解析、编译修正、安装包签名、性能与安全验证。
