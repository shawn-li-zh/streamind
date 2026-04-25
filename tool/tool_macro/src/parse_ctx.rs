use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemFn};

pub fn handle_ctx(item_fn: &mut ItemFn, attr: Attribute) -> TokenStream {
    let fn_ident = &item_fn.sig.ident;
    quote! {
        Box::new(|info|{
                let ctx=#fn_ident(info);
                Arc::new(RwLock::new(ctx))
            })
    }
}
