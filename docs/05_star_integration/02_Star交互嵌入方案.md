# Star 中的 Argo UI 集成建议

ID：ARGO-INT-002 · 版本：1.0

## 产品定位

Argo 是独立多机执行引擎；Star 是无限画布与任务协调平台。Star 侧提供一个「Argo 运行」面板即可，不要复制 Argo 的所有 Project/Issue 语义。平台选择统一，条件字段由 Argo adapter schema 决定。Star Ticket 卡片上只需要 Argo 任务状态徽标、执行节点和结果链接。

## UI 主流程

- 从 Star 的 Task/Ticket 卡片打开侧栏：自动填入任务标题与描述、仓库别名、Star Ticket ID。
- 中央面板仍使用 Argo 六平台卡片、条件化上下文选择、模型与节点；多任务在右侧列表并行展示。
- `dispatched` 状态明确显示「已交给 Orca/Multica」，不要使用成功完成的绿色图标。
- `completed` 显示 Git patch 审阅入口；`needs_review` 显示执行节点失联、人工确认必要。
- 首版可直接以独立 Argo 页面新标签打开；后续将 Argo Web 移植为可复用 React 组件或 Web Component，再挂载到 Star Canvas Drawer。直接 iframe 到带 Admin Token 的控制面不应作为生产设计。

## 权限与数据关联

Star 接入层负责项目的访问控制，Argo 的全局 Admin Token 仅可存在服务端。Star 应为用户产生最小权限 scoped token（Argo 第二版支持后使用）；下载 Artifact 前再次验证对应 Star Ticket 是否允许该用户访问。Multi-Tenant 需要隔离 Workspace/Node/Repo/Blob；当前 MVP 尚未实现。
