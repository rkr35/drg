use proc_macro::{Ident, TokenStream, TokenTree};

mod variant;
pub use variant::Fields;
use variant::Variant;

pub struct Enum {
    pub name: Ident,
    pub variants: Vec<Variant>,
}

impl From<TokenStream> for Enum {
    fn from(stream: TokenStream) -> Self {
        let mut stream = stream.into_iter();

        assert!(is_enum(&mut stream), "not an enum");

        let Some(TokenTree::Ident(name)) = stream.next() else {
            panic!("expected name after enum keyword");
        };

        let Some(TokenTree::Group(group)) = stream.next() else {
            panic!("expected variants after {name}");
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
            variants.push(Variant::parse(name, &mut tokens));
        }
    }

    variants
}
