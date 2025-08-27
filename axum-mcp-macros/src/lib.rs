use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, FnArg, Type, ReturnType};
use syn::punctuated::Punctuated;
use syn::{Meta, Token, Expr, ExprLit, Lit, Path};
use syn::parse::Parser;

#[proc_macro_attribute]
pub fn mcp_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Parse attributes: name = "...", desc = "...", state = "TypePath"
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse(attr.into()).unwrap_or_default();
    let mut tool_name: Option<String> = None;
    let mut tool_desc: Option<String> = None;
    let mut state_ty: Option<Path> = None;
    let mut structured: Option<bool> = None;
    for m in metas {
        match m {
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = nv.value { tool_name = Some(s.value()); }
            }
            Meta::NameValue(nv) if nv.path.is_ident("desc") => {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = nv.value { tool_desc = Some(s.value()); }
            }
            Meta::NameValue(nv) if nv.path.is_ident("state") => {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = nv.value { state_ty = Some(syn::parse_str(&s.value()).expect("invalid state type path")); }
            }
            Meta::NameValue(nv) if nv.path.is_ident("structured") => {
                if let Expr::Lit(ExprLit { lit: Lit::Bool(b), .. }) = nv.value { structured = Some(b.value); }
            }
            _ => {}
        }
    }

    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;

    // Parse params: exactly one Json<T>, and a State<S> param
    let mut json_ty: Option<Type> = None;
    let mut state_param_ty: Option<Type> = None;
    for p in &input_fn.sig.inputs {
        if let FnArg::Typed(pt) = p {
            if let Type::Path(tp) = &*pt.ty {
                let segs = &tp.path.segments;
                if segs.iter().any(|s| s.ident == "Json") {
                    if let syn::PathArguments::AngleBracketed(ab) = &segs.last().unwrap().arguments {
                        if let Some(syn::GenericArgument::Type(t)) = ab.args.first() { json_ty = Some(t.clone()); }
                    }
                }
                if segs.iter().any(|s| s.ident == "State") {
                    if let syn::PathArguments::AngleBracketed(ab) = &segs.last().unwrap().arguments {
                        if let Some(syn::GenericArgument::Type(t)) = ab.args.first() { state_param_ty = Some(t.clone()); }
                    }
                }
                // Allow presence of Path<_>, Query<_>, Extension<_> in HTTP usage, but they are not supported in MCP bridge.
            }
        }
    }
    // Validate state type path if provided via attribute against the parsed State<S>
    if let (Some(attr_state), Some(sig_state_opt)) = (state_ty.as_ref(), state_param_ty.as_ref()) {
        let a = quote!(#attr_state).to_string();
        let b = quote!(#sig_state_opt).to_string();
        if a != b {
            return syn::Error::new_spanned(&input_fn.sig.ident, format!("#[mcp_tool(state = \"{}\")] does not match State<{}> parameter", a, b)).to_compile_error().into();
        }
    }
    let json_ty = json_ty.expect("#[mcp_tool] requires exactly one axum::Json<T> parameter");
    let state_param_ty = state_param_ty.expect("#[mcp_tool] requires a State<S> parameter to downcast app_state");

    // Parse return type: Json<O> or O
    let (output_ty, _wrap_is_json) = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => {
            if let Type::Path(tp) = &**ty {
                let segs = &tp.path.segments;
                if segs.iter().any(|s| s.ident == "Json") {
                    if let syn::PathArguments::AngleBracketed(ab) = &segs.last().unwrap().arguments {
                        if let Some(syn::GenericArgument::Type(t)) = ab.args.first() { (t.clone(), true) } else { panic!("unable to infer Json<Out> type") }
                    } else { panic!("expected Json<Out>") }
                } else {
                    ((**ty).clone(), false)
                }
            } else {
                panic!("return type must be concrete (Json<Out> or Out)")
            }
        }
        _ => panic!("return type must be specified (Json<Out> or Out)"),
    };

    let tool_name_lit = tool_name.unwrap_or_else(|| fn_name.to_string());
    let structured_flag = structured.unwrap_or(false);
    let tool_desc_tokens = if let Some(d) = tool_desc { quote!{ Some(#d) } } else { quote!{ None } };
    let handler_ident = format_ident!("{}__mcp_tool_handler", fn_name.to_string());
    let reg_ident = format_ident!("{}_MCP_TOOL", fn_name.to_string().to_uppercase());
    // Link-time duplicate detection symbol removed to avoid unsafe attributes in expansion.

    let expanded = quote! {
        #vis #input_fn

        struct #handler_ident;
        #[async_trait::async_trait]
        impl axum_mcp::tool::ToolHandler for #handler_ident {
            async fn call(&self, ctx: &axum_mcp::tool::ToolCtx, args: serde_json::Value) -> Result<serde_json::Value, axum_mcp::tool::ToolError> {
                use axum::extract::State;
                let sref = ctx.app_state.as_ref().downcast_ref::<#state_param_ty>()
                    .ok_or_else(|| axum_mcp::tool::ToolError::Internal("invalid state type".into()))?;
                let state_val: #state_param_ty = sref.clone();
                let input: #json_ty = serde_json::from_value(args)
                    .map_err(|e| axum_mcp::tool::ToolError::InvalidArgs(e.to_string()))?;
                let out = #fn_name(State(state_val), axum::Json(input)).await;
                Ok(axum_mcp::IntoJsonValue::into_json_value(out))
            }
        }

        #[linkme::distributed_slice(axum_mcp::registry::TOOLS)]
        pub static #reg_ident: axum_mcp::registry::ToolRegistration = axum_mcp::registry::ToolRegistration {
            name: #tool_name_lit,
            description: #tool_desc_tokens,
            input_schema: || schemars::schema_for!(#json_ty),
            output_schema: || schemars::schema_for!(#output_ty),
            build_handler: || std::sync::Arc::new(#handler_ident),
            defined_at_file: file!(),
            defined_at_line: line!(),
            structured: #structured_flag,
        };

        // Duplicate detection is handled at registry-gather time with clear diagnostics.
    };

    TokenStream::from(expanded)
}
