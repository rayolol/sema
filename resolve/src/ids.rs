use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ModuleId identifies a module by the file it came from. The interner lets
// a ModuleId be decoded back to that file path later (e.g. for
// `Resolved*.file` in a downstream bridge), without threading the path
// alongside every ModuleId everywhere it's used.
static MODULE_ID_INTERNER: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct ModuleId(u64);

impl ModuleId {
    pub(crate) fn from(path: String) -> Self {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        let id = hasher.finish();

        let interner = MODULE_ID_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = interner.lock().unwrap().insert(id, path);

        Self(id)
    }

    fn decode(self) -> String {
        let interner = MODULE_ID_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
        interner
            .lock()
            .unwrap()
            .get(&self.0)
            .cloned()
            .unwrap_or_default()
    }

    /// The source file this module was parsed from.
    pub fn file_path(self) -> PathBuf {
        PathBuf::from(self.decode())
    }
}

// ItemId identifies an item by its crate-style module path and name (e.g.
// "motor" + "MotorConfig"), not by which file it happens to live in --
// matches how Rust itself names things, and is stable across file moves
// that don't change the module path.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct ItemId(u64);

impl ItemId {
    /// Deterministic id for an item given its crate-style module path and
    /// name (e.g. "motor" + "MotorConfig"). Public so a downstream consumer
    /// can mint compatible ids for things resolve itself doesn't name --
    /// impl blocks, for instance, which have no identifier of their own.
    pub fn from(module_path: &str, name: &str) -> Self {
        let qualified = format!("{module_path}::{name}");
        let mut hasher = DefaultHasher::new();
        qualified.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct ImplId(pub(crate) usize);
