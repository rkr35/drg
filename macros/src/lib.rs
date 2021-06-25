#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use proc_macro::TokenStream;

mod enumeration;
use enumeration::{Enum, Fields};

#[proc_macro_derive(NoPanicErrorDebug, attributes(from))]
pub fn derive_no_panic_error_debug(input: TokenStream) -> TokenStream {
    // for token in input {
    //     println!("{:?}", token);
    // }

    // TokenStream::new()

    let Enum { name, variants } = Enum::from(input);

    let variant_debugs: String = variants
        .iter()
        .map(|v| v.to_string())
        .collect();

    let from_impls: String = variants
        .into_iter()
        .filter_map(|v| {
            if let Fields::InnerError(inner_error) = v.fields {
                Some(format!(
                    include_str!("impl_from_inner_error"),
                    name,
                    v.name,
                    inner_error = inner_error,
                ))
            } else {
                None
            }
        })
        .collect();

    let implementation = format!(include_str!("impl_debug"), name, variant_debugs, from_impls);

    // println!("{}", implementation);

    implementation.parse().unwrap()
}
