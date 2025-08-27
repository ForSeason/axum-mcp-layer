# Plan A — HTTP MVP 优先（A-*)

目标：最短路径交付一个可在 Axum 上挂载的 MCP Streamable HTTP 端点（默认 `/mcp`），完成工具注册、列举与调用闭环；属性宏达“可用”粒度即可。默认仅监听 `127.0.0.1`、强制校验协议版本头；SSE 与 STDIO 后续里程碑补齐。

## 范围与非目标
- 范围：`POST /mcp` 请求—响应模式（初始化、`tools/list`、`tools/call`），最小错误模型与本地安全基线。
- 非目标：跨进程鉴权、远程部署、复杂会话管理、持久化与配额，均留待后续。

## 前置约束
- Rust 1.80+；`tokio` 1.47，`axum` 0.8.4+，`tower` 0.5，`tower-http` 0.6。
- 采用 `linkme` 收集由属性宏生成的静态注册项；Schema 通过 `schemars` 生成。

## 交付物
- 可用的 `axum_mcp::McpLayer` 与 `McpLayerConfig`。
- `ToolDescriptor/ToolHandler/ToolRegistry` 最小实现与单测。
- `axum-mcp-macros` 提供 `#[mcp_tool]` 可用版。
- `examples/demo` 可被客户端通过 HTTP 识别与调用。

## API 合同（MVP）
- 头部：客户端必须携带 `MCP-Protocol-Version: 2025-06-18`；否则 `400 Bad Request` 或 `426 Upgrade Required`（实现选一，默认 400）。
- 路由：`POST {path}`（默认 `/mcp`）。
- 请求（示例）：
  ```json
  { "op": "tools/list" }
  ```
  ```json
  { "op": "tools/call", "name": "sum", "args": { "a": 1, "b": 2 } }
  ```
- 响应（示例）：
  ```json
  { "tools": [{"name":"sum","description":"Add two integers","input_schema":{...},"output_schema":{...}}] }
  ```
  ```json
  { "ok": true, "result": { "sum": 3 } }
  ```
- 错误：`{ "ok": false, "code": "invalid_args" | "tool_not_found" | "internal", "message": "..." }`。

## 实施步骤

A-1 工作区初始化与依赖
- A-1.1：创建 workspace：`axum-mcp/`、`axum-mcp-macros/`、`examples/demo/`。
- A-1.2：添加依赖：axum、tokio、tower、tower-http、serde、serde_json、schemars、linkme、thiserror、mcp-protocol-sdk（类型复用即可）。
- A-1.3：设置 features：`http`（默认开）、`stdio`（后续用）。
- 验收：`cargo check -p axum-mcp -p axum-mcp-macros` 通过。

A-2 工具抽象与注册表（最小实现）
- A-2.1：定义 `ToolDescriptor`、`ToolHandler` trait、`ToolCtx`、`ToolError`。
- A-2.2：实现 `ToolRegistry`（线程安全 Map），暴露：`insert`、`get`、`call(name, args, ctx)`、`gather_with_state(state)`。
- A-2.3：序列化 `tools/list` 数据：`name/description/input_schema/output_schema`。
- 验收：`insert/call` 单测；`gather_with_state` 能收集 0/1/多工具。

A-3 属性宏（可用版）
- A-3.1：`#[mcp_tool(name=..., desc=..., state="TypePath")]` 解析；限定恰有一个 `Json<T>` 参数；可选 `State<S>/Extension<_>`。
- A-3.2：生成 `ToolDescriptor`、`ToolHandler` 实现与 `linkme` 注册项。
- A-3.3：用 `schemars` 生成输入/输出 JSON Schema；输出类型需 `Serialize + JsonSchema`。
- A-3.4：错误覆盖：缺少/多余 `Json<T>`、未实现 `JsonSchema`、重复工具名。
- 验收：`examples/demo::sum` 成功注册；`tools/list` 含正确 Schema。

A-4 HTTP Layer + Service（MVP）
- A-4.1：`McpLayer` 与 `McpLayerConfig { path:"/mcp", require_version:true, allowed_origins:Localhost, enable_sse:false }`。
- A-4.2：`McpService`：
  - `POST {path}`：处理 `tools/list`、`tools/call`（从 `ToolRegistry` 分发）。
  - 校验 `MCP-Protocol-Version`；缺失/不匹配→`400/426`（默认 400）。
- A-4.3：安全基线：仅监听 `127.0.0.1`；校验 `Origin` 为 `null` 或 `http://127.0.0.1:*`；开发态 CORS 放行本地端口。
- A-4.4：集成测试：`/mcp` 上的 `tools/list`、`tools/call(sum)`；版本头缺失/错误用例。
- 验收：`cargo test -p axum-mcp` 通过；`examples/demo` 可被 `claude mcp add --transport http http://127.0.0.1:37650/mcp` 识别。

A-5 示例与文档（首版）
- A-5.1：`examples/demo`: `#[mcp_tool] sum`；HTTP 正常返回与 MCP 工具输出共用数据结构。
- A-5.2：README：安装、宏用法、挂载 Layer、版本头与本地绑定说明。
- 验收：跟随 README 步骤≤3 分钟跑通。

A-6 健壮性与可观测性
- A-6.1：错误映射：参数校验/工具错误→统一 `{ ok:false, code, message }`。
- A-6.2：可观测性：集成 `tower::trace`；计数/耗时指标挂钩（留接口）。
- 验收：`RUST_LOG=info` 可见关键日志；失败请求含稳定错误码。

A-7 增强（下一里程碑，可选）
- A-7.1：开启 `enable_sse` 并实现 `GET {path}` 推送（SSE）。
- A-7.2：协议回退策略：可配置缺失版本头时回退到 `2025-03-26`；默认仍拒绝。
- A-7.3：鉴权挂钩（Bearer/OAuth 回调），默认关闭。
- 验收：SSE 冒烟；回退策略开关可控。

## 目录与代码骨架（建议）
```
axum-mcp/
  src/
    lib.rs
    layer.rs        // McpLayer/McpService
    registry.rs     // ToolRegistry
    tool.rs         // ToolDescriptor/ToolHandler/ToolCtx
    http.rs         // 请求路由与错误映射
    security.rs     // 版本头/Origin 校验
    prelude.rs
axum-mcp-macros/
  src/lib.rs        // #[mcp_tool]
examples/demo/
  src/main.rs
```

## 测试矩阵（MVP 必测）
- 列举工具：空目录/单工具/多工具，Schema 基本字段存在。
- 调用成功：`sum(1,2)=3`，返回体 `{ ok:true, result }`。
- 调用失败：未知工具、参数缺失/类型错误、业务错误（映射为 `invalid_args`/`internal`）。
- 安全：无版本头/错误版本头；`Origin` 非本地；CORS 仅本地放行。
- 稳定性：并发 10 请求调用 `sum` 结果正确、无共享状态崩溃。

## 清单（勾选式）
- [ ] `Tool*` 与 `Registry` 最小实现与单测
- [ ] `#[mcp_tool]` 宏可用版与演示
- [ ] `McpLayer`/`McpService` 路由与头校验
- [ ] 错误模型与日志/trace
- [ ] `examples/demo` 与 README
- [ ] 集成测试与本地冒烟脚本
