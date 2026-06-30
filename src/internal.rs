// internal/analyze.rs — allowed imports
pub use ra_ap_ide_db::RootDatabase;
pub use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
pub use ra_ap_project_model::{CargoConfig, ManifestPath};
// nothing else ra_ap_*

// internal/traverse.rs — allowed imports
pub use ra_ap_hir::{
    Adt, Crate, Enum, Function, HasAttrs, HasSource, HirFileId, Impl, Module, ModuleDef, ScopeDef,
    Struct, Trait, db::HirDatabase,
};
// nothing else ra_ap_*

// internal/bridge.rs — allowed imports
pub use ra_ap_syntax::AstNode; // only for .syntax().text()
// convert to string immediately, then syn::parse_str
// no ra_ap_hir types in return values

// model/, workspace/, lib.rs — allowed imports
// ZERO ra_ap_* imports
// only: syn, sema's own types
