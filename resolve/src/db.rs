use std::collections::HashMap;
use std::path::Path;

use crate::ids::{ImplId, ItemId, ModuleId};
use crate::item::{Item, ItemKind};

pub struct Impl {
    pub target_type: ItemId,
    pub trait_: Option<ItemId>,
    pub methods: Vec<String>,
    pub node: syn::ItemImpl,
    pub module_path: String,
    pub parent: ModuleId,
}

pub(crate) struct RawImpl {
    pub(crate) target_path: Vec<String>,
    pub(crate) trait_path: Option<Vec<String>>,
    pub(crate) methods: Vec<String>,
    pub(crate) node: syn::ItemImpl,
    pub(crate) found_in_module: ModuleId,
}

pub(crate) struct Module {
    pub(crate) items: Vec<ItemId>,
    // (declared name, resolved child module) -- kept as pairs rather than
    // two parallel vecs so a `mod` declaration that doesn't resolve to a
    // file can't silently desync the correspondence between a name and
    // its module.
    pub(crate) children: Vec<(String, ModuleId)>,
    pub(crate) module_path: String,
}

pub struct Db {
    modules: HashMap<ModuleId, Module>,
    items: HashMap<ItemId, Item>,
    inherent_impls: HashMap<ItemId, Vec<ImplId>>,
    trait_impls: HashMap<ItemId, Vec<ImplId>>,
    impls: Vec<Impl>,
}

// Abstracts the id -> data lookups that `resolve_path` needs, so it can run
// against either the fully-built `Db` or a partial view during bootstrap
// (before `inherent_impls`/`impls` exist yet), without duplicating the walk.
pub(crate) trait Lookup {
    fn module(&self, id: ModuleId) -> Option<&Module>;
    fn item(&self, id: ItemId) -> Option<&Item>;
    fn inherent_impls(&self, id: ItemId) -> &[ImplId];
    fn impl_(&self, id: ImplId) -> Option<&Impl>;
}

impl Lookup for Db {
    fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }
    fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.get(&id)
    }
    fn inherent_impls(&self, id: ItemId) -> &[ImplId] {
        self.inherent_impls.get(&id).map_or(&[], Vec::as_slice)
    }
    fn impl_(&self, id: ImplId) -> Option<&Impl> {
        self.impls.get(id.0)
    }
}

impl Db {
    /// Parses every `.rs` file under `src_dir` and resolves the module tree
    /// rooted at `entry_file` (typically `src_dir.join("lib.rs")` or
    /// `src_dir.join("main.rs")` -- a package with multiple crate targets
    /// has one independent module tree per target, so the root is an
    /// explicit choice, not something this function guesses).
    pub fn build(src_dir: &Path, entry_file: &Path) -> anyhow::Result<Db> {
        let (modules, raw_impls, items) = crate::parse::parse_files(src_dir, entry_file)?;
        let (inherent_impls, trait_impls, impls) = resolve_impls(&modules, &items, raw_impls);

        Ok(Db {
            modules,
            items,
            inherent_impls,
            trait_impls,
            impls,
        })
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }

    pub fn impls(&self) -> &[Impl] {
        &self.impls
    }

    pub fn resolve_path(&self, start_module: ModuleId, path: &[String]) -> Option<ItemId> {
        resolve_path(self, start_module, path)
    }
}

pub(crate) fn resolve_path(db: &impl Lookup, start_module: ModuleId, path: &[String]) -> Option<ItemId> {
    if path.is_empty() {
        return None;
    }
    let mut current_module = start_module;
    for (i, segment) in path.iter().enumerate() {
        let is_last = i == path.len() - 1;
        if is_last {
            let module = db.module(current_module)?;
            return module.items.iter().copied().find_map(|item_id| {
                db.item(item_id)
                    .filter(|item| item.name == *segment)
                    .map(|item| item.id)
            });
        } else {
            let module = db.module(current_module)?;

            if let Some((_, child_id)) = module.children.iter().find(|(name, _)| name == segment) {
                current_module = *child_id;
                continue;
            }

            if let Some(item_id) = module
                .items
                .iter()
                .copied()
                .find(|item_id| db.item(*item_id).map_or(false, |item| item.name == *segment))
            {
                let item = db.item(item_id)?;

                let remaining_segments = path.len() - i - 1;
                if remaining_segments == 1 {
                    let next_segment = &path[i + 1];

                    if let ItemKind::Enum(e) = &item.kind {
                        if e.variants.iter().any(|v| v.ident == next_segment) {
                            return Some(item.id);
                        }
                    }

                    for impl_id in db.inherent_impls(item.id) {
                        if let Some(impl_) = db.impl_(*impl_id) {
                            if impl_.methods.contains(next_segment) {
                                return Some(item.id);
                            }
                        }
                    }
                }
            }

            return None;
        }
    }
    None
}

// No impls are known yet at this point in the pipeline -- resolve_impls is
// what's about to produce them -- so inherent_impls/impl_ have nothing to
// report. That's fine: target-type paths never have a trailing method
// segment to resolve against them here.
struct Bootstrap<'a> {
    modules: &'a HashMap<ModuleId, Module>,
    items: &'a HashMap<ItemId, Item>,
}

impl Lookup for Bootstrap<'_> {
    fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }
    fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.get(&id)
    }
    fn inherent_impls(&self, _id: ItemId) -> &[ImplId] {
        &[]
    }
    fn impl_(&self, _id: ImplId) -> Option<&Impl> {
        None
    }
}

pub(crate) fn resolve_impls(
    modules: &HashMap<ModuleId, Module>,
    items: &HashMap<ItemId, Item>,
    raw_impls: Vec<RawImpl>,
) -> (
    HashMap<ItemId, Vec<ImplId>>,
    HashMap<ItemId, Vec<ImplId>>,
    Vec<Impl>,
) {
    let mut inherent_impls: HashMap<ItemId, Vec<ImplId>> = HashMap::new();
    let mut trait_impls: HashMap<ItemId, Vec<ImplId>> = HashMap::new();
    let mut impls = Vec::new();
    let bootstrap = Bootstrap { modules, items };

    for raw in raw_impls {
        let target_id = match resolve_path(&bootstrap, raw.found_in_module, &raw.target_path) {
            Some(id) => id,
            None => continue,
        };
        let trait_id = if let Some(tpath) = raw.trait_path {
            resolve_path(&bootstrap, raw.found_in_module, &tpath)
        } else {
            None
        };
        let module_path = modules
            .get(&raw.found_in_module)
            .map(|m| m.module_path.clone())
            .unwrap_or_default();

        let impl_id = ImplId(impls.len());
        impls.push(Impl {
            target_type: target_id,
            trait_: trait_id,
            methods: raw.methods,
            node: raw.node,
            module_path,
            parent: raw.found_in_module,
        });

        if trait_id.is_some() {
            trait_impls
                .entry(target_id)
                .or_insert_with(Vec::new)
                .push(impl_id);
        } else {
            inherent_impls
                .entry(target_id)
                .or_insert_with(Vec::new)
                .push(impl_id);
        }
    }

    (inherent_impls, trait_impls, impls)
}
