use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, FnArg, ItemFn, Meta, Pat, ReturnType, Signature, Type};

use crate::common;

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
pub struct ToolArgs {
    #[darling(rename = "anno")]
    pub anno: Option<String>,
    pub example: Option<String>,
    // pub active: Option<syn::Expr>,
    pub timeout: Option<String>,
    #[darling(rename = "limit")]
    pub rate_limit: Option<String>,
    #[darling(rename = "perms")]
    pub permissions: Option<String>,
}

pub fn handle_fn(
    item_fn: &mut ItemFn,
    attr: Attribute,
    generated_tools: &mut Vec<proc_macro2::TokenStream>,
) {
    // 2. 解析属性参数
    let args = ToolArgs::from_meta(&attr.meta).unwrap_or_default();

    // 3. 构建参数提取器 (extractors) 和 函数入参变量名 (arg_names)
    let (arg_extractors, arg_names) = build_arg_extractors(&mut item_fn.sig);

    //限流
    let rate_limit_tokens = parse_rate_limit_to_tokens(args.rate_limit.as_deref().unwrap_or(""));
    //超时
    let timeout_tokens = args
        .timeout
        .as_deref()
        .map(parse_time)
        .map(|v| quote! {Some(#v)})
        .unwrap_or(quote! {None});
    //权限
    let permissions_tokens = {
        if let Some(perms) = &args.permissions {
            let perms_vec: Vec<String> = perms
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            quote! {vec![#(#perms_vec),*].into_iter().collect()}
        } else {
            quote! {std::collections::HashSet::new()}
        }
    };
    // 5. 生成工具注册代码
    let tool_registration = build_tool_registration(
        item_fn,
        args,
        arg_extractors,
        arg_names,
        timeout_tokens,
        rate_limit_tokens,
        permissions_tokens,
    );

    generated_tools.push(tool_registration);
}

fn build_arg_extractors(
    sig: &mut Signature,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let mut extractors = Vec::new();
    let mut names = Vec::new();

    for arg in &mut sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            let ty = pat_type.ty.to_token_stream();
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let arg_ident = &pat_ident.ident;
                let arg_name_str = arg_ident.to_string();
                // 收集传递给原函数的变量名
                names.push(quote!(#arg_ident));
                if is_ctx_arg(&mut pat_type.attrs, false) {
                    extractors.push(quote! {
                        let #arg_ident = ctx;
                    });
                    continue;
                }
                // 生成从 HashMap 中提取并反序列化的代码
                extractors.push(quote! {
                    let #arg_ident:#ty = {
                        let arg_val = args.remove(#arg_name_str).unwrap_or_default();
                        serde_json::from_value(arg_val)
                            .map_err(|e| anyhow::anyhow!("{} 参数格式异常: {}", #arg_name_str, e))?
                    };
                });
            }
        }
    }
    (extractors, names)
}

fn parse_rate_limit_to_tokens(rate_limit: &str) -> proc_macro2::TokenStream {
    if rate_limit.is_empty() {
        return quote!(None);
    }
    // 编译期切分 "10s:5"
    let (time_str, count_str) = rate_limit
        .split_once(':')
        .expect("rate_limit 格式必须为 '时间:次数' (例如 10s:5)");
    let times: u32 = count_str.parse().expect("限流次数解析失败");
    let dur_tokens = parse_time(time_str);
    quote!(Some((#dur_tokens, #times)))
}
//解析时间code，转换成Duration
fn parse_time(time_str: &str) -> proc_macro2::TokenStream {
    // 分离数字和单位，如 "10" 和 "s"
    let unit_idx = time_str
        .find(|c: char| c.is_alphabetic())
        .expect("限流时间单位缺失");
    let (num_str, unit) = time_str.split_at(unit_idx);
    let num: u64 = num_str.parse().expect("限流时间数值解析失败");

    // 注意：标准库 Duration 没有 from_mins 和 from_hours，需要转换为 secs
    let dur_tokens = match unit {
        "ms" => quote!(std::time::Duration::from_millis(#num)),
        "s" => quote!(std::time::Duration::from_secs(#num)),
        "m" => quote!(std::time::Duration::from_secs(#num * 60)),
        "h" => quote!(std::time::Duration::from_secs(#num * 3600)),
        "d" => quote!(std::time::Duration::from_secs(#num * 86400)),
        _ => panic!("不支持的时间单位: '{}'", unit),
    };
    dur_tokens
}

fn build_tool_registration(
    item_fn: &mut ItemFn,
    args: ToolArgs,
    extractors: Vec<proc_macro2::TokenStream>,
    arg_names: Vec<proc_macro2::TokenStream>,
    timeout_tokens: proc_macro2::TokenStream,
    rate_limit_tokens: proc_macro2::TokenStream,
    permissions: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (args_t, return_t) = parse_fn_signature(item_fn);
    let fn_ident = &item_fn.sig.ident;
    // 如果没有配置 name，默认使用函数名
    let tool_name = fn_ident.to_string();
    let annotation = args.anno.unwrap_or_default();
    let example = args.example.unwrap_or_default();

    let is_async = item_fn.sig.asyncness.is_some();

    // 动态生成 handler 闭包
    let handler_closure = if is_async {
        quote! {
            async_tool_handler(async |mut args: std::collections::HashMap<String, serde_json::Value>,ctx:ToolCallContext| {
                #(#extractors)*
                let res = #fn_ident(#(#arg_names),*).await?;
                Ok(res.into())
            })
        }
    } else {
        quote! {
            sync_tool_handler(|mut args: std::collections::HashMap<String, serde_json::Value>,ctx:ToolCallContext| {
                #(#extractors)*
                let res = #fn_ident(#(#arg_names),*)?;
                Ok(res.into())
            })
        }
    };

    // 返回最终向 tools 插入的 TokenStream
    quote! {
        let mut tool=Tool::new(
            #tool_name,
            #annotation,
            #example,
            (vec![ #(#args_t),*],#return_t),
            #handler_closure,
        );
        tool.timeout=#timeout_tokens;
        tool.rate_limit=#rate_limit_tokens;
        tool.permissions=#permissions;
        tools.insert(#tool_name, tool);
    }
}

// =====================================================================
// 解析函数签名：提取参数列表和返回值
// =====================================================================
pub fn parse_fn_signature(item_fn: &mut ItemFn) -> (Vec<TokenStream>, TokenStream) {
    let mut args_info = Vec::new();

    // 1. 遍历并解析函数参数
    for input in &mut item_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            // 提取参数名
            let arg_name = if let Pat::Ident(pat_ident) = &*pat_type.pat {
                pat_ident.ident.to_string()
            } else {
                continue; // 忽略不规则的参数匹配
            };
            if is_ctx_arg(&mut pat_type.attrs,true) {
                continue;
            }
            // 提取 #[arg("...")] 属性并从 AST 中移除它
            let annotation = extract_arg_annotation(&mut pat_type.attrs);

            // 解析 Option 并映射到 ToolValueType (复用我们之前的函数)
            let (inner_ty, is_option) = common::unwrap_option_type(&pat_type.ty);
            let value_type_quote = common::map_to_tool_value_type(inner_ty, &None);

            // 生成 TypeItemInfo 的 TokenStream
            args_info.push(quote! {
                TypeItemInfo {
                    name: #arg_name,
                    annotation: #annotation,
                    value_type: #value_type_quote,
                    is_option: #is_option,
                }
            });
        }
    }

    // 2. 解析函数返回值
    let ret_token = match &item_fn.sig.output {
        ReturnType::Default => quote! {None}, // 函数没有写返回值 -> ()
        ReturnType::Type(_, ty) => {
            // 剥离 Result 外壳，获取真正的返回类型
            let inner_ret_ty = unwrap_result_type(ty);

            // 判断真实返回类型是否为空元组 ()
            if is_unit_type(inner_ret_ty) {
                quote! {None}
            } else {
                // 如果不是空元组，将其映射为 ToolValueType
                let val_type = common::map_to_tool_value_type(inner_ret_ty, &None);
                quote! {Some(#val_type)}
            }
        }
    };

    (args_info, ret_token)
}

// =====================================================================
// 辅助函数
// =====================================================================

/// 解析参数上的 #[arg("...")] 属性并移除它
fn extract_arg_annotation(attrs: &mut Vec<Attribute>) -> String {
    let mut annotation = String::new();
    if let Some(idx) = attrs.iter().position(|a| a.path().is_ident("arg")) {
        let attr = attrs.remove(idx);
        // 解析形如 #[arg("文件路径")] 的单字符串参数
        if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
            annotation = lit.value();
        }
    }
    annotation
}
fn is_ctx_arg(attrs: &mut Vec<Attribute>, remove: bool) -> bool {
    if let Some(idx) = attrs.iter().position(|a| a.path().is_ident("ctx")) {
        if remove {
            let attr = attrs.remove(idx);
        }
        return true;
    }
    false
}
/// 剥离 Result/anyhow::Result 包装，获取 T
fn unwrap_result_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        // 获取路径的最后一段，检查是否为 Result
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Result<T, E> 或 anyhow::Result<T> 的第一个泛型参数都是 T
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    ty // 如果不是 Result，原样返回
}

/// 判断类型是否为空元组 ()
fn is_unit_type(ty: &Type) -> bool {
    if let Type::Tuple(tuple) = ty {
        tuple.elems.is_empty()
    } else {
        false
    }
}
