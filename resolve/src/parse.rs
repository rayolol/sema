use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::db::{Module, RawImpl};
use crate::ids::{ItemId, ModuleId};
use crate::item::{Item, lower_items};

fn discover_rust_files(src_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(src_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|f| f.path().extension().map_or(false, |ext| ext == "rs"))
        .map(|f| f.path().to_path_buf())
        .collect()
}

pub(crate) fn parse_files(
    src_dir: &Path,
    entry_file: &Path,
) -> anyhow::Result<(HashMap<ModuleId, Module>, Vec<RawImpl>, HashMap<ItemId, Item>)> {
    // Pass 1: parse every file once, keeping the AST around instead of
    // lowering immediately -- item ids depend on crate-style module paths
    // (pass 3), which in turn depend on the module tree (pass 2), which is
    // cheap to build from each file's already-parsed `mod` declarations
    // without a second read+reparse.
    let mut parsed: HashMap<ModuleId, (PathBuf, syn::File)> = HashMap::new();
    for file in discover_rust_files(src_dir) {
        let module_id = ModuleId::from(file.to_string_lossy().into_owned());
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("resolve: skipping {} (read error: {e})", file.display());
                continue;
            }
        };
        let ast = match syn::parse_file(&content) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("resolve: skipping {} (parse error: {e})", file.display());
                continue;
            }
        };
        parsed.insert(module_id, (file, ast));
    }

    let entry_id = ModuleId::from(entry_file.to_string_lossy().into_owned());
    if !parsed.contains_key(&entry_id) {
        anyhow::bail!(
            "entry file {} was not found under {} (or failed to read/parse)",
            entry_file.display(),
            src_dir.display()
        );
    }

    // Pass 2: link each file's `mod name;` declarations to the file that
    // satisfies them by filename convention (name.rs or name/mod.rs).
    let mut modules: HashMap<ModuleId, Module> = HashMap::new();
    for module_id in parsed.keys().copied().collect::<Vec<_>>() {
        let ast = &parsed[&module_id].1;
        let decl_names: Vec<String> = ast
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(m) if m.content.is_none() => Some(m.ident.to_string()),
                _ => None,
            })
            .collect();

        let children: Vec<(String, ModuleId)> = decl_names
            .into_iter()
            .filter_map(|name| {
                // `mod name;` is satisfied by either sibling `name.rs` or
                // `name/mod.rs` -- both are valid Rust module layouts.
                let flat_suffix = format!("/{name}.rs");
                let nested_suffix = format!("/{name}/mod.rs");
                parsed
                    .iter()
                    .find(|(_, (path, _))| {
                        let p = path.to_string_lossy();
                        p.ends_with(&flat_suffix) || p.ends_with(&nested_suffix)
                    })
                    .map(|(child_id, _)| (name, *child_id))
            })
            .collect();

        modules.insert(
            module_id,
            Module {
                items: vec![],
                children,
                module_path: String::new(),
            },
        );
    }

    // Pass 3: BFS from the entry module, assigning crate-style module paths
    // ("" for the root, "motor", "motor::state", ...). Modules unreachable
    // from the entry -- stray files, or files belonging to a different
    // Cargo target under the same src tree (e.g. src/bin/other.rs) -- are
    // logged and excluded below rather than given a fabricated path.
    let mut reached: HashSet<ModuleId> = HashSet::new();
    let mut queue = VecDeque::new();
    reached.insert(entry_id);
    modules.get_mut(&entry_id).unwrap().module_path = String::new();
    queue.push_back(entry_id);

    while let Some(current) = queue.pop_front() {
        let (children, current_path) = {
            let m = &modules[&current];
            (m.children.clone(), m.module_path.clone())
        };
        for (name, child_id) in children {
            if reached.insert(child_id) {
                let child_path = if current_path.is_empty() {
                    name
                } else {
                    format!("{current_path}::{name}")
                };
                modules.get_mut(&child_id).unwrap().module_path = child_path;
                queue.push_back(child_id);
            }
        }
    }

    let unreached: Vec<&PathBuf> = parsed
        .iter()
        .filter(|(id, _)| !reached.contains(id))
        .map(|(_, (path, _))| path)
        .collect();
    if !unreached.is_empty() {
        eprintln!(
            "resolve: {} file(s) not reachable via `mod` declarations from {}, skipped: {:?}",
            unreached.len(),
            entry_file.display(),
            unreached
        );
    }
    modules.retain(|id, _| reached.contains(id));

    // Pass 4: lower items, now that every reached module's path is known.
    let mut raw_impls = Vec::new();
    let mut items_map: HashMap<ItemId, Item> = HashMap::new();

    for module_id in reached {
        let ast = &parsed[&module_id].1;
        let module_path = modules[&module_id].module_path.clone();
        let mut item_ids = Vec::new();

        for item in &ast.items {
            match item {
                syn::Item::Mod(m) if m.content.is_none() => {} // already linked in pass 2
                syn::Item::Impl(i) => {
                    let target_path = if let syn::Type::Path(tp) = &*i.self_ty {
                        tp.path
                            .segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    let trait_path = i
                        .trait_
                        .as_ref()
                        .map(|(_, t, _)| t.segments.iter().map(|s| s.ident.to_string()).collect());
                    let methods = i
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            syn::ImplItem::Fn(f) => Some(f.sig.ident.to_string()),
                            _ => None,
                        })
                        .collect();
                    raw_impls.push(RawImpl {
                        target_path,
                        trait_path,
                        methods,
                        node: i.clone(),
                        found_in_module: module_id,
                    });
                }
                other => {
                    let item = lower_items(other, module_id, &module_path);
                    item_ids.push(item.id);
                    items_map.insert(item.id, item);
                }
            }
        }
        modules.get_mut(&module_id).unwrap().items = item_ids;
    }

    Ok((modules, raw_impls, items_map))
}
