//! The announcements of the items of a `#[coinduction]` module: how the module names announce's
//! attribute macro and derive, and adding the impls of a type to its announcement.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::NoArgPath;

/// A name of announce's attribute macro, derive or crate, and whether it is reached through the
/// re-export `<coinduction>::announce`. The generated code of such an announcement is given
/// `crate = <coinduction>::announce`, so that the crate need not depend on announce.
type Name = (Ident, bool);

/// How the module names announce's attribute macro and its derive.
///
/// A path names them when it is `<announce>::announce` or `<announce>::Announce`, `<announce>`
/// being the name of the announce dependency in the crate's `Cargo.toml` (renames included) or
/// a name the module imports `<coinduction>::announce` as, or when it is
/// `<coinduction>::announce::announce` or `<coinduction>::announce::Announce`. A single name does
/// when the module imports it from there, also under another name, or when the module does not
/// import that name from another crate (it may come from a glob import or from an outer module).
/// Attributes inside `#[cfg_attr(..)]` count too.
pub struct AnnounceNames {
    coinduction: NoArgPath,
    crates: Vec<Name>,
    attrs: Vec<Name>,
    derives: Vec<Name>,
}

/// Whether `path` is `prefix` followed by `rest`, comparing the names only.
fn is_path(path: &[&Ident], prefix: &NoArgPath, rest: &[&str]) -> bool {
    let prefix = prefix.0.segments.iter().map(|seg| &seg.ident).collect::<Vec<_>>();
    path.len() == prefix.len() + rest.len()
        && path.iter().zip(&prefix).all(|(a, b)| a == b)
        && path[prefix.len()..].iter().zip(rest).all(|(a, b)| a == b)
}

impl AnnounceNames {
    pub(crate) fn new(items: &[Item], coinduction: &NoArgPath) -> Self {
        let krate = announce::machinery::Options::default().krate();
        let span = proc_macro2::Span::call_site();
        let mut names = AnnounceNames {
            coinduction: coinduction.clone(),
            crates: vec![(krate.segments.last().unwrap().ident.clone(), false)],
            attrs: vec![(Ident::new("announce", span), false)],
            derives: vec![(Ident::new("Announce", span), false)],
        };
        let uses = items
            .iter()
            .filter_map(|item| match item {
                Item::Use(item_use) => Some(&item_use.tree),
                _ => None,
            })
            .collect::<Vec<_>>();
        // The names of the crate first: `use coinduction::announce as ann; use ann::Announce;`
        for tree in &uses {
            names.collect_imports(tree, &mut Vec::new(), &mut Vec::new(), true);
        }
        let mut foreign = Vec::new();
        for tree in &uses {
            names.collect_imports(tree, &mut Vec::new(), &mut foreign, false);
        }
        names.attrs.retain(|(name, _)| !foreign.contains(name));
        names.derives.retain(|(name, _)| !foreign.contains(name));
        names
    }

    /// Record the names of the announce crate that `tree` imports (`crates`), or the names of its
    /// attribute macro and derive, and in `foreign` the names it imports from another crate.
    fn collect_imports(
        &mut self,
        tree: &UseTree,
        prefix: &mut Vec<Ident>,
        foreign: &mut Vec<Ident>,
        crates: bool,
    ) {
        let mut import = |ident: &Ident, name: &Ident, prefix: &[Ident]| {
            let mut path = prefix.iter().collect::<Vec<_>>();
            path.push(ident);
            if crates {
                if is_path(&path, &self.coinduction, &["announce"]) {
                    self.crates.push((name.clone(), true));
                }
                return;
            }
            let reexport = is_path(&path[..path.len() - 1], &self.coinduction, &["announce"]);
            let krate = match prefix {
                [krate] => self.crates.iter().find(|(name, _)| name == krate).map(|(_, via)| *via),
                _ => None,
            };
            let from = if reexport { Some(true) } else { krate };
            let first = prefix.first();
            match (from, ident.to_string().as_str()) {
                (Some(via), "announce") => self.attrs.push((name.clone(), via)),
                (Some(via), "Announce") => self.derives.push((name.clone(), via)),
                _ if is_path(&path, &self.coinduction, &["announce"]) => {}
                _ if first.is_some_and(|first| {
                    first != "crate" && first != "self" && first != "super"
                }) =>
                {
                    foreign.push(name.clone())
                }
                // A re-export in this crate: it may be announce's
                (None, "announce") => self.attrs.push((name.clone(), false)),
                (None, "Announce") => self.derives.push((name.clone(), false)),
                _ => {}
            }
        };
        match tree {
            UseTree::Path(UsePath { ident, tree, .. }) => {
                prefix.push(ident.clone());
                self.collect_imports(tree, prefix, foreign, crates);
                prefix.pop();
            }
            UseTree::Name(UseName { ident }) => import(ident, ident, prefix),
            UseTree::Rename(UseRename { ident, rename, .. }) => import(ident, rename, prefix),
            UseTree::Group(UseGroup { items, .. }) => {
                for tree in items {
                    self.collect_imports(tree, prefix, foreign, crates);
                }
            }
            UseTree::Glob(_) => {}
        }
    }

    /// Whether `path` names announce's attribute macro (`leaf` = `announce`) or derive (`leaf` =
    /// `Announce`), given the single names in `names`; and whether through the re-export.
    fn names(&self, path: &Path, names: &[Name], leaf: &str) -> Option<bool> {
        let segments = path.segments.iter().map(|seg| &seg.ident).collect::<Vec<_>>();
        match segments[..] {
            [name] if path.leading_colon.is_none() => {
                names.iter().find(|(n, _)| n == name).map(|(_, via)| *via)
            }
            [krate, last] if last == leaf => {
                self.crates.iter().find(|(n, _)| n == krate).map(|(_, via)| *via)
            }
            _ => is_path(&segments, &self.coinduction, &["announce", leaf]).then_some(true),
        }
    }

    /// Whether `meta` is `#[announce]` / `#[announce(...)]`: the attribute macro, or the helper
    /// attribute of `#[derive(Announce)]`.
    fn attr(&self, meta: &Meta) -> Option<bool> {
        self.names(meta.path(), &self.attrs, "announce")
    }

    /// The predicates of the `cfg_attr`s around the first `#[derive(Announce)]` in `meta`, and
    /// whether the derive is reached through the re-export.
    fn derive_cfgs(&self, meta: &Meta, mut cfgs: Vec<Meta>) -> Option<(Vec<Meta>, bool)> {
        if meta.path().is_ident("derive") {
            let paths = meta
                .require_list()
                .ok()?
                .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()?;
            paths
                .iter()
                .find_map(|path| self.names(path, &self.derives, "Announce"))
                .map(|via| (cfgs, via))
        } else {
            let (predicate, metas) = cfg_attr_parts(meta)?;
            cfgs.push(predicate);
            metas
                .iter()
                .find_map(|meta| self.derive_cfgs(meta, cfgs.clone()))
        }
    }

    fn announces(&self, meta: &Meta) -> bool {
        self.attr(meta).is_some()
            || self.derive_cfgs(meta, Vec::new()).is_some()
            || cfg_attr_parts(meta)
                .is_some_and(|(_, metas)| metas.iter().any(|meta| self.announces(meta)))
    }

    /// Whether the item is announced: it has `#[announce]`, or derives `Announce`.
    pub fn is_announced(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| self.announces(&attr.meta))
    }

    /// Add `meta = <add>` to the options of the announcement of an item: to its `#[announce(...)]`
    /// attribute, or, for `#[derive(Announce)]` without one, in a new helper attribute under the
    /// same `cfg_attr`s as the derive. An announcement through the re-export also gets
    /// `crate = <coinduction>::announce`, unless it has a `crate` option.
    pub fn add_meta(&self, attrs: &mut Vec<Attribute>, add: &TokenStream) {
        let derive = attrs
            .iter()
            .find_map(|attr| self.derive_cfgs(&attr.meta, Vec::new()));
        for attr in attrs.iter_mut() {
            if self.add_meta_to(&mut attr.meta, add, derive.as_ref().map(|(_, via)| *via)) {
                return;
            }
        }
        let (cfgs, via) = derive.unwrap_or_default();
        let krate = via.then(|| self.krate());
        if cfgs.is_empty() {
            attrs.push(parse_quote!(#[announce(#krate meta = #add)]));
        } else {
            attrs.push(parse_quote!(#[cfg_attr(all(#(#cfgs),*), announce(#krate meta = #add))]));
        }
    }

    /// `crate = <coinduction>::announce,`
    fn krate(&self) -> TokenStream {
        let coinduction = &self.coinduction;
        quote!(crate = #coinduction::announce,)
    }

    /// `derive`: whether the derive of the item is reached through the re-export, if it derives
    /// `Announce`; its helper attribute is `announce` under any name of the derive.
    fn add_meta_to(&self, meta: &mut Meta, add: &TokenStream, derive: Option<bool>) -> bool {
        let via = match derive {
            Some(via) if meta.path().is_ident("announce") => Some(via),
            _ => self.attr(meta),
        };
        if let Some(via) = via {
            let path = meta.path().clone();
            let options = match &*meta {
                Meta::Path(_) => TokenStream::new(),
                Meta::List(list) => {
                    let mut options = list.tokens.clone();
                    if !options.is_empty()
                        && !matches!(options.clone().into_iter().last(), Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == ',')
                    {
                        options.extend(quote!(,));
                    }
                    options
                }
                Meta::NameValue(_) => abort!(meta, "expected `#[announce]` or `#[announce(...)]`"),
            };
            let has_krate = parse2::<announce::machinery::Options>(options.clone())
                .is_ok_and(|options| options.krate.is_some());
            let krate = (via && !has_krate).then(|| self.krate());
            *meta = parse_quote!(#path(#options #krate meta = #add));
            true
        } else if let Some((predicate, mut metas)) = cfg_attr_parts(meta) {
            let added = metas.iter_mut().any(|meta| self.add_meta_to(meta, add, derive));
            if added {
                *meta = parse_quote!(cfg_attr(#predicate, #metas));
            }
            added
        } else {
            false
        }
    }
}

/// The predicate and the attributes of `cfg_attr(<predicate>, <attributes>)`.
fn cfg_attr_parts(meta: &Meta) -> Option<(Meta, Punctuated<Meta, Token![,]>)> {
    let Meta::List(list) = meta else {
        return None;
    };
    if !list.path.is_ident("cfg_attr") {
        return None;
    }
    list.parse_args_with(|input: ParseStream| {
        let predicate = input.parse()?;
        input.parse::<Token![,]>()?;
        Ok((predicate, Punctuated::parse_terminated(input)?))
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(uses: &[&str]) -> AnnounceNames {
        let items = uses
            .iter()
            .map(|u| syn::parse_str::<Item>(u).unwrap())
            .collect::<Vec<_>>();
        AnnounceNames::new(&items, &NoArgPath(parse_quote!(::coinduction)))
    }

    fn announced(names: &AnnounceNames, attrs: &str) -> bool {
        let item: ItemStruct = syn::parse_str(&format!("{} struct S;", attrs)).unwrap();
        names.is_announced(&item.attrs)
    }

    #[test]
    fn attribute_paths() {
        let n = names(&[]);
        assert!(announced(&n, "#[announce]"));
        assert!(announced(&n, "#[announce(macro_name = M)]"));
        assert!(announced(&n, "#[announce::announce]"));
        assert!(announced(&n, "#[::announce::announce]"));
        assert!(!announced(&n, "#[my::announce]"));
        assert!(!announced(&n, "#[a::announce::announce]"));
        assert!(!announced(&n, "#[announce::other]"));
    }

    #[test]
    fn derive_paths() {
        let n = names(&[]);
        assert!(announced(&n, "#[derive(Debug, Announce)]"));
        assert!(announced(&n, "#[derive(announce::Announce)]"));
        assert!(!announced(&n, "#[derive(other::Announce)]"));
        assert!(!announced(&n, "#[derive(Debug)]"));
    }

    #[test]
    fn imports() {
        let n = names(&["use announce::announce as ann;", "use announce::{Announce as Ann};"]);
        assert!(announced(&n, "#[ann]"));
        assert!(announced(&n, "#[derive(Ann)]"));
        // Imported from another crate
        let n = names(&["use my_attrs::announce;", "use other::Announce;"]);
        assert!(!announced(&n, "#[announce]"));
        assert!(!announced(&n, "#[derive(Announce)]"));
        assert!(announced(&n, "#[announce::announce]"));
        // Re-exported in this crate, or from a glob import: may be announce's
        let n = names(&["use super::announce;", "use crate::*;"]);
        assert!(announced(&n, "#[announce]"));
        assert!(announced(&n, "#[derive(Announce)]"));
    }

    #[test]
    fn cfg_attr() {
        let n = names(&[]);
        assert!(announced(&n, "#[cfg_attr(test, announce::announce)]"));
        assert!(announced(&n, "#[cfg_attr(test, derive(announce::Announce))]"));
        assert!(announced(&n, "#[cfg_attr(test, cfg_attr(feature = \"x\", derive(Announce)))]"));
        assert!(!announced(&n, "#[cfg_attr(test, derive(other::Announce))]"));
    }

    fn add(attrs: &str) -> String {
        let n = names(&[]);
        let mut item: ItemStruct = syn::parse_str(&format!("{} struct S;", attrs)).unwrap();
        n.add_meta(&mut item.attrs, &quote!(m));
        let attrs = &item.attrs;
        quote!(#(#attrs)*).to_string().replace(' ', "")
    }

    #[test]
    fn add_meta() {
        assert_eq!(add("#[announce]"), "#[announce(meta=m)]");
        assert_eq!(add("#[announce::announce(macro_name = M)]"), "#[announce::announce(macro_name=M,meta=m)]");
        assert_eq!(add("#[derive(Announce)]"), "#[derive(Announce)]#[announce(meta=m)]");
        assert_eq!(
            add("#[cfg_attr(test, cfg_attr(unix, announce))]"),
            "#[cfg_attr(test,cfg_attr(unix,announce(meta=m)))]"
        );
        assert_eq!(
            add("#[cfg_attr(test, cfg_attr(unix, derive(Debug, Announce)))]"),
            "#[cfg_attr(test,cfg_attr(unix,derive(Debug,Announce)))]#[cfg_attr(all(test,unix),announce(meta=m))]"
        );
    }
}

#[cfg(test)]
mod reexport_tests {
    use super::*;

    fn names_in(uses: &[&str], coinduction: Path) -> AnnounceNames {
        let items = uses
            .iter()
            .map(|u| syn::parse_str::<Item>(u).unwrap())
            .collect::<Vec<_>>();
        AnnounceNames::new(&items, &NoArgPath(coinduction))
    }

    fn announced(names: &AnnounceNames, attrs: &str) -> bool {
        let item: ItemStruct = syn::parse_str(&format!("{} struct S;", attrs)).unwrap();
        names.is_announced(&item.attrs)
    }

    fn add(names: &AnnounceNames, attrs: &str) -> String {
        let mut item: ItemStruct = syn::parse_str(&format!("{} struct S;", attrs)).unwrap();
        names.add_meta(&mut item.attrs, &quote!(m));
        let attrs = &item.attrs;
        quote!(#(#attrs)*).to_string().replace(' ', "")
    }

    #[test]
    fn paths() {
        let n = names_in(&[], parse_quote!(::coinduction));
        assert!(announced(&n, "#[coinduction::announce::announce]"));
        assert!(announced(&n, "#[::coinduction::announce::announce]"));
        assert!(announced(&n, "#[derive(coinduction::announce::Announce)]"));
        assert!(!announced(&n, "#[other::announce::announce]"));
        // Through the `coinduction = <path>` argument
        let n = names_in(&[], parse_quote!(crate::reexport));
        assert!(announced(&n, "#[crate::reexport::announce::announce]"));
        assert!(!announced(&n, "#[coinduction::announce::announce]"));
    }

    #[test]
    fn imports() {
        let n = names_in(
            &["use coinduction::announce::{announce as a, Announce as A};"],
            parse_quote!(::coinduction),
        );
        assert!(announced(&n, "#[a]"));
        assert!(announced(&n, "#[derive(A)]"));
        let n = names_in(&["use coinduction::announce as ann;", "use ann::Announce as D;"], parse_quote!(::coinduction));
        assert!(announced(&n, "#[ann::announce]"));
        assert!(announced(&n, "#[derive(ann::Announce)]"));
        assert!(announced(&n, "#[derive(D)]"));
    }

    #[test]
    fn krate() {
        let n = names_in(&["use coinduction::announce::Announce as A;"], parse_quote!(::coinduction));
        assert_eq!(
            add(&n, "#[coinduction::announce::announce]"),
            "#[coinduction::announce::announce(crate=::coinduction::announce,meta=m)]"
        );
        assert_eq!(
            add(&n, "#[coinduction::announce::announce(crate = x)]"),
            "#[coinduction::announce::announce(crate=x,meta=m)]"
        );
        assert_eq!(
            add(&n, "#[derive(A)]"),
            "#[derive(A)]#[announce(crate=::coinduction::announce,meta=m)]"
        );
        // The helper attribute of a derive through the re-export
        assert_eq!(
            add(&n, "#[derive(A)] #[announce(macro_name = M)]"),
            "#[derive(A)]#[announce(macro_name=M,crate=::coinduction::announce,meta=m)]"
        );
        // announce from the crate's own dependency
        assert_eq!(add(&n, "#[announce::announce]"), "#[announce::announce(meta=m)]");
        assert_eq!(add(&n, "#[derive(announce::Announce)]"), "#[derive(announce::Announce)]#[announce(meta=m)]");
    }
}
