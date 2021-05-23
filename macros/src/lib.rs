#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use proc_macro::{Ident, TokenStream, TokenTree};
use std::convert::TryFrom;

#[derive(Debug)]
enum Error {
    NotEnum,
    NameIdentAfterEnum,
    VariantsAfterNameIndent,
    InnerFromAttribute,
}

struct Variant {
    name: Ident,
    inner_error: Option<String>,
}

struct Enum {
    name: Ident,
    variants: Vec<Variant>,
}

fn is_enum(tokens: &mut impl Iterator<Item = TokenTree>) -> bool {
    tokens.any(|t| {
        if let TokenTree::Ident(ident) = t {
            ident.to_string() == "enum"
        } else {
            false
        }
    })
}

fn parse_variants(tokens: &mut impl Iterator<Item = TokenTree>) -> Result<Vec<Variant>, Error> {
    let mut variants = vec![];

    while let Some(token) = tokens.next() {
        if let TokenTree::Ident(name) = token {
            variants.push(parse_variant(name, tokens)?);
        }
    }

    Ok(variants)
}

fn parse_variant(
    name: Ident,
    tokens: &mut impl Iterator<Item = TokenTree>,
) -> Result<Variant, Error> {
    match tokens.next() {
        None | Some(TokenTree::Punct(_)) => Ok(Variant {
            name,
            inner_error: None,
        }),

        Some(TokenTree::Group(inside_variant)) => {
            let mut inside_variant = inside_variant.stream().into_iter();
            advance_inner_from_attribute(&mut inside_variant)?;

            // anything else inside this group is the inner error name.
            Ok(Variant {
                name,
                inner_error: Some(inside_variant.map(|t| t.to_string()).collect()),
            })
        }

        _ => {
            unreachable!()
        }
    }
}

fn advance_inner_from_attribute(tokens: &mut impl Iterator<Item = TokenTree>) -> Result<(), Error> {
    fn advance(tokens: &mut impl Iterator<Item = TokenTree>) -> Option<()> {
        tokens.next().filter(|t| matches!(t, TokenTree::Punct(_)))?;

        if let TokenTree::Group(_) = tokens.next()? {
            Some(())
        } else {
            None
        }
    }

    advance(tokens).ok_or(Error::InnerFromAttribute)
}

impl TryFrom<TokenStream> for Enum {
    type Error = Error;

    fn try_from(stream: TokenStream) -> Result<Self, Self::Error> {
        let mut stream = stream.into_iter();

        if !is_enum(&mut stream) {
            return Err(Error::NotEnum);
        }

        let name = if let TokenTree::Ident(ident) = stream.next().expect("name after enum") {
            ident
        } else {
            return Err(Error::NameIdentAfterEnum);
        };

        let group = if let TokenTree::Group(group) = stream.next().expect("variants after name") {
            group
        } else {
            return Err(Error::VariantsAfterNameIndent);
        };

        let mut group = group.stream().into_iter();
        let variants = parse_variants(&mut group)?;

        Ok(Self { name, variants })
    }
}

#[proc_macro_derive(NoPanicErrorDebug, attributes(from))]
pub fn derive_no_panic_error_debug(input: TokenStream) -> TokenStream {
    // for token in input {
    //     println!("{:?}", token);
    // }

    // TokenStream::new()

    match Enum::try_from(input) {
        Ok(Enum { name, variants }) => {
            let variant_debugs: String = variants
                .iter()
                .map(|v| {
                    if v.inner_error.is_some() {
                        format!(
                            "Self::{variant}(inner) => write!(f, \"{variant}({{:?}})\", inner)?,\n",
                            variant = v.name
                        )
                    } else {
                        format!(
                            "Self::{variant} => f.write_str(\"{variant}\")?,\n",
                            variant = v.name
                        )
                    }
                })
                .collect();

            let from_impls: String = variants
                .into_iter()
                .filter(|v| v.inner_error.is_some())
                .map(|v| {
                    format!(
                        include_str!("impl_from_inner_error"),
                        name,
                        v.name,
                        inner_error = v.inner_error.unwrap()
                    )
                })
                .collect();

            let implementation =
                format!(include_str!("impl_debug"), name, variant_debugs, from_impls);

            // println!("{}", implementation);

            implementation.parse().unwrap()
        }

        Err(e) => {
            panic!("failed to implement NoPanicErrorDebug: {:?}", e);
        }
    }
}
