use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{
    ItemId, ItemKind, ItemRef, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct,
    ResolvedTrait,
};
use crate::workspaces::Workspace;

// Extract the base type name from a written type string.
// "MotorConfig" -> "MotorConfig", "Vec < T >" -> "Vec", "path :: to :: Foo" -> "Foo"
fn base_name(written: &str) -> &str {
    let without_generics = written.split('<').next().unwrap_or(written).trim();
    without_generics
        .split("::")
        .last()
        .unwrap_or(without_generics)
        .trim()
}

fn convert_function(item: &resolve::Item, f: &syn::ItemFn) -> ResolvedFunction {
    ResolvedFunction {
        id: item.id,
        module_path: item.module_path.clone(),
        node: f.clone(),
        file: item.parent.file_path(),
    }
}

fn convert_enum(item: &resolve::Item, e: &syn::ItemEnum) -> ResolvedEnum {
    ResolvedEnum {
        id: item.id,
        module_path: item.module_path.clone(),
        node: e.clone(),
        file: item.parent.file_path(),
    }
}

fn convert_struct(item: &resolve::Item, s: &syn::ItemStruct) -> ResolvedStruct {
    ResolvedStruct {
        id: item.id,
        module_path: item.module_path.clone(),
        node: s.clone(),
        file: item.parent.file_path(),
    }
}

fn convert_trait(item: &resolve::Item, t: &syn::ItemTrait) -> ResolvedTrait {
    ResolvedTrait {
        id: item.id,
        module_path: item.module_path.clone(),
        node: t.clone(),
        file: item.parent.file_path(),
    }
}

// Impl blocks aren't named in source, so they need a synthetic id -- built
// the same way the old RA-backed bridge built it, from the target type's
// name plus (if present) the implemented trait's base name.
fn convert_impl(impl_: &resolve::Impl, items_by_id: &HashMap<resolve::ItemId, &resolve::Item>) -> ResolvedImpl {
    let self_ty_ast = &impl_.node.self_ty;
    let self_ty_written = quote::quote!(#self_ty_ast).to_string();

    let trait_written = impl_
        .node
        .trait_
        .as_ref()
        .map(|(_, path, _)| quote::quote!(#path).to_string());

    let resolved_self_name = items_by_id
        .get(&impl_.target_type)
        .map(|i| i.name.as_str())
        .unwrap_or("unknown");

    let trait_suffix = trait_written
        .as_deref()
        .map(|tw| format!("__{}", base_name(tw)))
        .unwrap_or_default();

    let id = ItemId::from(
        &impl_.module_path,
        &format!("impl_{resolved_self_name}{trait_suffix}"),
    );

    let self_ty = ItemRef {
        written: self_ty_written,
        kind: ItemKind::Type((*impl_.node.self_ty).clone()),
        resolved: Some(impl_.target_type),
        generics: vec![],
    };

    let trait_ = impl_.node.trait_.as_ref().map(|(_, path, _)| ItemRef {
        written: trait_written.clone().unwrap_or_default(),
        resolved: impl_.trait_,
        generics: vec![],
        kind: ItemKind::Path(path.clone()),
    });

    ResolvedImpl {
        id,
        self_ty,
        trait_,
        module_path: impl_.module_path.clone(),
        node: impl_.node.clone(),
        file: impl_.parent.file_path(),
    }
}

pub(crate) fn build_workspace(db: resolve::Db) -> Workspace {
    let items_by_id: HashMap<resolve::ItemId, &resolve::Item> =
        db.items().map(|item| (item.id, item)).collect();

    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut functions = Vec::new();

    for item in db.items() {
        match &item.kind {
            resolve::ItemKind::Fn(f) => functions.push(convert_function(item, f)),
            resolve::ItemKind::Enum(e) => enums.push(convert_enum(item, e)),
            resolve::ItemKind::Struct(s) => structs.push(convert_struct(item, s)),
            resolve::ItemKind::Trait(t) => traits.push(convert_trait(item, t)),
            // Consts, nested modules-as-items, and unhandled item kinds
            // don't have a `Resolved*` counterpart -- matches the RA-backed
            // bridge this replaces, which never walked those either.
            resolve::ItemKind::Const(_) | resolve::ItemKind::Module(_) | resolve::ItemKind::Unhandled => {}
        }
    }

    let impls: Vec<ResolvedImpl> = db
        .impls()
        .iter()
        .map(|impl_| convert_impl(impl_, &items_by_id))
        .collect();

    let files: Vec<PathBuf> = structs
        .iter()
        .map(|s| s.file.clone())
        .chain(enums.iter().map(|e| e.file.clone()))
        .chain(traits.iter().map(|t| t.file.clone()))
        .chain(impls.iter().map(|i| i.file.clone()))
        .chain(functions.iter().map(|f| f.file.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Workspace::new(structs, enums, traits, impls, files, functions)
}
