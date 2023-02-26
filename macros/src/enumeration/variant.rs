use proc_macro::{Delimiter, Group, Ident, TokenStream, TokenTree};
use std::fmt;

pub enum Fields {
    None,
    InnerError(String),
    Tuple(usize),
    // Struct(Vec<String>),
}

pub struct Variant {
    pub name: Ident,
    pub fields: Fields,
}

impl Variant {
    pub fn parse(name: Ident, mut tokens: impl Iterator<Item = TokenTree>) -> Self {
        match tokens.next() {
            None | Some(TokenTree::Punct(_)) => {
                // Fieldless variant.
                Variant {
                    name,
                    fields: Fields::None,
                }
            }

            Some(TokenTree::Group(group)) => {
                let fields = Self::parse_fields(&name, &group);
                Variant { name, fields }
            }

            token => {
                unreachable!("unexpected token after variant {}: {:?}", name, token);
            }
        }
    }

    fn parse_fields(name: &Ident, fields: &Group) -> Fields {
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
            Delimiter::Parenthesis => Self::parse_tuple_variant(name, field_tokens),

            Delimiter::Brace => Self::parse_struct_variant(name, field_tokens),

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
            Some(TokenTree::Punct(p)) if p.as_char() == '#' => {
                Self::parse_inner_error(name, tokens)
            }

            Some(_) => Fields::Tuple(Self::count_tuple_variant_fields(tokens)),

            None => {
                panic!("expected fields for {name}");
            }
        }
    }

    fn parse_struct_variant(name: &Ident, mut _tokens: impl Iterator<Item = TokenTree>) -> Fields {
        todo!("parse struct variant {}", name);
    }

    fn parse_inner_error(name: &Ident, mut tokens: impl Iterator<Item = TokenTree>) -> Fields {
        let has_from_attribute = tokens
            .next()
            .and_then(|token| if let TokenTree::Group(attribute) = token {
                Some(attribute)
            } else {
                None
            })
            .filter(|attribute| attribute.delimiter() == Delimiter::Bracket)
            .filter(|attribute| matches!(attribute.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "from"))
            .is_some();

        assert!(has_from_attribute, "expected #[from] attribute for variant {name}");

        let mut tokens = tokens.peekable();

        if let Some(TokenTree::Ident(_)) = tokens.peek() {
            let inner_error: TokenStream = tokens.collect();
            Fields::InnerError(inner_error.to_string())
        } else {
            panic!(
                "expected inner error name after #[from] attribute for variant {name}"
            );
        }
    }

    fn count_tuple_variant_fields(tokens: impl Iterator<Item = TokenTree>) -> usize {
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
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.fields {
            Fields::None => {
                writeln!(
                    f,
                    "Self::{variant} => f.write_str(\"{variant}\")?,\n",
                    variant = self.name
                )
            }

            Fields::InnerError(_) => {
                writeln!(
                    f,
                    "Self::{variant}(inner) => write!(f, \"{variant}({{:?}})\", inner)?,\n",
                    variant = self.name
                )
            }

            Fields::Tuple(size) => {
                let (mut placeholders, fields): (String, String) =
                    (0..*size).map(|i| ("{}, ", format!("f{i}, "))).unzip();

                // Trim final ", ".
                placeholders.pop();
                placeholders.pop();

                writeln!(
                    f,
                    "Self::{variant}({fields}) => write!(f, \"{variant}({placeholders})\", {fields})?,\n",
                    variant = self.name,
                    fields = fields,
                    placeholders = placeholders,
                )
            } // Fields::Struct(fields) => {
              //     let (placeholders, fields): (String, String) = fields
              //         .iter()
              //         .map(|field| (format!("{}: {{}}, ", field), format!("{}, ", field)))
              //         .unzip();

              //     writeln!(
              //         f,
              //         "Self::{variant} {{ {fields} }} => write!(f, \"{variant} {{ {placeholders} }}\", {fields})?,\n",
              //         variant = self.name,
              //         fields = fields,
              //         placeholders = placeholders,
              //     )
              // }
        }
    }
}
