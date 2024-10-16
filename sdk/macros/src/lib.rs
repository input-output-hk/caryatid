// Procedural macros for Caryatid module definition

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let expanded = quote! {
        #input

        impl Module for #struct_name {
            fn init(&self, context: &Context) -> anyhow::Result<()> {
                Ok(())
            }

            fn get_name(&self) -> &'static str {
                stringify!(#struct_name)
            }
        }

        #[no_mangle]
        pub extern "C" fn create_module(context: &Context) -> *mut dyn Module {
            let module = #struct_name {};
            module.init(context).unwrap();
            Box::into_raw(Box::new(module))
        }
    };

    TokenStream::from(expanded)
}
