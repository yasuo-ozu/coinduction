//! The announcements of the items of a `#[coinduction]` module: how the module names announce's
//! attribute macro, and adding the impls of a type to its announcement.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::NoArgPath;

/// How the module names announce's crate and attribute macro.
///
/// A path names the attribute macro when it is `<announce>::announce`, `<announce>` being the name
/// of the announce dependency in the crate's `Cargo.toml` (renames included) or a name the module
/// imports `<coinduction>::announce` as, or when it is `<coinduction>::announce::announce`. A
/// single name does when the module imports it from there, also under another name, or when the
/// module does not import that name from another crate (it may come from a glob import or from an
/// outer module). Attributes inside `#[cfg_attr(..)]` count too.
pub struct AnnounceNames {
    coinduction: NoArgPath,
    /// `(item, name, via)`: `item` is `crate` or `announce`, and `via` whether it is
    /// reached through the re-export `<coinduction>::announce`. The generated code of such an
    /// announcement is given `crate = <coinduction>::announce`, so that the crate need not depend
    /// on announce.
    names: Vec<(String, Ident, bool)>,
}

/// The imports of `tree`: the imported path, and the name it is imported as.
fn imports(tree: &UseTree, prefix: &mut Vec<Ident>, out: &mut Vec<(Vec<Ident>, Ident)>) {
    let mut import = |ident: &Ident, name: &Ident| {
        // `{self}` and `{self as name}` import the module itself
        let (path, name) = match prefix.last() {
            Some(last) if ident == "self" => {
                (prefix.to_vec(), if name == "self" { last } else { name })
            }
            _ => ([&prefix[..], std::slice::from_ref(ident)].concat(), name),
        };
        out.push((path, name.clone()))
    };
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            prefix.push(ident.clone());
            imports(tree, prefix, out);
            prefix.pop();
        }
        UseTree::Name(UseName { ident }) => import(ident, ident),
        UseTree::Rename(UseRename { ident, rename, .. }) => import(ident, rename),
        UseTree::Group(UseGroup { items, .. }) => {
            items.iter().for_each(|tree| imports(tree, prefix, out))
        }
        UseTree::Glob(_) => {}
    }
}

impl AnnounceNames {
    pub(crate) fn new(items: &[Item], coinduction: &NoArgPath) -> Self {
        let krate = announce::machinery::Options::default().krate();
        let span = proc_macro2::Span::call_site();
        let mut this = AnnounceNames {
            coinduction: coinduction.clone(),
            names: vec![
                (
                    "crate".into(),
                    krate.segments.last().unwrap().ident.clone(),
                    false,
                ),
                ("announce".into(), Ident::new("announce", span), false),
            ],
        };
        let mut uses = Vec::new();
        for item in items {
            if let Item::Use(item_use) = item {
                imports(&item_use.tree, &mut Vec::new(), &mut uses);
            }
        }
        // The names of the crate first: `use coinduction::announce as ann; use ann::announce;`
        for (path, name) in &uses {
            if this.is_reexport(path, &[]) {
                this.names.push(("crate".into(), name.clone(), true));
            }
        }
        let mut foreign = Vec::new();
        for (path, name) in &uses {
            let (item, prefix) = path.split_last().unwrap();
            let item = item.to_string();
            let from = match prefix {
                _ if this.is_reexport(prefix, &[]) => Some(true),
                [krate] => this.lookup("crate", krate),
                _ => None,
            };
            let announce = item == "announce";
            match from {
                Some(via) if announce => this.names.push((item, name.clone(), via)),
                _ if this.is_reexport(path, &[]) => {}
                _ if prefix.first().is_some_and(|first| {
                    first != "crate" && first != "self" && first != "super"
                }) =>
                {
                    foreign.push(name.clone())
                }
                // A re-export in this crate: it may be announce's
                _ if announce => this.names.push((item, name.clone(), false)),
                _ => {}
            }
        }
        this.names
            .retain(|(item, name, _)| item == "crate" || !foreign.contains(name));
        this
    }

    /// The last entry wins: an import shadows the defaults, as it shadows the extern prelude.
    fn lookup(&self, item: &str, name: &Ident) -> Option<bool> {
        self.names
            .iter()
            .rev()
            .find(|(i, n, _)| i == item && n == name)
            .map(|(_, _, via)| *via)
    }

    /// Whether `path` is `<coinduction>::announce` followed by `rest`.
    fn is_reexport(&self, path: &[Ident], rest: &[&str]) -> bool {
        let expected = self
            .coinduction
            .0
            .segments
            .iter()
            .map(|seg| seg.ident.to_string());
        let expected = expected.chain(["announce"].iter().chain(rest).map(|s| s.to_string()));
        path.iter().map(|ident| ident.to_string()).eq(expected)
    }

    /// Whether `path` names the attribute macro, and whether through the re-export.
    fn names(&self, path: &Path) -> Option<bool> {
        let segments = path
            .segments
            .iter()
            .map(|seg| seg.ident.clone())
            .collect::<Vec<_>>();
        match &segments[..] {
            [name] if path.leading_colon.is_none() => self.lookup("announce", name),
            [krate, last] if last == "announce" => self.lookup("crate", krate),
            _ => self.is_reexport(&segments, &["announce"]).then_some(true),
        }
    }

    /// Whether `meta` is the `#[announce]` attribute, also inside `cfg_attr`s.
    fn find(&self, meta: &Meta) -> bool {
        self.names(meta.path()).is_some()
            || cfg_attr_parts(meta).is_some_and(|(_, metas)| metas.iter().any(|m| self.find(m)))
    }

    /// Whether the item is announced: it has `#[announce]`.
    pub fn is_announced(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| self.find(&attr.meta))
    }

    /// Add `meta = <add>` to the options of the first `#[announce]` attribute of an item. An
    /// announcement through the re-export also gets `crate = <coinduction>::announce`, unless it
    /// has a `crate` option; with no `add`, that is the only change.
    pub fn add_meta(&self, attrs: &mut [Attribute], add: Option<&TokenStream>) {
        attrs
            .iter_mut()
            .any(|attr| self.add_meta_to(&mut attr.meta, add));
    }

    fn add_meta_to(&self, meta: &mut Meta, add: Option<&TokenStream>) -> bool {
        let Some(via) = self.names(meta.path()) else {
            let Some((predicate, mut metas)) = cfg_attr_parts(meta) else {
                return false;
            };
            let added = metas.iter_mut().any(|meta| self.add_meta_to(meta, add));
            if added {
                *meta = parse_quote!(cfg_attr(#predicate, #metas));
            }
            return added;
        };
        let options = match &*meta {
            Meta::Path(_) => TokenStream::new(),
            Meta::List(list) => list.tokens.clone(),
            Meta::NameValue(_) => abort!(meta, "expected `#[announce]` or `#[announce(...)]`"),
        };
        let has_krate = parse2::<announce::machinery::Options>(options.clone())
            .is_ok_and(|options| options.krate.is_some());
        let coinduction = &self.coinduction;
        let krate = (via && !has_krate).then(|| quote!(crate = #coinduction::announce,));
        if krate.is_some() || add.is_some() {
            let path = meta.path().clone();
            let add = add.map(|add| quote!(meta = #add,));
            *meta = parse_quote!(#path(#krate #add #options));
        }
        true
    }
}

/// The predicate and the attributes of `cfg_attr(<predicate>, <attributes>)`.
fn cfg_attr_parts(meta: &Meta) -> Option<(Meta, Punctuated<Meta, Token![,]>)> {
    if !meta.path().is_ident("cfg_attr") {
        return None;
    }
    meta.require_list()
        .ok()?
        .parse_args_with(|input: ParseStream| {
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
        names_in(uses, parse_quote!(::coinduction))
    }

    fn names_in(uses: &[&str], coinduction: Path) -> AnnounceNames {
        let items = uses
            .iter()
            .map(|u| parse_str(u).unwrap())
            .collect::<Vec<Item>>();
        AnnounceNames::new(&items, &NoArgPath(coinduction))
    }

    fn item(attrs: &str) -> ItemStruct {
        parse_str(&format!("{} struct S;", attrs)).unwrap()
    }

    fn announced(names: &AnnounceNames, attrs: &str) -> bool {
        names.is_announced(&item(attrs).attrs)
    }

    fn add(names: &AnnounceNames, attrs: &str) -> String {
        add_some(names, attrs, Some(&quote!(m)))
    }

    fn add_some(names: &AnnounceNames, attrs: &str, add: Option<&TokenStream>) -> String {
        let mut item = item(attrs);
        names.add_meta(&mut item.attrs, add);
        let attrs = &item.attrs;
        quote!(#(#attrs)*).to_string().replace(' ', "")
    }

    #[test]
    fn paths() {
        let n = names(&[]);
        for attrs in [
            "#[announce]",
            "#[announce(macro_name = M)]",
            "#[announce::announce]",
            "#[::announce::announce]",
            "#[coinduction::announce::announce]",
            "#[::coinduction::announce::announce]",
        ] {
            assert!(announced(&n, attrs), "{}", attrs);
        }
        for attrs in [
            "#[my::announce]",
            "#[a::announce::announce]",
            "#[announce::other]",
            "#[other::announce::announce]",
            "#[derive(Debug)]",
        ] {
            assert!(!announced(&n, attrs), "{}", attrs);
        }
        // Through the `coinduction = <path>` argument
        let n = names_in(&[], parse_quote!(crate::reexport));
        assert!(announced(&n, "#[crate::reexport::announce::announce]"));
        assert!(!announced(&n, "#[coinduction::announce::announce]"));
    }

    #[test]
    fn imports() {
        let n = names(&[
            "use announce::announce as ann;",
            "use coinduction::announce::{announce as a};",
            "use coinduction::announce as reexport;",
            "use reexport::announce as r;",
        ]);
        for attrs in ["#[ann]", "#[a]", "#[reexport::announce]", "#[r]"] {
            assert!(announced(&n, attrs), "{}", attrs);
        }
        // Imported from another crate
        let n = names(&["use my_attrs::announce;"]);
        assert!(!announced(&n, "#[announce]"));
        assert!(announced(&n, "#[announce::announce]"));
        // Re-exported in this crate, or from a glob import: may be announce's
        let n = names(&["use super::announce;", "use crate::*;"]);
        assert!(announced(&n, "#[announce]"));
    }

    #[test]
    fn cfg_attr() {
        let n = names(&[]);
        assert!(announced(&n, "#[cfg_attr(test, announce::announce)]"));
        assert!(announced(&n, "#[cfg_attr(test, derive(Debug), announce)]"));
        assert!(announced(
            &n,
            "#[cfg_attr(test, cfg_attr(feature = \"x\", announce))]"
        ));
        assert!(!announced(&n, "#[cfg_attr(test, other::announce)]"));
    }

    #[test]
    fn add_meta() {
        let n = names(&["use coinduction::announce::announce as a;"]);
        for (attrs, expected) in [
            ("#[announce]", "#[announce(meta=m,)]"),
            ("#[announce::announce]", "#[announce::announce(meta=m,)]"),
            (
                "#[announce::announce(macro_name = M)]",
                "#[announce::announce(meta=m,macro_name=M)]",
            ),
            (
                "#[cfg_attr(test, cfg_attr(unix, announce))]",
                "#[cfg_attr(test,cfg_attr(unix,announce(meta=m,)))]",
            ),
            // The first announcement only
            (
                "#[announce] #[announce(allow_primitive = true)]",
                "#[announce(meta=m,)]#[announce(allow_primitive=true)]",
            ),
            // Through the re-export: `crate = ..` unless given
            (
                "#[coinduction::announce::announce]",
                "#[coinduction::announce::announce(crate=::coinduction::announce,meta=m,)]",
            ),
            (
                "#[coinduction::announce::announce(crate = x)]",
                "#[coinduction::announce::announce(meta=m,crate=x)]",
            ),
            ("#[a]", "#[a(crate=::coinduction::announce,meta=m,)]"),
        ] {
            assert_eq!(add(&n, attrs), expected);
        }
        // Imported from the re-export under the names of the defaults
        let own = names(&["use coinduction::announce::announce;"]);
        let krate = names(&["use coinduction::announce;"]);
        let krate_self = names(&["use coinduction::announce::{self};"]);
        let alias_self = names(&["use coinduction::{announce::{self as ann}};"]);
        for (names, attrs, expected) in [
            (
                &own,
                "#[announce]",
                "#[announce(crate=::coinduction::announce,meta=m,)]",
            ),
            (
                &krate,
                "#[announce::announce]",
                "#[announce::announce(crate=::coinduction::announce,meta=m,)]",
            ),
            (
                &krate_self,
                "#[announce::announce]",
                "#[announce::announce(crate=::coinduction::announce,meta=m,)]",
            ),
            (
                &alias_self,
                "#[ann::announce]",
                "#[ann::announce(crate=::coinduction::announce,meta=m,)]",
            ),
        ] {
            assert_eq!(add(names, attrs), expected);
        }
        // Without `meta`: only `crate = ..` through the re-export
        for (attrs, expected) in [
            ("#[announce]", "#[announce]"),
            (
                "#[coinduction::announce::announce]",
                "#[coinduction::announce::announce(crate=::coinduction::announce,)]",
            ),
            (
                "#[coinduction::announce::announce(crate = x)]",
                "#[coinduction::announce::announce(crate=x)]",
            ),
        ] {
            assert_eq!(add_some(&n, attrs, None), expected);
        }
    }
}
