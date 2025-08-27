# Plan C — 本地/嵌入式（STDIO）优先（C-*）

目标：优先交付 STDIO 传输以服务本地/嵌入式 & IDE/Agent 直连场景；HTTP 作为后续增强。强调零网络暴露与最小依赖。

## 前置约束
- 依赖基线同 Plan A；重点复用 `mcp-protocol-sdk` 的 STDIO 服务实现。

## 步骤

C-1 工作区与依赖
- C-1.1：初始化 workspace 与依赖（`stdio` feature 默认开，`http` 关闭）。
- C-1.2：在核心库中为 `stdio` 提供独立模块与 feature gate。
- 验收：`cargo check --features stdio` 通过。

C-2 工具抽象与注册表
- C-2.1：与 Plan A 的 A-2 保持一致；确保 `ToolCtx.app_state` 能注入任意 `Arc<Any + Send + Sync>`。
- C-2.2：为 STDIO 会话准备上下文提取与跟踪（会话 ID、用户代理等元信息）。
- 验收：单测覆盖 `call` 分发与错误映射。

C-3 STDIO 运行器
- C-3.1：实现 `axum_mcp::stdio::run_stdio(registry, state)`，桥接到 `mcp_protocol_sdk::server::run_stdio`。
- C-3.2：将工具目录、调用结果映射到协议期望的类型；保留 structuredContent 能力位。
- C-3.3：提供关闭钩子与优雅退出（Ctrl-C、会话结束）。
- 验收：本地回环测试：输入 `tools/list`、`tools/call(sum)`，输出正确。

C-4 示例与文档
- C-4.1：`examples/demo`：`#[mcp_tool] sum`；`main` 中仅启动 STDIO。
- C-4.2：README：何时选 STDIO、与 HTTP 的差异、在 IDE/Agent 中配置的方法。
- 验收：`cargo run -p examples/demo` 后可被客户端通过 STDIO 识别并调用。

C-5 安全与限制说明
- C-5.1：强调 STDIO 模式不经网络暴露；权限边界依赖宿主进程。
- C-5.2：提供可插拔的授权回调（可选），默认信任本地宿主。
- 验收：文档阐明边界与风控建议。

C-6 后续：HTTP 与 SSE（可选）
- C-6.1：引入 `http` feature，最小实现 POST `/mcp`；版本头校验。
- C-6.2：按需增加 GET `/mcp`（SSE）。
- 验收：HTTP 冒烟测试通过即可。
