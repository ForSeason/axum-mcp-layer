# Plan B — 宏与 DX 优先（B-*）

目标：先把开发者体验打磨到位（强校验、清晰报错、自动 Schema、极简接入），再补齐传输层。适用于需要被大量业务团队复用、强调“零心智负担”的场景。

## 前置约束
- 同 Plan A 依赖基线；宏 crate `axum-mcp-macros` 单独发布，尽量减少编译时间与依赖面。

## 步骤

B-1 基础骨架与注册表（支撑宏落地）
- B-1.1：实现最小 `ToolDescriptor/ToolHandler/ToolRegistry` 与 `gather_with_state`。
- B-1.2：提供 `axum-mcp::prelude` 导出：宏使用者只需少量导入即可工作。
- 验收：无传输，仅能枚举与调用注册表中的工具（单测）。

B-2 属性宏（完整版体验）
- B-2.1：签名解析：支持 `State<S>`、`Extension<_>`、`Json<T>`（唯一）及可选 `Path<_>/Query<_>` 排除出工具参数。
- B-2.2：生成桥接代码：共享一份业务逻辑，HTTP 与 MCP 工具输出复用同一返回类型。
- B-2.3：Schema 生成：`schemars::schema_for!(T/O)`；支持 `#[mcp(rename = ...)]`、`#[mcp(skip)]` 的字段级微调（尽量复用 schemars 属性）。
- B-2.4：编译期诊断：
  - 唯一工具名校验（全局重复时报错，含定义位置）。
  - 缺少 `Json<T>`/多 `Json<_>`/输出类型缺少 `JsonSchema` 等报错信息清晰、可定位。
  - `state = "TypePath"` 与 `State<S>` 不一致时报错与修复建议。
- 验收：对常见误用给出期望的编译期错误；`examples/demo` 展示 2–3 个典型签名。

B-3 结构化内容与文档导出
- B-3.1：可选开启 structuredContent（对齐 2025-06-18）；在工具元数据中暴露能力位。
- B-3.2：导出工具目录（JSON）用于文档站/自动化校验：`cargo xtask export-tools`（或库函数）。
- 验收：生成的 JSON 与 `tools/list` 一致；structuredContent 示例能被客户端识别（本地伪造）。

B-4 传输层（基础版）
- B-4.1：HTTP：`McpLayer` + POST `/mcp`；版本头严格校验；SSE 暂缓。
- B-4.2：STDIO：包装 `mcp-protocol-sdk::server::run_stdio` 与 `ToolRegistry` 的桥接。
- 验收：HTTP/STDIO 至少一种可用；建议先打通 HTTP。

B-5 示例、脚手架与文档
- B-5.1：`examples/demo` 覆盖 sum、带 `State<S>` 的用例、错误场景演示。
- B-5.2：文档强化“宏报错排查”与“最佳实践”（命名、版本化、Schema 变更守则）。
- 验收：新同学 10 分钟内可上手添加一个工具并跑通。

B-6 增强与发布
- B-6.1：SSE（GET `/mcp`）、错误分级与指标。
- B-6.2：预发布到 crates.io（宏与核心库分开发布）。
- 验收：SemVer 约束与最小示例在稳定 Rust 上可编译运行。
