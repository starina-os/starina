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

    if func.sig.inputs.first().is_none() {
        abort!(
            func.sig.inputs.span(),
           "main function should have one parameter, i.e. fn main(env: ftl_api::environ::Environ) (ftl_api_macros::main)"
        );
    }

    let output: proc_macro2::TokenStream = quote! {
        #func

        #[no_mangle]
        pub extern "C" fn ftl_app_main(
            vsyscall: *const ::ftl_api::types::syscall::VsyscallPage,
        ) {
            // SAFETY: We won't call this function twice.
            unsafe {
                ::ftl_api::init::init_internal(vsyscall);
            }

            let env_bytes = unsafe {
                ::core::slice::from_raw_parts((*vsyscall).environ_ptr, (*vsyscall).environ_len)
            };
            let env_str = ::core::str::from_utf8(env_bytes).unwrap();
            let env = ::ftl_api::environ::Environ::parse(env_str);

            main(env);
        }
    };

    proc_macro::TokenStream::from(output)
}
