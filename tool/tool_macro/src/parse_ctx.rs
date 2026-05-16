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

pub fn handle_shutdown(item_fn: &mut ItemFn, attr: Attribute) -> TokenStream {
    let fn_ident = &item_fn.sig.ident;
    //判断是否异步
    if item_fn.sig.asyncness.is_some() {
        quote! {
            Box::new(move |info,context_data|{
               #fn_ident(info,context_data).boxed()
            })
        }
    } else {
        quote! {
            Box::new(move |info,context_data|{
                #fn_ident(info,context_data)?;
                async {Ok(())}.boxed()
            })
        }
    }
}
