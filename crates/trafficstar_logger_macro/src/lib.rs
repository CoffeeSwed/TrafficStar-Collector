extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(StructLoggerName)]
pub fn struct_logger_name_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a `DeriveInput` (a struct or enum)
    let input = parse_macro_input!(input as DeriveInput);

    // Get the struct name
    let name = &input.ident;

    // Generate the implementation of the `hello` function for this struct
    let expanded = quote! {
        impl trafficstar_logger::trafficstar_logger_trait::TrafficStarStructName for #name {
            fn struct_name() -> &'static str {
                concat!("!", stringify!(#name))
            }
        }
    };

    // Return the generated code
    TokenStream::from(expanded)
}


#[proc_macro]
///Inserts hooks to register logger name at each function entry
pub fn register_logger_name(input: TokenStream) -> TokenStream {
    /*println!("Input is : \n{}",input);
    let input_clone = input.clone();
    let input_item = parse_macro_input!(input as syn::Item);  // Parse as a generic `Item`
    let tokens = input_item.to_token_stream();
    let mut impl_blocks = vec![];
    let parsed: syn::File = syn::parse2(tokens).expect("Failed to parse input");

    // Collect all impl blocks from the file
    for item in parsed.items {
        if let syn::Item::Impl(syn::ItemImpl { items, .. }) = item {
            impl_blocks.push(items);
            println!("Found one impl!");
        }
    }
    input_clone*/
    input
}