use crate::ids::{ItemId, ModuleId};

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub id: ItemId,
    pub kind: ItemKind,
    pub is_pub: bool,
    pub parent: ModuleId,
    pub module_path: String,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Fn(syn::ItemFn),
    Enum(syn::ItemEnum),
    Struct(syn::ItemStruct),
    Trait(syn::ItemTrait),
    Const(syn::ItemConst),
    Module(syn::ItemMod),
    Unhandled,
}

// `syn::Item::Impl` is intercepted before it reaches this function -- impls
// aren't name-addressable items, they attach methods to a type, so they're
// tracked separately as `RawImpl`/`Impl` (see db.rs).
pub(crate) fn lower_items(item: &syn::Item, parent: ModuleId, module_path: &str) -> Item {
    match item {
        syn::Item::Fn(f) => Item {
            name: f.sig.ident.to_string(),
            id: ItemId::from(module_path, &f.sig.ident.to_string()),
            is_pub: matches!(f.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Fn(f.clone()),
        },
        syn::Item::Enum(e) => Item {
            name: e.ident.to_string(),
            id: ItemId::from(module_path, &e.ident.to_string()),
            is_pub: matches!(e.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Enum(e.clone()),
        },
        syn::Item::Struct(s) => Item {
            name: s.ident.to_string(),
            id: ItemId::from(module_path, &s.ident.to_string()),
            is_pub: matches!(s.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Struct(s.clone()),
        },
        syn::Item::Trait(t) => Item {
            name: t.ident.to_string(),
            id: ItemId::from(module_path, &t.ident.to_string()),
            is_pub: matches!(t.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Trait(t.clone()),
        },
        syn::Item::Const(c) => Item {
            name: c.ident.to_string(),
            id: ItemId::from(module_path, &c.ident.to_string()),
            is_pub: matches!(c.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Const(c.clone()),
        },
        syn::Item::Mod(m) => Item {
            name: m.ident.to_string(),
            id: ItemId::from(module_path, &m.ident.to_string()),
            is_pub: matches!(m.vis, syn::Visibility::Public(_)),
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Module(m.clone()),
        },
        _ => Item {
            name: "unhandled".to_string(),
            id: ItemId::from(module_path, "unhandled"),
            is_pub: false,
            parent,
            module_path: module_path.to_string(),
            kind: ItemKind::Unhandled,
        },
    }
}
