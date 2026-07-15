use crate::query::{Query, SemaItem};
use crate::types::{
    ItemId, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct, ResolvedTrait,
};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Workspace {
    structs: Vec<ResolvedStruct>,
    enums: Vec<ResolvedEnum>,
    traits: Vec<ResolvedTrait>,
    impls: Vec<ResolvedImpl>,
    files: Vec<PathBuf>,
    functions: Vec<ResolvedFunction>,

    struct_by_id: HashMap<ItemId, usize>,
    trait_by_id: HashMap<ItemId, usize>,
    impls_by_self_ty: HashMap<ItemId, Vec<usize>>,
    impls_by_trait: HashMap<ItemId, Vec<usize>>,
}

impl Workspace {
    pub(crate) fn new(
        structs: Vec<ResolvedStruct>,
        enums: Vec<ResolvedEnum>,
        traits: Vec<ResolvedTrait>,
        impls: Vec<ResolvedImpl>,
        files: Vec<PathBuf>,
        functions: Vec<ResolvedFunction>,
    ) -> Self {
        let mut struct_by_id = HashMap::new();
        let mut trait_by_id = HashMap::new();
        let mut impls_by_self_ty = HashMap::new();
        let mut impls_by_trait = HashMap::new();

        for (idx, s) in structs.iter().enumerate() {
            struct_by_id.insert(s.id, idx);
        }

        for (idx, t) in traits.iter().enumerate() {
            trait_by_id.insert(t.id, idx);
        }

        for (idx, i) in impls.iter().enumerate() {
            if let Some(id) = i.self_ty.resolved {
                impls_by_self_ty
                    .entry(id)
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
            if let Some(id) = i.trait_.as_ref().and_then(|t| t.resolved) {
                impls_by_trait.entry(id).or_insert_with(Vec::new).push(idx);
            }
        }

        Self {
            structs,
            enums,
            functions,
            traits,
            impls,
            files,
            struct_by_id,
            trait_by_id,
            impls_by_self_ty,
            impls_by_trait,
        }
    }
    pub fn all_items(&self) -> Query<'_, dyn SemaItem> {
        let iter = self
            .structs
            .iter()
            .map(|s| s as &dyn SemaItem)
            .chain(self.enums.iter().map(|e| e as &dyn SemaItem))
            .chain(self.traits.iter().map(|t| t as &dyn SemaItem))
            .chain(self.impls.iter().map(|i| i as &dyn SemaItem));

        Query::new(iter)
    }

    pub fn by_id(&self, id: ItemId) -> Option<&dyn SemaItem> {
        self.all_items().into_iter().find(|f| f.id() == id)
    }

    pub fn functions(&self) -> Query<'_, ResolvedFunction> {
        Query::new(self.functions.iter())
    }

    pub fn structs(&self) -> Query<'_, ResolvedStruct> {
        Query::new(self.structs.iter())
    }
    pub fn enums(&self) -> Query<'_, ResolvedEnum> {
        Query::new(self.enums.iter())
    }
    pub fn traits(&self) -> Query<'_, ResolvedTrait> {
        Query::new(self.traits.iter())
    }
    pub fn impls(&self) -> Query<'_, ResolvedImpl> {
        Query::new(self.impls.iter())
    }
    pub fn emit_rerun_directives(&self) {
        println!("cargo:rerun-if-changed=Cargo.toml");
        for file in &self.files {
            println!("cargo:rerun-if-changed={}", file.display());
        }
    }

    pub fn impls_for_struct(&self, struct_id: ItemId) -> Vec<&ResolvedImpl> {
        self.impls_by_self_ty
            .get(&struct_id)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.impls.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn impls_of_trait(&self, trait_id: ItemId) -> Vec<&ResolvedImpl> {
        self.impls_by_trait
            .get(&trait_id)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.impls.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn trait_by_id(&self, id: ItemId) -> Option<&ResolvedTrait> {
        self.trait_by_id
            .get(&id)
            .and_then(|&idx| self.traits.get(idx))
    }

    pub fn struct_by_id(&self, id: ItemId) -> Option<&ResolvedStruct> {
        self.struct_by_id
            .get(&id)
            .and_then(|&idx| self.structs.get(idx))
    }

    /// True if any impl block targets this id as its self type (struct or enum).
    pub fn has_impls_for(&self, id: ItemId) -> bool {
        self.impls_by_self_ty.contains_key(&id)
    }

    /// True if at least one impl block implements this trait.
    pub fn has_implementors(&self, trait_id: ItemId) -> bool {
        self.impls_by_trait.contains_key(&trait_id)
    }
}
