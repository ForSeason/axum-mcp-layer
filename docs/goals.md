下面是可直接交给工程师实现的设计文档（含依赖与版本建议、crate 结构/抽象、示例与落地步骤）。已先检索并核对到 2025‑08‑19（东京时间）相关依赖与规范更新；关键点处附引用。

⸻

0) 目标与选择
	•	以 Axum 中的 Layer 形式挂载一个 MCP 服务器端点（默认 /mcp），不中断现有路由。
	•	通过 属性宏在需要暴露的 handler 上做最小侵入的标注；未标注的 handler 不暴露为 MCP 工具。
与“路由 group 扫描/注册”方案相比，属性宏无需反射 Router 内部结构，注册更直观、编译期可验证，成本更低、也更可控。
	•	同时支持两种传输：
	1.	Streamable HTTP（MCP 2025‑06‑18 规范，单路径、POST/GET，带可选 SSE/流式推送）
	2.	STDIO（面向本地/嵌入式场景）
规范与传输要求参考官方文档与变更说明 ￼ ￼ ￼。
	•	遵循最新安全与版本要求：HTTP 传输需校验 MCP-Protocol-Version 头与来源等 ￼ ￼。

⸻

1) 依赖与版本（建议）

以下为当前稳定/常用版本，已核对到 2025‑08。若公司有统一版本基线，可按需微调次版本。

crate	版本	用途
axum	0.8.4	Web 框架（0.8 系，hyper1 生态） ￼ ￼
tokio	1.47.x	异步运行时 ￼ ￼
tower	0.5.x	中间件抽象（Layer/Service） ￼
tower-http	0.6.x	CORS/Trace/Compression 等中间件 ￼
serde / serde_json	1.0.219 / 1.0	序列化/反序列化 ￼ ￼
schemars	1.x	由类型生成 JSON Schema，用于工具参数/输出说明 ￼ ￼
linkme（或 inventory）	0.3.33 / 0.3.21	分布式注册收集被宏标注的工具 ￼ ￼
thiserror	2.x	错误定义
mcp-protocol-sdk	0.5.1	MCP Rust SDK（2025‑06‑18 版），含 STDIO/HTTP/SSE 传输实现与类型 ￼

说明与依据
	•	Axum 自 0.7 起全面对接 hyper 1.0；0.8 为现行主线（注意 0.8.2 曾被 yanked，选 0.8.4 或更高补丁版） ￼ ￼ ￼。
	•	最新 MCP 规范（2025‑06‑18）明确 Streamable HTTP 与协议版本头要求 ￼ ￼ ￼。
	•	mcp-protocol-sdk 0.5.1 同时声明支持 STDIO 与 HTTP/SSE 传输（并可选用 axum/tower 生态） ￼。

⸻

2) 工作区与 crate 拆分

axum-mcp/
├─ axum-mcp/               # 主库：Layer、注册表、HTTP/STDIO 适配、调度
├─ axum-mcp-macros/        # 宏库：#[mcp_tool] 属性宏、派生宏
└─ examples/
   └─ demo/

axum-mcp（lib）模块划分
	•	layer: McpLayer（Tower Layer），拦截 /mcp 路径并交给内部 MCP 服务处理，其余请求透传。
	•	registry: ToolRegistry（名称→工具描述与调用器），线程安全。
	•	tool: ToolDescriptor、ToolHandler（对象安全 async 调用接口）、ToolIo（输入/输出 Schema）。
	•	http: Streamable HTTP 实现（POST/GET 单路径、可选 SSE），校验 MCP-Protocol-Version、Origin 等安全项 ￼ ￼。
	•	stdio: STDIO runner（复用同一 ToolRegistry），基于 mcp-protocol-sdk。
	•	schema: 对接 schemars 生成 JSON Schema；执行 JSON Schema 校验（可选 jsonschema）。
	•	security: CORS、Origin 校验、绑定 127.0.0.1 的建议默认值（避免 DNS rebinding） ￼。
	•	prelude: 对外常用导出。

axum-mcp-macros（proc-macro）
	•	#[mcp_tool(...)]：标注在 Axum handler 上，生成：
	•	工具元数据（名称、描述、输入/输出 Schema）；
	•	一个注册项（通过 linkme/inventory 分布式收集）；
	•	一个“工具调用桥”把 MCP 调用映射到同一业务实现。

选择 linkme：零运行期开销、编译期收集；跨 crate 汇聚工具项 ￼ ￼。

⸻

3) 关键抽象

// axum-mcp/src/tool.rs
pub struct ToolDescriptor {
  pub name: &'static str,
  pub description: Option<&'static str>,
  pub input_schema: schemars::schema::RootSchema,
  pub output_schema: schemars::schema::RootSchema,
  pub handler: Arc<dyn ToolHandler + Send + Sync>,
}

#[async_trait::async_trait]
pub trait ToolHandler: Send + Sync {
  async fn call(&self, ctx: &ToolCtx, args: serde_json::Value)
      -> Result<serde_json::Value, ToolError>;
}

pub struct ToolCtx {
  // 访问全局状态（Any + downcast），日志、鉴权上下文等
  pub app_state: std::sync::Arc<dyn std::any::Any + Send + Sync>,
  pub req_meta: ReqMeta, // 会话/用户/headers 等
}

// 注册表：启动时收集宏生成的静态注册项
pub struct ToolRegistry { /* HashMap<String, ToolDescriptor> */ }

属性宏行为（简述）
	•	要求 handler 恰好一个 Json<T> 参数作为 MCP 工具的 输入结构；State<S>/Extension<_> 等可选，Path/Query 不参与工具参数。
	•	返回值建议为业务 O: Serialize + JsonSchema（HTTP 仍可 impl IntoResponse），宏会包装成 serde_json::Value 写回 MCP。
	•	宏生成的注册项通过 linkme 汇聚进 ToolRegistry。

为什么选属性宏而非 group 扫描？
Axum Router 不提供稳定的“枚举所有 handler + 反射签名”的 API；用 group 方案需人工重复注册/维护表，或侵入 Router 构建逻辑，工程成本更高且出错概率更大。属性宏直接贴在 意图清晰的业务函数 上，配合 Schema 自动生成，更优雅且低成本。

⸻

4) Layer 与路由整合

// axum-mcp/src/layer.rs
pub struct McpLayerConfig {
  pub path: &'static str,                 // 默认 "/mcp"
  pub require_version: bool,              // 校验 MCP-Protocol-Version
  pub allowed_origins: AllowedOrigins,    // 安全：避免 DNS rebinding
  pub enable_sse: bool,                   // 是否开放 SSE 流推送
}

pub struct McpLayer { /* 持有 ToolRegistry + Config */ }

impl<S> tower::Layer<S> for McpLayer { /* wrap service */ }

// McpService: 匹配 path == config.path 则进入 MCP HTTP 处理：
//  - POST: 处理初始化/调用等 JSON 消息（Streamable HTTP）
//  - GET : 可选 SSE（兼容/增值流式）

Streamable HTTP 要点
	•	单一端点同时支持 POST/GET；POST 为客户端→服务器消息，GET 可用于流推送（兼容 SSE） ￼。
	•	客户端后续请求 MUST 发送 MCP-Protocol-Version: 2025-06-18 头，服务端需据此处理/回退 ￼ ￼。

⸻

5) STDIO 运行器

// axum-mcp/src/stdio.rs
pub async fn run_stdio(registry: Arc<ToolRegistry>, state: Arc<dyn Any + Send + Sync>) -> anyhow::Result<()> {
  // 复用 mcp-protocol-sdk 的 STDIO transport 与会话生命周期
  mcp_protocol_sdk::server::run_stdio(registry.into(), state).await
}

mcp-protocol-sdk 0.5.1 提供 STDIO/HTTP/SSE 等传输实现与完整协议类型，直接复用更稳妥 ￼。

⸻

6) 安全与合规
	•	对 HTTP 传输：
	•	校验 MCP-Protocol-Version 头（缺失时按规范回退/拒绝，默认回退到 2025‑03‑26 仅为兼容旧客户端） ￼ ￼；
	•	校验 Origin、默认仅绑定 127.0.0.1，并建议接入 OAuth/Bearer 等鉴权（规范推荐） ￼ ￼。
	•	可叠加 tower-http 的 CorsLayer、TraceLayer 等 ￼。

⸻

7) 对外 API（示例）

Cargo.toml（应用侧）

[dependencies]
axum = "0.8.4"
tokio = { version = "1.47", features = ["macros","rt-multi-thread"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors","trace"] }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "1", features = ["derive"] }
thiserror = "2"
axum-mcp = { path = "../axum-mcp", features = ["http","stdio"] }
axum-mcp-macros = { path = "../axum-mcp-macros" }

业务 handler（带宏标注）

use axum::{extract::{State, Json}, response::IntoResponse};
use axum_mcp_macros::mcp_tool;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, JsonSchema)]
struct SumIn { a: i64, b: i64 }

#[derive(Serialize, JsonSchema)]
struct SumOut { sum: i64 }

#[mcp_tool(name="sum", desc="Add two integers", state = "AppState")]
pub async fn sum(State(app): State<AppState>, Json(inp): Json<SumIn>) -> impl IntoResponse {
    let s = inp.a + inp.b;
    // HTTP 正常返回
    axum::Json(SumOut { sum: s })
    // 宏会同时生成 MCP 工具桥，直接复用 SumOut 作为工具输出
}

应用启动（同时支持 HTTP + STDIO）

use axum::{Router, routing::post};
use axum_mcp::{McpLayer, McpLayerConfig, ToolRegistry};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {/* ... */}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState { /* ... */ });

    // 构建 Router（原有业务路由保持不变）
    let app = Router::new()
        .route("/sum", post(sum))
        .layer(
            McpLayer::new(
                ToolRegistry::gather_with_state(state.clone()),
                McpLayerConfig {
                    path: "/mcp",
                    require_version: true,
                    allowed_origins: Default::default(),
                    enable_sse: true,
                }
            )
        );

    // 启动 HTTP
    let http = tokio::spawn(async move {
        use axum::http::Method;
        use tower_http::cors::{Any, CorsLayer};

        let app = app.layer(
            CorsLayer::new().allow_methods([Method::GET, Method::POST]).allow_origin(Any)
        );
        axum::serve(
            tokio::net::TcpListener::bind("127.0.0.1:37650").await?,
            app
        ).await?;
        anyhow::Ok(())
    });

    // 同时提供 STDIO（可选）
    let stdio = tokio::spawn(async move {
        axum_mcp::stdio::run_stdio(ToolRegistry::global(), state.clone()).await
            .map_err(Into::into)
    });

    let _ = tokio::try_join!(http, stdio)?;
    Ok(())
}

客户端连接（示例）

# Streamable HTTP
claude mcp add --transport http demo http://127.0.0.1:37650/mcp
# STDIO（按工具或宿主要求）


⸻

8) 宏设计细节（工程要点）
	•	语法：
#[mcp_tool(name="...", desc="...", state="TypePath")]
	•	state：宏在生成的 MCP 工具桥里，从 ToolCtx.app_state 中 downcast_ref 到指定类型，确保与 State<S> 的业务一致。
	•	规则：
	•	只能有一个 Json<T> 参数作为工具输入；T: Deserialize + JsonSchema。
	•	返回类型建议为可序列化业务类型 O: Serialize + JsonSchema（HTTP 仍可 IntoResponse）。
	•	代码生成：
	1.	生成 const 注册项，写入 linkme 分布式切片；
	2.	为工具桥生成 ToolHandler 实现，内部调用同一业务逻辑（避免重复实现）；
	3.	借助 schemars 自动产出输入/输出 Schema 并随工具导出；
	4.	（可选）为工具输出增加 structuredContent 支持（对齐 2025‑06‑18） ￼。

⸻

9) HTTP 传输实现要点
	•	单路径路由：/mcp
	•	POST /mcp：接收 JSON 消息（初始化、tools/list、tools/call 等），分发到注册表；
	•	GET /mcp（可选）：SSE/事件流（向客户端推送通知/部分结果） ￼。
	•	版本头：拒绝缺失或不匹配时的请求；或按规范回退 ￼ ￼。
	•	安全：校验 Origin、默认仅监听 127.0.0.1、建议启用鉴权（Bearer/OAuth） ￼ ￼ ￼。

⸻

10) 实施步骤（给开发）
	1.	初始化仓库：创建 axum-mcp / axum-mcp-macros 工作区。
	2.	实现注册表：ToolRegistry（基于 DashMap/RwLock<HashMap>），提供：
	•	gather_with_state(state)：收集 linkme 静态项并注入共享状态；
	•	查找/调度 call(name, args, ctx)。
	3.	宏落地：
	•	解析 handler 签名（syn），定位 Json<T> 与可选 State<S>；
	•	生成 ToolDescriptor + ToolHandler 实现 + linkme 注册；
	•	生成 Schema（schemars::schema_for!(T) / schema_for!(O)）。
	4.	Layer/Service：
	•	McpLayer 包裹 Service<Request<Body>>，命中 /mcp 交给 McpService；
	•	McpService 解析版本头、路由 POST/GET；对 tools 相关消息分发到 ToolRegistry。
	5.	STDIO：封装 mcp-protocol-sdk STDIO 服务器，桥接至 ToolRegistry ￼。
	6.	安全：提供默认 AllowedOrigins::Localhost、可选 CORS、校验 Origin。
	7.	示例与测试：
	•	examples/demo 覆盖 sum 用例；
	•	HTTP：对 /mcp 的初始化、tools/list、tools/call 进行集成测试；
	•	STDIO：模拟输入/输出回环测试。
	8.	文档：README 说明宏用法、Axum 集成、HTTP/STDIO 同时启用方式与注意事项。

⸻

11) 兼容性与注意
	•	Axum 版本：推荐 0.8.4+（0.8.2 已被 yanked，避免使用） ￼ ￼。
	•	MCP 规范：以 2025‑06‑18 为基线；确保实现 Streamable HTTP 的单端点语义与版本头校验 ￼ ￼ ￼。
	•	旧客户端：如需兼容 2024‑11‑05 的 HTTP+SSE，可同时保留旧端点（可选） ￼。

⸻

12) 参考/依据
	•	Axum 对 hyper1 的支持、升级提示与与 tower-http 协同 ￼ ￼。
	•	Axum 最新发布与版本（0.8.x） ￼ ￼ ￼。
	•	Tokio 最新版 1.47.x ￼ ￼。
	•	Serde 1.0.219 ￼、Schemars v1 发布 ￼。
	•	MCP 2025‑06‑18 规范与传输、安全要求 ￼ ￼ ￼ ￼。
	•	Rust MCP SDK（mcp-protocol-sdk 0.5.1）特性与传输支持 ￼。

⸻

如需，我可以把上述骨架直接扩成最小可运行仓库（含 axum-mcp、axum-mcp-macros 与 examples/demo），按此文档落地即可。
