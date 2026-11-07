# Star 服务端接入 Argo 的示例

1. 将 `argo-client.ts` 放到 Star 的 `src/server/integrations/argo-client.ts` 或同等位置。
2. 只在 Star 的 **Server Action / Route Handler** 中调用本模块；前端不得直接获取 Argo Admin Token。
3. Star 环境变量：`ARGO_SERVER_URL=https://argo.example.com` 与 `ARGO_ADMIN_TOKEN=<独立密钥>`（不可使用 `NEXT_PUBLIC_` 前缀）。
4. Star 接受提交后将 `{starTicketId, argoTaskId, projectId, requesterId}` 存入独立关联表，方便页面展示对应状态与回溯。
5. 在 Star 自身执行身份登录、角色权限与租户隔离检查。当前 Argo MVP 使用全局 Admin Token，无法直接代替 Star 的项目级授权。
6. 用户上传附件先传 Star 后端并完成类型/体积检查；后端再将二进制上传 Argo `POST /api/v1/blobs`，随后在 `submitArgoTask(...).attachments` 中传入 SHA-256 引用。
7. 为避免无关 Star 组件每次刷新任务，建议通过 Server Action 聚合、少量轮询事件；后续生产环境可使用队列化 Webhook 与回调签名。

## 映射示例

```typescript
import {submitArgoTask} from '@/server/integrations/argo-client';

// 前提：当前会话拥有 ticket 所属项目的 execute-agent 权限。
const queued = await submitArgoTask({
  title: `Star #${ticket.id} ${ticket.title}`,
  prompt: ticket.description,
  provider: 'codex',
  target_node: null,
  repo_alias: 'star',
  context: {star_ticket_id: ticket.id},
});
// 将 queued.id 与 ticket.id 的映射保存到 Star 数据库。
```

## 状态语义

- Argo `completed`：本地 CLI 进程已成功结束；仍需代码审核和测试。
- Argo `dispatched`：Orca/Multica 已接受派发，不是 Agent 完成；Star 必须展示“已派发”。
- Argo `needs_review`：租约超时，人工核对副作用，不自动重试创建任务卡等操作。
