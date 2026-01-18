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

/// Try to parse `coinduction = <path>` as the first argument
/// Returns coinduction path, defaults to `::coinduction`
fn try_parse_coinduction_args(input: ParseStream) -> syn::Result<NoArgPath> {
    if input.peek(Ident) && input.peek2(Token![=]) {
        let ident: Ident = input.parse()?;
        if ident == "coinduction" {
            input.parse::<Token![=]>()?;
            let path: NoArgPath = input.parse()?;

            // Parse optional comma after coinduction = <path>
            // If there's no comma and input is not empty, it's an error
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else if !input.is_empty() {
                return Err(input.error("Expected comma after coinduction argument"));
            }

            return Ok(path);
        } else {
            abort!(&ident, "Bad argument: {}", &ident);
        }
    }
    let default_path: Path = syn::parse_str("::coinduction").unwrap();
    Ok(NoArgPath(default_path))
}

fn get_random() -> u64 {
    use core::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

mod coinduction;
mod matching;
mod next_step;
mod solver;
mod traitdef;
mod typedef;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn traitdef(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);
    let args = parse_macro_input!(attr as traitdef::TraitDefArgs);
    traitdef::traitdef(item, args).into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn typedef(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemMod);
    let args = parse_macro_input!(attr as typedef::TypeDefArgs);
    typedef::typedef(item, args).into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn coinduction(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemMod);
    let args = parse_macro_input!(attr as coinduction::CoinductionArgs);
    coinduction::coinduction(item, args).into()
}

#[proc_macro]
pub fn __next_step(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as next_step::NextStepArgs);
    next_step::next_step(args).into()
}
