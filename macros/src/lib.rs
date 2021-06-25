#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]

use proc_macro::{Delimiter, Group, Ident, TokenStream, TokenTree};

enum Fields {
    None,
    InnerError(String),
    Tuple(usize),
    Struct(Vec<String>),
}

struct Variant {
    name: Ident,
    fields: Fields,
}

struct Enum {
    name: Ident,
    variants: Vec<Variant>,
}

impl From<TokenStream> for Enum {
    fn from(stream: TokenStream) -> Self {
        let mut stream = stream.into_iter();

        if !is_enum(&mut stream) {
            panic!("not an enum");
        }

        let name = if let Some(TokenTree::Ident(ident)) = stream.next() {
            ident
        } else {
            panic!("expected name after enum keyword");
        };

        let group = if let Some(TokenTree::Group(group)) = stream.next() {
            group
        } else {
            panic!("expected variants after {}", name);
        };

        Self {
            name,
            variants: parse_variants(group.stream().into_iter()),
        }
    }
}

fn is_enum(mut tokens: impl Iterator<Item = TokenTree>) -> bool {
    tokens.any(|t| matches!(t, TokenTree::Ident(ident) if ident.to_string() == "enum"))
}

fn parse_variants(mut tokens: impl Iterator<Item = TokenTree>) -> Vec<Variant> {
    let mut variants = vec![];

    while let Some(token) = tokens.next() {
        if let TokenTree::Ident(name) = token {
            variants.push(parse_variant(name, &mut tokens));
        }
    }

    variants
}

fn parse_variant(name: Ident, mut tokens: impl Iterator<Item = TokenTree>) -> Variant {
    match tokens.next() {
        None | Some(TokenTree::Punct(_)) => {
            // Fieldless variant.
            Variant {
                name,
                fields: Fields::None,
            }
        }

        Some(TokenTree::Group(group)) => {
            let fields = parse_variant_fields(&name, group);

            Variant { name, fields }
        }

        token => {
            unreachable!("unexpected token after variant {}: {:?}", name, token);
        }
    }
}

fn parse_variant_fields(name: &Ident, fields: Group) -> Fields {
    /*
    enum MyEnum {
        UnitVariant,

        TupleVariant(Type1, Type2, ...),

        VariantWithInnerError(#[from] InnerError),

        StructVariant {
            Field1: Type1,
            Field2: Type2,
            ...
        },

        UnitVariantWithoutPunct
    }
    */

    let field_tokens = fields.stream().into_iter();

    match fields.delimiter() {
        Delimiter::Parenthesis => parse_tuple_variant(name, field_tokens),

        Delimiter::Brace => {
            // parse_struct_variant(name, field_tokens)
            todo!("parse struct variant")
        }

        unknown_delimiter => {
            unreachable!(
                "unexpected field delimiter for variant {}: {:?}",
                name, unknown_delimiter
            );
        }
    }
}

fn parse_tuple_variant(name: &Ident, mut tokens: impl Iterator<Item = TokenTree>) -> Fields {
    match tokens.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == '#' => parse_inner_error(name, tokens),

        Some(_) => Fields::Tuple(count_tuple_variant_fields(tokens)),

        None => {
            panic!("expected fields for {}", name);
        }
    }
}

fn count_tuple_variant_fields(mut tokens: impl Iterator<Item = TokenTree>) -> usize {
    let mut num_fields = 1;

    let mut tokens = tokens.peekable();

    while let Some(token) = tokens.next() {
        if matches!(token, TokenTree::Punct(p) if p.as_char() == ',') {
            let is_there_a_field_after_this_comma = tokens.peek().is_some();

            if is_there_a_field_after_this_comma {
                num_fields += 1;
            }
        }
    }

    num_fields
}

fn parse_inner_error(name: &Ident, mut tokens: impl Iterator<Item = TokenTree>) -> Fields {
    let is_missing_from_attribute = tokens
        .next()
        .and_then(|token| if let TokenTree::Group(attribute) = token {
            Some(attribute)
        } else {
            None
        })
        .filter(|attribute| attribute.delimiter() == Delimiter::Bracket)
        .filter(|attribute| matches!(attribute.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "from"))
        .is_none();

    if is_missing_from_attribute {
        panic!("expected #[from] attribute for variant {}", name);
    }

    let mut tokens = tokens.peekable();

    if let Some(TokenTree::Ident(_)) = tokens.peek() {
        let inner_error: TokenStream = tokens.collect();
        Fields::InnerError(inner_error.to_string())
    } else {
        panic!(
            "expected inner error name after #[from] attribute for variant {}",
            name
        );
    }
}

#[proc_macro_derive(NoPanicErrorDebug, attributes(from))]
pub fn derive_no_panic_error_debug(input: TokenStream) -> TokenStream {
    // for token in input {
    //     println!("{:?}", token);
    // }

    // TokenStream::new()

    let Enum { name, variants } = Enum::from(input);

    let variant_debugs: String = variants
        .iter()
        .map(|v| {
            match &v.fields {
                Fields::None => {
                    format!(
                        "Self::{variant} => f.write_str(\"{variant}\")?,\n",
                        variant = v.name
                    )
                }

                Fields::InnerError(_) => {
                    format!(
                        "Self::{variant}(inner) => write!(f, \"{variant}({{:?}})\", inner)?,\n",
                        variant = v.name
                    )
                }

                Fields::Tuple(size) => {
                    let (mut placeholders, fields): (String, String) =
                        (0..*size)
                            .map(|i| ("{:?}, ", format!("f{}, ", i)))
                            .unzip();

                    // Trim final ", ".
                    placeholders.pop();
                    placeholders.pop();

                    format!(
                        "Self::{variant}({fields}) => write!(f, \"{variant}({placeholders})\", {fields})?,\n",
                        variant = v.name,
                        fields = fields,
                        placeholders = placeholders,
                    )
                }

                Fields::Struct(fields) => {
                    let (placeholders, fields): (String, String) =
                        fields.iter()
                            .map(|field| (format!("{}: {{:?}}, ", field), format!("{}, ", field)))
                            .unzip();

                    format!(
                        "Self::{variant} {{ {fields} }} => write!(f, \"{variant} {{ {placeholders} }}\", {fields})?,\n",
                        variant = v.name,
                        fields = fields,
                        placeholders = placeholders,
                    )
                }
            }
        })
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

    println!("{}", implementation);

    implementation.parse().unwrap()
}
