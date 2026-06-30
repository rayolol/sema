use std::collections::HashMap;
use std::path::PathBuf;

use crate::analyse::RawItems;
use crate::internal::{Function, Module, RootDatabase, Struct};
use crate::types::{
    ItemId, ItemKind, ItemRef, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct,
    ResolvedTrait,
};
use crate::workspaces::Workspace;
use ra_ap_hir::{Enum, HasSource, Impl, Trait};
use ra_ap_syntax::AstNode;
use ra_ap_vfs::Vfs;

fn convert_function(f: Function, db: &RootDatabase, vfs: &Vfs) -> Option<ResolvedFunction> {
    let source = f.source(db)?;

    let file = source.file_id.file_id().and_then(|fid| {
        vfs.file_path(fid.file_id(db))
            .as_path()
            .map(|p| p.to_path_buf())
    })?;

    let file = PathBuf::from(file);

    let text = source.value.syntax().text().to_string();
    let node: syn::ItemFn = syn::parse_str(&text).ok()?;

    let module_path = module_path_string(f.module(db), db);
    let id = ItemId::from_name(&module_path, f.name(db).as_str());

    Some(ResolvedFunction {
        id,
        module_path,
        node,
        file,
    })
}

fn convert_enum(e: Enum, db: &RootDatabase, vfs: &Vfs) -> Option<ResolvedEnum> {
    let source = e.source(db)?;

    let file = source.file_id.file_id().and_then(|fid| {
        vfs.file_path(fid.file_id(db))
            .as_path()
            .map(|p| p.to_path_buf())
    })?;

    let file: PathBuf = PathBuf::from(file);

    let text = source.value.syntax().text().to_string();
    let node: syn::ItemEnum = syn::parse_str(&text).ok()?;

    let module_path = module_path_string(e.module(db), db);
    let id = ItemId::from_name(&module_path, e.name(db).as_str());

    Some(ResolvedEnum {
        id,
        module_path,
        node,
        file,
    })
}

fn convert_trait(t: Trait, db: &RootDatabase, vfs: &Vfs) -> Option<ResolvedTrait> {
    let source = t.source(db)?;

    let file = source.file_id.file_id().and_then(|fid| {
        vfs.file_path(fid.file_id(db))
            .as_path()
            .map(|p| p.to_path_buf())
    })?;

    let file: PathBuf = PathBuf::from(file);

    let text = source.value.syntax().text().to_string();
    let node: syn::ItemTrait = syn::parse_str(&text).ok()?;
    let module_path = module_path_string(t.module(db), db);
    let id = ItemId::from_name(&module_path, t.name(db).as_str());

    Some(ResolvedTrait {
        id,
        module_path,
        node,
        file,
    })
}

// Extract the base type name from a written type string.
// "MotorConfig" → "MotorConfig", "Vec<T>" → "Vec", "path::to::Foo" → "Foo"
fn base_name(written: &str) -> &str {
    let without_generics = written.split('<').next().unwrap_or(written).trim();
    without_generics.split("::").last().unwrap_or(without_generics)
}

fn convert_impl(
    i: Impl,
    db: &RootDatabase,
    vfs: &Vfs,
    struct_ids: &HashMap<String, ItemId>,
    enum_ids: &HashMap<String, ItemId>,
    trait_ids: &HashMap<String, ItemId>,
) -> Option<ResolvedImpl> {
    let source = i.source(db)?;

    let file = source.file_id.file_id().and_then(|fid| {
        vfs.file_path(fid.file_id(db))
            .as_path()
            .map(|p| p.to_path_buf())
    })?;

    let file: PathBuf = PathBuf::from(file);

    let self_ty_written = source
        .value
        .self_ty()
        .map(|ty| ty.syntax().text().to_string())
        .unwrap_or_else(|| "?".to_string());

    let trait_written = source
        .value
        .trait_()
        .map(|ty| ty.syntax().text().to_string());

    let text = source.value.syntax().text().to_string();
    let node: syn::ItemImpl = syn::parse_str(&text).ok()?;

    let module_path = module_path_string(i.module(db), db);

    // Resolve self_ty by matching the written base name against known structs/enums.
    // Structs take priority; falls back to enums.
    let self_base = base_name(&self_ty_written);
    let resolved_self = struct_ids
        .get(self_base)
        .or_else(|| enum_ids.get(self_base))
        .copied();

    // Resolve trait by matching its base name against known traits.
    let resolved_trait = trait_written.as_deref().and_then(|tw| {
        trait_ids.get(base_name(tw)).copied()
    });

    let trait_suffix = trait_written
        .as_deref()
        .map(|tw| format!("__{}", base_name(tw)))
        .unwrap_or_default();

    let item_id = ItemId::from_name(
        &module_path,
        &format!("impl_{}{}", self_base, trait_suffix),
    );

    let self_ty = ItemRef {
        written: self_ty_written,
        kind: ItemKind::Type(*node.self_ty.clone()),
        resolved: resolved_self,
        generics: vec![],
    };

    let trait_ = if let Some((_token, path, _)) = &node.trait_ {
        Some(ItemRef {
            written: trait_written.unwrap_or_default(),
            resolved: resolved_trait,
            generics: vec![],
            kind: ItemKind::Path(path.clone()),
        })
    } else {
        None
    };

    Some(ResolvedImpl {
        id: item_id,
        self_ty,
        module_path,
        node,
        file,
        trait_,
    })
}

pub(crate) fn build_workspace(raw: RawItems, db: &RootDatabase, vfs: &Vfs) -> Workspace {
    let structs: Vec<ResolvedStruct> = raw
        .structs
        .into_iter()
        .filter_map(|s| convert_struct(s, db, vfs))
        .collect();

    let enums: Vec<ResolvedEnum> = raw
        .enums
        .into_iter()
        .filter_map(|e| convert_enum(e, db, vfs))
        .collect();

    let traits: Vec<ResolvedTrait> = raw
        .traits
        .into_iter()
        .filter_map(|t| convert_trait(t, db, vfs))
        .collect();

    // Build name→id maps so convert_impl can resolve types without HIR type inference
    let struct_ids: HashMap<String, ItemId> = structs
        .iter()
        .map(|s| (s.node.ident.to_string(), s.id))
        .collect();

    let enum_ids: HashMap<String, ItemId> = enums
        .iter()
        .map(|e| (e.node.ident.to_string(), e.id))
        .collect();

    let trait_ids: HashMap<String, ItemId> = traits
        .iter()
        .map(|t| (t.node.ident.to_string(), t.id))
        .collect();

    let impls: Vec<ResolvedImpl> = raw
        .impls
        .into_iter()
        .filter_map(|i| convert_impl(i, db, vfs, &struct_ids, &enum_ids, &trait_ids))
        .collect();

    let files: Vec<PathBuf> = structs
        .iter()
        .map(|s| s.file.clone())
        .chain(enums.iter().map(|e| e.file.clone()))
        .chain(traits.iter().map(|t| t.file.clone()))
        .chain(impls.iter().map(|i| i.file.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let functions: Vec<ResolvedFunction> = raw
        .functions
        .into_iter()
        .filter_map(|f| convert_function(f, db, vfs))
        .collect();

    Workspace::new(structs, enums, traits, impls, files, functions)
}

fn module_path_string(module: Module, db: &RootDatabase) -> String {
    module
        .path_to_root(db)
        .into_iter()
        .rev()
        .filter_map(|m| m.name(db))
        .map(|n| n.as_str().to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn convert_struct(s: Struct, db: &RootDatabase, vfs: &Vfs) -> Option<ResolvedStruct> {
    let source = s.source(db)?;

    let file = source.file_id.file_id().and_then(|fid| {
        vfs.file_path(fid.file_id(db))
            .as_path()
            .map(|p| p.to_path_buf())
    })?;

    let file: PathBuf = PathBuf::from(file);

    let text = source.value.syntax().text().to_string();
    let node: syn::ItemStruct = syn::parse_str(&text).ok()?;

    let module_path = module_path_string(s.module(db), db);
    let id = ItemId::from_name(&module_path, &node.ident.to_string());

    Some(ResolvedStruct {
        id,
        module_path,
        node,
        file,
    })
}
