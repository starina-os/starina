use proc_macro::TokenStream;
use proc_macro_error::abort;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::parse_macro_input;
use syn::spanned::Spanned;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as syn::ItemFn);

    if func.attrs.len() > 0 {
        abort!(
            func.span(),
            "main function should not have any attributes (ftl_api_macros::main)"
        );
    }

    if func.sig.asyncness.is_some() {
        abort!(
            func.span(),
            "main function should not be async (ftl_api_macros::main)"
        );
    }

    if func.sig.ident != "main" {
        abort!(
            func.span(),
            "main function should be named 'main (ftl_api_macros::main)'"
        );
    }

    if func.sig.inputs.len() > 0 {
        abort!(
            func.span(),
            "main function should not have any arguments (ftl_api_macros::main)"
        );
    }

    let output: proc_macro2::TokenStream = quote! {
        #[no_mangle]
        #func
    };

    proc_macro::TokenStream::from(output)
}
