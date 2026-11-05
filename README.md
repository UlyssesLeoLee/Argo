# Argo — 独立多节点 AI Agent 统一控制台

> 版本 0.1.0 · 2026-11-05 · 首版可审查源码 + 网页控制台 + Rust 控制面/Worker + 测试 + IPA 风格设计资料。

Argo 不修改 Star 仓库；作为独立项目运行，可通过 REST API 由 Star 服务端接入。统一界面选择 Claude Code、Codex、Hermes、Orca、MiniMax Code、Multica，自动将用户输入转换为各自的 CLI 调用，并在授权的 Windows/Linux/macOS Worker 节点执行。

## 交付状态与真实边界

- ✅ 前端：可响应式统一输入、平台上下文表单、Orca/Multica 元数据发现、文件拖放、节点与任务列表、日志、取消、结果文件下载、深浅主题、Token 会话、无凭据演示模式。
- ✅ Rust **源码实现**：控制面 REST、SQLite 队列、节点注册与心跳、CAS 领取、租约、Worker 并行槽、进程执行和 stdout/stderr 事件、附件与 SHA-256 Blob、Git 工作树与差异回传、输出文件回传。
- ✅ 六个平台的命令适配器源码与参数校验；Orca/Multica 的 `dispatched` 代表平台接受派发，不代表远端 Agent 已完成。
- ✅ 已执行：Node.js 前端 10 项单体/契约测试。`tests/integration_cluster.py` 可在 Rust 可用时自动运行两 Worker 的不收费端到端模拟测试。
- ⚠️ 当前制作环境**没有 Cargo / rustc**，因此 Rust 二进制未在此环境编译，Rust 单测和真实跨机 E2E 未执行；请在有 Rust 工具链的环境执行 `cargo test` 和集成脚本。不得将源码存在误称为经过生产验证。
- ⚠️ 第三方 CLI 是否登录、安装、版本匹配需在用户节点验证；没有接入任何真实账号，也没有调用付费模型。
- ⚠️ 首版身份认证是 Admin/Worker 双 Token，不包含生产级多租户 RBAC、OS 容器/VM 级强沙箱、自动模型分片和透明显存共享。Git Worktree 只隔离 Git 工作目录，不能当作安全沙箱。

## 目录

```text
Argo/
├── crates/argo/src/           # Rust server / worker / model adapters
├── web/                       # 独立可运行 Web UI（无需构建依赖）
├── docs/                      # IPA 阶段式设计文档、UT/IT/ST/E2E
├── tests/                     # Node 纯函数测试、Python 双 Worker E2E
├── examples/                  # OpenAPI、Star 对接示例、环境变量
├── scripts/run-tests.sh       # 单键质量检查
├── requirements_and_tests.xlsx
└── Cargo.toml
```

## 快速预览（只测试 Web 页面，不使用模型）

在 Argo 根目录运行：

```powershell
python -m http.server 8899 --directory web
# 浏览器打开 http://127.0.0.1:8899/?demo=1
```

演示模式内所有任务和模型输出都明确标识为模拟数据，不会执行真实 CLI。

## Windows：启动真实本地控制面

安装 Rust stable（建议 rustup 安装）和 Git 后，打开 PowerShell：

```powershell
cd D:\source\Argo
$env:ARGO_ADMIN_TOKEN = 'CHANGE-TO-STRONG-RANDOM-ADMIN-TOKEN-32'
$env:ARGO_WORKER_TOKEN = 'CHANGE-TO-STRONG-RANDOM-WORKER-TOKEN-32'
$env:ARGO_BIND = '127.0.0.1:8788'
$env:ARGO_DATA_DIR = '.\data'
$env:ARGO_WEB_DIR = '.\web'
cargo run -p argo-runtime -- server
```

新 PowerShell 会话，使用相同 Worker Token（不要混用 Admin）：

```powershell
cd D:\source\Argo
$env:ARGO_WORKER_TOKEN = 'CHANGE-TO-STRONG-RANDOM-WORKER-TOKEN-32'
$env:ARGO_SERVER = 'http://127.0.0.1:8788'
$env:ARGO_NODE_ID = 'win-main'
$env:ARGO_MAX_PARALLEL = '2'
$env:ARGO_REPOS = 'star=D:\source\Star;ide=D:\source\IDE1.0'
cargo run -p argo-runtime -- worker
```

打开 `http://127.0.0.1:8788/` →「连接设置」→ 输入 Admin Token → 选择 Agent、仓库、目标节点并提交。

## 第二台计算机

第二台电脑安装相同版本 Argo，并安装自己要使用的 AI CLI：

```powershell
$env:ARGO_SERVER = 'https://argo.example.com'     # 指向第一台控制面的 HTTPS 入口
$env:ARGO_WORKER_TOKEN = '与控制面相同的 Worker Token'
$env:ARGO_NODE_ID = 'win-gpu-02'
$env:ARGO_REPOS = 'star=D:\source\Star'
$env:ARGO_MAX_PARALLEL = '3'
cargo run -p argo-runtime -- worker
```

注意：控制面默认只监听 `127.0.0.1`。跨机请通过 Tailscale 或 HTTPS 反向代理保护服务；公网不可直接以无 TLS 的 `0.0.0.0` 暴露 Bearer Token。

## 平台准备

| 选择 | Node 侧必须可用 | 执行语义 |
|---|---|---|
| Claude | `claude`、完成本地认证 | `claude -p ...` 后退出 |
| Codex | `codex`、完成认证 | `codex exec --json ...` |
| Hermes | `hermes`、完成认证 | `hermes chat --query-file ... --oneshot` |
| MiniMax | `mcode`、完成认证 | `mcode exec ...` |
| Orca | 已运行的 Orca、已注册 `orca` CLI | 创建 Worktree 并触发 Agent；状态 `dispatched` |
| Multica | `multica`、Workspace 授权、本地 Daemon | 创建/关联 Issue 并分配；状态 `dispatched` |

MiniMax 和 Multica 的账户配置以及模型权限由它们自己维护，Argo 不储存第三方凭据。

## 验证命令

```bash
node --test tests/ui.contract.test.mjs     # 当前环境已通过
cargo fmt --all -- --check                # 需 Rust 工具链
cargo test --workspace                    # 需 Rust 工具链
python3 tests/integration_cluster.py      # 两个模拟 Worker，不触发模型计费
```

Windows 下如已构建：`python tests/integration_cluster.py .\target\debug\argo-runtime.exe`。

## Star 集成

参照 `docs/05_star_integration/01_Star接入.md` 和 `examples/star-integration/`。Star 只调用 Argo 的任务与查询 API；模型凭据仅保留在 Worker 侧。切勿把 `ARGO_ADMIN_TOKEN` 放入 Next.js 的 `NEXT_PUBLIC_*` 环境变量。

## 许可与核验来源

Argo 代码以 MIT 许可证交付。第三方平台采用各自的许可证，使用者应自行遵守。适配器依据的官方或仓库文档见 `docs/03_detailed_design/02_平台适配器契约.md`。
