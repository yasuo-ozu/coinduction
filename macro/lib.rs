use proc_macro::TokenStream;

mod common;
mod traitdef;
mod typedef;
mod coinduction;
mod internal;
mod finalize;

#[proc_macro_attribute]
pub fn traitdef(args: TokenStream, input: TokenStream) -> TokenStream {
    traitdef::traitdef_impl(args, input)
}

#[proc_macro_attribute]
pub fn typedef(args: TokenStream, input: TokenStream) -> TokenStream {
    typedef::typedef_impl(args, input)
}

#[proc_macro_attribute]
pub fn coinduction(args: TokenStream, input: TokenStream) -> TokenStream {
    coinduction::coinduction_impl(args, input)
}

#[proc_macro]
pub fn __internal(input: TokenStream) -> TokenStream {
    internal::internal_impl(input)
}

#[proc_macro]
pub fn __finalize(input: TokenStream) -> TokenStream {
    finalize::finalize_impl(input)
}