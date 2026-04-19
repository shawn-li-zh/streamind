use darling::{FromMeta, ast::NestedMeta};
use quote::quote;
use syn::{Attribute, Field, ItemStruct, Meta, Type};

use crate::common;

// =====================================================================
// 1. Darling 属性定义
// =====================================================================

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct TypeAttr {
    anno: Option<String>,
}

#[derive(Debug, FromMeta, Default)]
#[darling(default)]
struct FieldAttr {
    anno: Option<String>,
    // 修复：使用 syn::LitStr 替代 String，适配 darling 的字面量解析
    enums: Option<Vec<syn::LitStr>>,
}

// =====================================================================
// 2. 主入口函数 (保持极简)
// =====================================================================

pub fn handle_struct(
    item_struct: &mut ItemStruct,
    attr: Attribute,
    generated_types: &mut Vec<proc_macro2::TokenStream>,
) {
    // 提取注释
    let struct_annotation = extract_struct_annotation(&attr);

    let struct_name = item_struct.ident.to_string();

    // 遍历处理每个字段
    let fields_tokens: Vec<_> = item_struct.fields.iter_mut().map(process_field).collect();

    // 拼装最终结构
    generated_types.push(quote! {
        types.insert(#struct_name, TypeInfo {
            name: #struct_name,
            annotation: #struct_annotation,
            fields: vec![ #(#fields_tokens),* ],
        });
    });
}

/// 处理单个字段，生成 TypeFieldInfo 的 TokenStream
fn process_field(field: &mut Field) -> proc_macro2::TokenStream {
    let field_name = field.ident.as_ref().unwrap().to_string();
    let attr_info = extract_field_attr(&mut field.attrs);

    let (inner_ty, is_option) = common::unwrap_option_type(&field.ty);
    let value_type_quote = common::map_to_tool_value_type(inner_ty, &attr_info.enums);
    let annotation = attr_info.anno.unwrap_or_default();

    quote! {
        TypeItemInfo {
            name: #field_name,
            annotation: #annotation,
            value_type: #value_type_quote,
            is_option: #is_option,
        }
    }
}

/// 解析字段上的 #[field(...)] 属性
fn extract_field_attr(attrs: &mut Vec<Attribute>) -> FieldAttr {
    if let Some(idx) = attrs.iter().position(|a| a.path().is_ident("field")) {
        let attr = attrs.remove(idx);
        if let Meta::List(list) = attr.meta {
            if let Ok(nested) = NestedMeta::parse_meta_list(list.tokens.clone()) {
                if let Ok(parsed) = FieldAttr::from_list(&nested) {
                    return parsed;
                }
            }
        }
    }
    FieldAttr::default()
}

/// 解析结构体上的 #[class(...)] 属性
fn extract_struct_annotation(attr: &Attribute) -> String {
    // 尝试解析 #[class(anno="...")]
    if let Meta::List(list) = &attr.meta {
        if let Ok(nested) = NestedMeta::parse_meta_list(list.tokens.clone()) {
            if let Ok(parsed) = TypeAttr::from_list(&nested) {
                if let Some(ann) = parsed.anno {
                    return ann;
                }
            }
        }
    }
    // 兼容处理单字符串 #[class("...")]
    if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
        return lit.value();
    }
    String::new()
}
