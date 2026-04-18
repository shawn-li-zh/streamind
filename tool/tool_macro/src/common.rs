use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Field, ItemStruct, Meta, Type};
/// 剥离 Option 包装，返回 (内部类型, 是否为 Option)
pub fn unwrap_option_type(ty: &Type) -> (&Type, bool) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return (inner, true);
                    }
                }
            }
        }
    }
    (ty, false)
}

/// 将 Rust 类型映射为 ToolValueType 对应的 TokenStream
pub fn map_to_tool_value_type(inner_ty: &Type, enums: &Option<Vec<syn::LitStr>>) -> TokenStream {
    // 1. 优先判断是否存在 #[field(enums=["..."])] 注解
    if let Some(enum_lits) = enums {
        if !enum_lits.is_empty() {
            let vals = enum_lits.iter().map(|l| l.value());
            return quote!(ToolValueType::Enum(vec![ #(#vals),* ]));
        }
    }

    // 2. 递归解析真正的 AST 类型
    resolve_tool_type(inner_ty)
}

/// 递归解析 Rust AST Type，映射为 ToolValueType 树
fn resolve_tool_type(ty: &Type) -> TokenStream {
    match ty {
        // 处理引用类型，如 &str, &[T], &MyStruct
        Type::Reference(type_ref) => {
            if let Type::Slice(type_slice) = &*type_ref.elem {
                // &[T] 切片
                let inner_ts = resolve_tool_type(&type_slice.elem);
                quote!(ToolValueType::Array(Box::new(#inner_ts)))
            } else if let Type::Path(type_path) = &*type_ref.elem {
                if type_path.path.is_ident("str") {
                    // &str
                    quote!(ToolValueType::String)
                } else {
                    // 其他引用 &T，直接解析 T
                    resolve_tool_type(&type_ref.elem)
                }
            } else {
                resolve_tool_type(&type_ref.elem)
            }
        }

        // 处理固定长度数组，如 [T; N]
        Type::Array(type_array) => {
            let inner_ts = resolve_tool_type(&type_array.elem);
            quote!(ToolValueType::Array(Box::new(#inner_ts)))
        }

        // 处理常规类型路径，如 String, Vec<T>, HashMap<K, V>, File
        Type::Path(type_path) => {
            // 获取路径的最后一段（例如 std::vec::Vec 的 Vec）
            let segment = type_path.path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                // 基础类型
                "bool" => quote!(ToolValueType::Bool),
                "String" | "char" => quote!(ToolValueType::String),
                "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
                | "usize" | "isize" => {
                    quote!(ToolValueType::Number)
                }
                "DateTime" | "Local" | "Utc" => quote!(ToolValueType::String),

                // 数组集合 Vec<T>
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_ts = resolve_tool_type(inner_ty);
                            return quote!(ToolValueType::Array(Box::new(#inner_ts)));
                        }
                    }
                    // 兜底：如果没写泛型，假定为 Object("Unknown")
                    quote!(ToolValueType::Array(Box::new(ToolValueType::Object(
                        "Unknown"
                    ))))
                }

                // 键值对映射 HashMap<K, V> 或 BTreeMap<K, V>
                "HashMap" | "BTreeMap" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        // Map 需要提取第二个泛型参数 (Value类型)
                        let mut types = args.args.iter().filter_map(|arg| {
                            if let syn::GenericArgument::Type(t) = arg {
                                Some(t)
                            } else {
                                None
                            }
                        });
                        types.next(); // 跳过 Key
                        if let Some(value_ty) = types.next() {
                            let inner_ts = resolve_tool_type(value_ty);
                            return quote!(ToolValueType::Map(Box::new(#inner_ts)));
                        }
                    }
                    quote!(ToolValueType::Map(Box::new(ToolValueType::Object(
                        "Unknown"
                    ))))
                }

                // 无法匹配基础和集合类型的，统统视为自定义 Object
                _ => {
                    // ident_str 就是对象名，例如 "File", "Context"
                    quote!(ToolValueType::Object(#ident_str))
                }
            }
        }

        // 兜底：处理非常规类型（如 Tuple 元组、闭包等）
        _ => {
            let fallback_name = quote!(#ty).to_string().replace(" ", "");
            quote!(ToolValueType::Object(#fallback_name))
        }
    }
}
