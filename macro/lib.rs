use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use syn::parse::{Parse, ParseStream};
use syn::*;
use template_quote::ToTokens;

/// A wrapper for Path that ensures no path arguments are present
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NoArgPath(pub Path);

impl Parse for NoArgPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        for segment in &path.segments {
            if !matches!(segment.arguments, PathArguments::None) {
                abort!(segment, "Path arguments are not allowed");
            }
        }
        Ok(NoArgPath(path))
    }
}

impl ToTokens for NoArgPath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

fn remove_path_args(path: &Path) -> NoArgPath {
    let mut new_path = path.clone();
    new_path
        .segments
        .iter_mut()
        .last()
        .expect("pats should have at least one segment")
        .arguments = PathArguments::None;
    NoArgPath(new_path)
}

fn get_random() -> u64 {
    use core::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

/// The text of `tokens`, to compare paths by. announce writes `crate::` as `$crate::` in what it
/// sends, so `$crate` is compared as `crate`: within one crate they name the same item.
fn comparison_key(tokens: &impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .replace("$crate", "crate")
}

/// `tokens` for error messages, closer to how they are written: `a::B<C>`, not `a :: B < C >`.
fn display(tokens: &impl ToTokens) -> String {
    let s = format!(" {} ", tokens.to_token_stream());
    s.replace(" :: ", "::")
        .replace(" < ", "<")
        .replace(" <", "<")
        .replace(" >", ">")
        .replace(" : ", ": ")
        .replace("& ", "&")
        .trim()
        .to_string()
}

/// Unwrap TypeGroup/TypeParen which may be introduced during macro expansion
fn unwrap_type_group(typ: Type) -> Type {
    match typ {
        Type::Group(TypeGroup { elem, .. }) | Type::Paren(TypeParen { elem, .. }) => {
            unwrap_type_group(*elem)
        }
        _ => typ,
    }
}

mod announce_attrs;
mod coinduction;
mod matching;
mod next_step;
mod rules;
mod solver;
mod typedef;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn coinduction(attr: TokenStream, input: TokenStream) -> TokenStream {
    match parse_macro_input!(input as Item) {
        Item::Mod(item) => {
            let args = parse_macro_input!(attr as coinduction::ModuleArgs);
            coinduction::coinduction(item, args).into()
        }
        item => abort!(item, "`#[coinduction]` can only be applied to a `mod`"),
    }
}

#[proc_macro_error]
#[proc_macro]
pub fn __next_step(input: TokenStream) -> TokenStream {
    next_step::next_step_macro(input.into()).into()
}
