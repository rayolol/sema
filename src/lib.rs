mod query;
mod resolve_bridge;
mod types;
mod workspaces;

pub use query::{Query, SemaItem};
pub use types::{
    ItemId, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct, ResolvedTrait,
};
pub use workspaces::Workspace;

pub struct Config {
    pub manifest_path: std::path::PathBuf,
    // Reserved for the on-disk resolve cache (postcard-serialized `Db`,
    // keyed by a content hash of the source tree) -- not implemented yet,
    // but this is the directory it will live in once it is.
    pub target_dir: std::path::PathBuf,
}

pub fn analysis(config: Config) -> anyhow::Result<Workspace> {
    let src_dir = config
        .manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("manifest path has no parent directory"))?
        .join("src");
    let entry = src_dir.join("lib.rs");

    let db = resolve::Db::build(&src_dir, &entry)?;
    Ok(resolve_bridge::build_workspace(db))
}
