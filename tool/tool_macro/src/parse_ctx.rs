use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemFn};

pub fn handle_ctx(item_fn: &mut ItemFn, attr: Attribute) -> TokenStream {
    let fn_ident = &item_fn.sig.ident;
    quote! {
        Box::new(|info|{
            Box::new(#fn_ident(info))
        })
    }
}
