use std::hash::Hasher;
use std::path::PathBuf;

pub struct ResolvedStruct {
    pub id: ItemId,
    pub module_path: String,   // "crate::motor::state"
    pub node: syn::ItemStruct, // EVERYTHING — fields, generics,
    pub file: PathBuf,
}

pub struct ResolvedEnum {
    pub id: ItemId,
    pub module_path: String,
    pub node: syn::ItemEnum,
    pub file: PathBuf,
}

pub struct ResolvedFunction {
    pub id: ItemId,
    pub module_path: String,
    pub node: syn::ItemFn,
    pub file: PathBuf,
}

pub struct ResolvedTrait {
    pub id: ItemId,
    pub module_path: String,
    pub node: syn::ItemTrait, // all methods, all signatures
    pub file: PathBuf,
}

pub struct ResolvedImpl {
    pub id: ItemId,
    pub self_ty: ItemRef,        // what type
    pub trait_: Option<ItemRef>, // what trait, if
    pub module_path: String,
    pub node: syn::ItemImpl, // complete impl block
    pub file: PathBuf,
}

pub enum ItemKind {
    Type(syn::Type),
    Path(syn::Path),
}

pub struct ItemRef {
    pub written: String,          // "Vec<ArmJointState>" as written
    pub resolved: Option<ItemId>, // points to known item if resolvable
    pub generics: Vec<ItemRef>,
    pub kind: ItemKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

impl ItemId {
    pub fn from_name(module_path: &str, name: &str) -> Self {
        let full = format!("{}::{}", module_path, name);
        let mut hasher = fnv::FnvHasher::with_key(0);
        hasher.write(full.as_bytes());
        ItemId(hasher.finish())
    }
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub params: usize,
    pub is_async: bool,
    pub is_unsafe: bool,
}

impl ResolvedStruct {
    pub fn impls<'a>(&self, workspace: &'a crate::workspaces::Workspace) -> Vec<&'a ResolvedImpl> {
        workspace.impls_for_struct(self.id)
    }

    pub fn methods<'a>(&self, workspace: &'a crate::workspaces::Workspace) -> Vec<MethodInfo> {
        self.impls(workspace)
            .iter()
            .flat_map(|impl_| {
                impl_.node.items.iter().filter_map(|item| {
                    if let syn::ImplItem::Fn(method) = item {
                        Some(MethodInfo {
                            name: method.sig.ident.to_string(),
                            params: method.sig.inputs.len(),
                            is_async: method.sig.asyncness.is_some(),
                            is_unsafe: method.sig.unsafety.is_some(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    pub fn trait_impls<'a>(
        &self,
        workspace: &'a crate::workspaces::Workspace,
    ) -> Vec<&'a ResolvedTrait> {
        self.impls(workspace)
            .iter()
            .filter_map(|impl_| {
                impl_
                    .trait_
                    .as_ref()
                    .and_then(|t| t.resolved)
                    .and_then(|id| workspace.trait_by_id(id))
            })
            .collect()
    }
}
