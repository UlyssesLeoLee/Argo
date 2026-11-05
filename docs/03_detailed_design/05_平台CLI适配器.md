# 适配器详细设计（七平台）

所有本机 CLI 均通过 Rust `tokio::process::Command` 独立参数数组执行，不拼接 shell。实际执行 Node 必须安装并认证对应软件，二进制支持参数请以 `--help` 和对应官方文档为准。

| 平台 | 表单适配 | 当前 Runner | 状态语义 |
|---|---|---|---|
| Claude Code | Task / Model | `claude -p ... --output-format stream-json` | 子进程结束 |
| Codex | Task / Model | `codex exec --json --sandbox workspace-write ...` | 子进程结束 |
| Hermes | Task / Skills 可选 | `hermes chat --query-file prompt.txt --quiet` | 子进程结束 |
| Orca | Orca Repo ID / Agent | `orca worktree create --repo ... --agent ... --prompt ... --json` | dispatched，非最终完成 |
| MiniMax | Task / MiniMax Model | `hermes chat --provider minimax --query-file prompt.txt --quiet` | Hermes 工具代理结束；不是 MiniMax 官方 CLI |
| Multica | Workspace / Project / Issue / Agent | Multica `issue create/comment/assign` | dispatched，非最终完成 |
| STAR | Project / Ticket (新建/已有) | 仅已配置的 HTTPS Webhook | dispatched，未推出平台占位 |

对于版本变动仍需适配测试，特别是 Orca/Multica 和不同 Hermes 发行版；不能仅有程序名就断言功能兼容。Worker 通过 `PATH` 检测本机可用工具并注册，STAR 只有配置可信 HTTPS 端点和 Token 才注册。

官方文档来源（2026-11-05 设计时核对）：
- https://learn.chatgpt.com/docs/non-interactive-mode
- https://hermes-agent.nousresearch.com/docs/user-guide/cli
- https://multica.ai/docs/cli
- https://multica.ai/docs/daemon-runtimes
