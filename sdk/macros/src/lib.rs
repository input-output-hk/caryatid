// Procedural macros for Caryatid module definition

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct, Meta, NestedMeta, LitStr};

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    // Parse the attributes
    let parsed_attrs = parse_macro_input!(attr as syn::AttributeArgs);
    let mut name = None;
    let mut description = None;

    // Extract name and description from the attributes
    for meta in parsed_attrs {
        if let NestedMeta::Meta(Meta::NameValue(meta_name_value)) = meta {
            if meta_name_value.path.is_ident("name") {
                if let syn::Lit::Str(lit) = meta_name_value.lit {
                    name = Some(lit);
                }
            } else if meta_name_value.path.is_ident("description") {
                if let syn::Lit::Str(lit) = meta_name_value.lit {
                    description = Some(lit);
                }
            }
        }
    }

    let name = name.expect("Module name is required");
    let description = description.unwrap_or_else(
        || LitStr::new("No description provided", name.span()));

    let expanded = quote! {
        #input

        impl Module for #struct_name {
            fn init(&self, context: &Context, config: &Config)
                    -> anyhow::Result<()> {
                Ok(())
            }

            fn get_name(&self) -> &'static str {
                #name
            }

            fn get_description(&self) -> &'static str {
                #description
            }
        }

        #[no_mangle]
        pub fn create_module(context: &Context, config: &Config)
                             -> Arc<dyn Module> {
            // Initialise own own tracing
            tracing_subscriber::fmt::init();

            // Use the process's Tokio runtime for the duration of init()
            // so it applies for its task spawns
            let _guard = context.tokio_runtime.enter();

            let module = #struct_name {};
            module.init(context, config).unwrap();
            Arc::new(module)
        }

        // Implement basic Debug for tracing
        impl std::fmt::Debug for #struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(#name)
                    .finish()
            }
        }
    };

    TokenStream::from(expanded)
}
