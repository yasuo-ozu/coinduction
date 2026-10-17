//! The announcements of the items of a `#[coinduction]` module: how the module names announce's
//! attribute macro and derive, and adding the impls of a type to its announcement.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::NoArgPath;

/// How the module names announce's crate, attribute macro and derive.
///
/// A path names the attribute macro (the derive) when it is `<announce>::announce`
/// (`<announce>::Announce`), `<announce>` being the name of the announce dependency in the crate's
/// `Cargo.toml` (renames included) or a name the module imports `<coinduction>::announce` as, or
/// when it is `<coinduction>::announce::announce` (`::Announce`). A single name does when the
/// module imports it from there, also under another name, or when the module does not import that
/// name from another crate (it may come from a glob import or from an outer module). Attributes
/// inside `#[cfg_attr(..)]` count too.
pub struct AnnounceNames {
    coinduction: NoArgPath,
    /// `(item, name, via)`: `item` is `crate`, `announce` or `Announce`, and `via` whether it is
    /// reached through the re-export `<coinduction>::announce`. The generated code of such an
    /// announcement is given `crate = <coinduction>::announce`, so that the crate need not depend
    /// on announce.
    names: Vec<(String, Ident, bool)>,
}

/// The imports of `tree`: the imported path, and the name it is imported as.
fn imports(tree: &UseTree, prefix: &mut Vec<Ident>, out: &mut Vec<(Vec<Ident>, Ident)>) {
    let mut import = |ident: &Ident, name: &Ident| {
        out.push((
            [&prefix[..], std::slice::from_ref(ident)].concat(),
            name.clone(),
        ))
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
                ("Announce".into(), Ident::new("Announce", span), false),
            ],
        };
        let mut uses = Vec::new();
        for item in items {
            if let Item::Use(item_use) = item {
                imports(&item_use.tree, &mut Vec::new(), &mut uses);
            }
        }
        // The names of the crate first: `use coinduction::announce as ann; use ann::Announce;`
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
            let announce = item == "announce" || item == "Announce";
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

    fn lookup(&self, item: &str, name: &Ident) -> Option<bool> {
        self.names
            .iter()
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

    /// Whether `path` names `item` (`announce` or `Announce`), and whether through the re-export.
    fn names(&self, path: &Path, item: &str) -> Option<bool> {
        let segments = path
            .segments
            .iter()
            .map(|seg| seg.ident.clone())
            .collect::<Vec<_>>();
        match &segments[..] {
            [name] if path.leading_colon.is_none() => self.lookup(item, name),
            [krate, last] if last == item => self.lookup("crate", krate),
            _ => self.is_reexport(&segments, &[item]).then_some(true),
        }
    }

    /// The `#[announce]` attribute (`item` = `announce`) or `#[derive(Announce)]` (`Announce`) in
    /// `meta`: the predicates of the `cfg_attr`s around it, and whether it is reached through the
    /// re-export.
    fn find(&self, meta: &Meta, item: &str, mut cfgs: Vec<Meta>) -> Option<(Vec<Meta>, bool)> {
        let via = match item {
            "announce" => self.names(meta.path(), item),
            _ if meta.path().is_ident("derive") => meta
                .require_list()
                .ok()?
                .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()?
                .iter()
                .find_map(|path| self.names(path, item)),
            _ => None,
        };
        if let Some(via) = via {
            return Some((cfgs, via));
        }
        let (predicate, metas) = cfg_attr_parts(meta)?;
        cfgs.push(predicate);
        metas
            .iter()
            .find_map(|meta| self.find(meta, item, cfgs.clone()))
    }

    /// Whether the item is announced: it has `#[announce]`, or derives `Announce`.
    pub fn is_announced(&self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| {
            ["announce", "Announce"]
                .iter()
                .any(|item| self.find(&attr.meta, item, Vec::new()).is_some())
        })
    }

    /// Add `meta = <add>` to the options of the announcement of an item: to its `#[announce(...)]`
    /// attribute, or, for `#[derive(Announce)]` without one, in a new helper attribute under the
    /// same `cfg_attr`s as the derive. An announcement through the re-export also gets
    /// `crate = <coinduction>::announce`, unless it has a `crate` option; with no `add`, that is
    /// the only change.
    pub fn add_meta(&self, attrs: &mut Vec<Attribute>, add: Option<&TokenStream>) {
        let derive = attrs
            .iter()
            .find_map(|attr| self.find(&attr.meta, "Announce", Vec::new()));
        let via = derive.as_ref().map(|(_, via)| *via);
        if attrs
            .iter_mut()
            .any(|attr| self.add_meta_to(&mut attr.meta, add, via))
            || add.is_none()
        {
            return;
        }
        let (cfgs, via) = derive.unwrap_or_default();
        let mut helper = parse_quote!(announce);
        self.add_meta_to(&mut helper, add, Some(via));
        attrs.push(if cfgs.is_empty() {
            parse_quote!(#[#helper])
        } else {
            parse_quote!(#[cfg_attr(all(#(#cfgs),*), #helper)])
        });
    }

    /// `derive`: whether the derive of the item is reached through the re-export, if it derives
    /// `Announce`; its helper attribute is `announce` under any name of the derive.
    fn add_meta_to(
        &self,
        meta: &mut Meta,
        add: Option<&TokenStream>,
        derive: Option<bool>,
    ) -> bool {
        let via = match derive {
            Some(via) if meta.path().is_ident("announce") => Some(via),
            _ => self.names(meta.path(), "announce"),
        };
        let Some(via) = via else {
            let Some((predicate, mut metas)) = cfg_attr_parts(meta) else {
                return false;
            };
            let added = metas
                .iter_mut()
                .any(|meta| self.add_meta_to(meta, add, derive));
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
            "#[derive(Debug, Announce)]",
            "#[derive(announce::Announce)]",
            "#[coinduction::announce::announce]",
            "#[::coinduction::announce::announce]",
            "#[derive(coinduction::announce::Announce)]",
        ] {
            assert!(announced(&n, attrs), "{}", attrs);
        }
        for attrs in [
            "#[my::announce]",
            "#[a::announce::announce]",
            "#[announce::other]",
            "#[other::announce::announce]",
            "#[derive(other::Announce)]",
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
            "use announce::{Announce as Ann};",
            "use coinduction::announce::{announce as a, Announce as A};",
            "use coinduction::announce as reexport;",
            "use reexport::Announce as D;",
        ]);
        for attrs in [
            "#[ann]",
            "#[derive(Ann)]",
            "#[a]",
            "#[derive(A)]",
            "#[reexport::announce]",
        ] {
            assert!(announced(&n, attrs), "{}", attrs);
        }
        assert!(announced(&n, "#[derive(reexport::Announce)]"));
        assert!(announced(&n, "#[derive(D)]"));
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
        assert!(announced(
            &n,
            "#[cfg_attr(test, derive(announce::Announce))]"
        ));
        assert!(announced(
            &n,
            "#[cfg_attr(test, cfg_attr(feature = \"x\", derive(Announce)))]"
        ));
        assert!(!announced(&n, "#[cfg_attr(test, derive(other::Announce))]"));
    }

    #[test]
    fn add_meta() {
        let n = names(&["use coinduction::announce::Announce as A;"]);
        for (attrs, expected) in [
            ("#[announce]", "#[announce(meta=m,)]"),
            ("#[announce::announce]", "#[announce::announce(meta=m,)]"),
            ("#[announce::announce(macro_name = M)]", "#[announce::announce(meta=m,macro_name=M)]"),
            ("#[derive(Announce)]", "#[derive(Announce)]#[announce(meta=m,)]"),
            (
                "#[cfg_attr(test, cfg_attr(unix, announce))]",
                "#[cfg_attr(test,cfg_attr(unix,announce(meta=m,)))]",
            ),
            (
                "#[cfg_attr(test, cfg_attr(unix, derive(Debug, Announce)))]",
                "#[cfg_attr(test,cfg_attr(unix,derive(Debug,Announce)))]#[cfg_attr(all(test,unix),announce(meta=m,))]",
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
            ("#[derive(A)]", "#[derive(A)]#[announce(crate=::coinduction::announce,meta=m,)]"),
            (
                "#[derive(A)] #[announce(macro_name = M)]",
                "#[derive(A)]#[announce(crate=::coinduction::announce,meta=m,macro_name=M)]",
            ),
        ] {
            assert_eq!(add(&n, attrs), expected);
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
