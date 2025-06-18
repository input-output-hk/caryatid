// Procedural macros for Caryatid module definition

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Fields, ItemStruct, Lit, LitStr, Meta, NestedMeta};

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let has_generics = !generics.params.is_empty();

    // Parse the attributes
    let parsed_attrs = parse_macro_input!(attr as syn::AttributeArgs);
    let mut name = None;
    let mut description = None;
    let mut message_type = None;

    // Extract name and description from the attributes
    for meta in parsed_attrs {
        match meta {
            NestedMeta::Meta(Meta::NameValue(ref meta_name_value))
                if meta_name_value.path.is_ident("name") =>
            {
                if let Lit::Str(ref lit) = meta_name_value.lit {
                    name = Some(lit.clone());
                }
            }
            NestedMeta::Meta(Meta::NameValue(ref meta_name_value))
                if meta_name_value.path.is_ident("description") =>
            {
                if let Lit::Str(ref lit) = meta_name_value.lit {
                    description = Some(lit.clone());
                }
            }
            NestedMeta::Meta(Meta::List(ref meta_list))
                if meta_list.path.is_ident("message_type") =>
            {
                if let Some(NestedMeta::Meta(Meta::Path(ref path))) = meta_list.nested.first() {
                    message_type = Some(path.clone());
                }
            }
            _ => {}
        }
    }

    let name = match name {
        Some(n) => n,
        None => {
            return syn::Error::new_spanned(&struct_name, "Module attribute 'name' is required")
                .to_compile_error()
                .into();
        }
    };

    let description =
        description.unwrap_or_else(|| LitStr::new("No description provided", name.span()));

    let message_type = match message_type {
        Some(t) => t,
        None => {
            return syn::Error::new_spanned(
                &struct_name,
                "Module attribute 'message_type' is required",
            )
            .to_compile_error()
            .into();
        }
    };

    // Optional marker needed for generic modules
    let marker = if has_generics {
        quote! {
            _marker: std::marker::PhantomData,
        }
    } else {
        quote! {}
    };

    // Automatically add the marker field if it is generic
    if has_generics {
        if let Fields::Unit = input.fields {
            input.fields = Fields::Named(syn::FieldsNamed {
                brace_token: Default::default(),
                named: syn::punctuated::Punctuated::new(),
            });
        }

        if let Fields::Named(ref mut fields_named) = input.fields {
            fields_named.named.push(syn::Field {
                attrs: vec![],
                vis: syn::Visibility::Inherited,
                ident: Some(syn::Ident::new("_marker", struct_name.span())),
                colon_token: Some(Default::default()),
                ty: syn::parse_quote!(std::marker::PhantomData #type_generics),
            });
        }
    }

    // Macro expansion
    let expanded = quote! {
        // Struct definition - possibly modified
        #input

        // Implement Module init etc.
        #[caryatid_sdk::async_trait]
        impl #impl_generics Module<#message_type> for #struct_name #type_generics #where_clause {

            // Implement init, calling down to struct's own
            async fn init(&self, context: Arc<Context<#message_type>>, config: Arc<Config>)
                    -> anyhow::Result<()> {
                #struct_name::init(self, context, config).await
            }

            // Get name using macro's attribute
            fn get_name(&self) -> &'static str {
                #name
            }

            // Get description using macro's attribute
            fn get_description(&self) -> &'static str {
                #description
            }
        }

        // Provide a register() function
        impl #impl_generics #struct_name #type_generics #where_clause {

            // Register at startup (call this in main())
            pub fn register(registry: &mut dyn caryatid_sdk::ModuleRegistry<#message_type>) {
                let module = Arc::new(#struct_name {
                    #marker
                });
                registry.register(module);
            }
        }

        // Implement basic Debug for tracing
        impl #impl_generics std::fmt::Debug for #struct_name #type_generics {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(#name)
                    .finish()
            }
        }
    };

    TokenStream::from(expanded)
}
