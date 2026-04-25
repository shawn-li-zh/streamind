extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemMod, parse_macro_input};
mod common;
mod parse_ctx;
mod parse_fn;
mod parse_type;

#[proc_macro_attribute]
pub fn package(args: TokenStream, input: TokenStream) -> TokenStream {
    // 解析宏的参数，例如 #[package("文件操作工具包")]
    let pkg_desc = parse_macro_input!(args as syn::LitStr).value();

    // 解析整个模块
    let mut module = parse_macro_input!(input as ItemMod);
    let mod_name = module.ident.to_string();

    let mut generated_types: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut generated_tools = Vec::new();
    let mut ctx_t = quote! {Box::new(|_|Arc::new(RwLock::new(())))};
    // 遍历模块中的所有的项 (struct, fn 等)
    if let Some((_, ref mut items)) = module.content {
        for item in items.iter_mut() {
            match item {
                // 处理结构体 (提取 TS 类型)
                syn::Item::Struct(item_struct) => {
                    if let Some(idx) = item_struct
                        .attrs
                        .iter()
                        .position(|a| a.path().is_ident("class"))
                    {
                        let attr = item_struct.attrs.remove(idx);
                        parse_type::handle_struct(item_struct,attr, &mut generated_types);
                    }
                }

                // 处理函数 (提取 Tool)
                syn::Item::Fn(item_fn) => {
                    for attr_idx in 0..item_fn.attrs.len() {
                        let a = &item_fn.attrs[attr_idx];
                        if a.path().is_ident("tool") {
                            let attr = item_fn.attrs.remove(attr_idx);
                            parse_fn::handle_fn(item_fn, attr, &mut generated_tools);
                            continue;
                        }
                        if a.path().is_ident("context") {
                            let attr = item_fn.attrs.remove(attr_idx);
                            ctx_t = parse_ctx::handle_ctx(item_fn, attr);
                        }
                    }
                }
                _ => {}
            }
        }

        // 注入 build_package 函数到模块内部
        let build_fn = quote! {
            pub fn build_package() -> streamind_core::tool::ToolPackage {
                use std::collections::HashMap;
                use tokio::sync::RwLock;
                use std::sync::Arc;
                use streamind_core::tool::{
                    ToolCallContext,
                    Tool, ToolHandler, ToolPackage,
                    async_tool_handler,
                    sync_tool_handler,
                };
                use streamind_core::tool::value_type::{ToolValueType, TypeInfo, TypeItemInfo};
                let mut types = HashMap::new();
                #(#generated_types)*

                let mut tools = HashMap::new();
                #(#generated_tools)*
                ToolPackage::new(
                    #mod_name,
                    #pkg_desc,
                    types,
                    tools,
                    #ctx_t
                 )
            }
        };
        // eprintln!("{}", build_fn);
        // 将生成的函数追加到模块的 items 中
        items.push(syn::parse2(build_fn).unwrap());
    }

    // 重新输出修改后的模块（自定义属性已被移除，并加上了 build 自动构建函数）
    quote!(#module).into()
}
