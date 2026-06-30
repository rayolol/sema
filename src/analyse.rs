//resolve the import
use std::collections::HashSet;

use crate::internal::{
    Adt, Crate, Enum, Function, Impl, Module, ModuleDef, RootDatabase, ScopeDef, Struct, Trait,
};

#[derive(Default, Clone)]
pub(crate) struct RawItems {
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub functions: Vec<Function>,
}

pub fn collect(db: &RootDatabase) -> RawItems {
    let mut out = RawItems::default();
    let crates = Crate::all(db);

    for crt in crates.iter() {
        let crt: &Crate = crt;
        if !crt.origin(db).is_local() {
            continue;
        }

        let root_module = crt.root_module(db);

        let mut visited = HashSet::new();

        out.impls.extend(Impl::all_in_crate(db, *crt));
        walk_module(root_module, db, &mut out, &mut visited);
    }

    out
}

fn walk_module(
    module: Module,
    db: &RootDatabase,
    out: &mut RawItems,
    visited: &mut HashSet<Module>,
) {
    if !visited.insert(module) {
        return;
    }

    for (_name, def) in module.scope(db, None) {
        if let ScopeDef::ModuleDef(n) = def {
            match n {
                ModuleDef::Adt(Adt::Struct(s)) => {
                    if s.module(db) == module {
                        out.structs.push(s);
                    }
                }
                ModuleDef::Adt(Adt::Enum(e)) => {
                    if e.module(db) == module {
                        out.enums.push(e);
                    }
                }
                ModuleDef::Trait(t) => {
                    if t.module(db) == module {
                        out.traits.push(t);
                    }
                }
                ModuleDef::Function(f) => {
                    if f.module(db) == module {
                        out.functions.push(f);
                    }
                }
                ModuleDef::Module(m) => {
                    walk_module(m, db, out, visited);
                }
                _ => {}
            }
        }
    }
}
