mod analyse;
mod bridge;
pub mod internal;
mod query;
mod types;
mod workspaces;
use anyhow::Ok;

use camino::Utf8PathBuf;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_paths::AbsPathBuf;
use ra_ap_project_model::{CargoConfig, ManifestPath, TargetDirectoryConfig};

pub use query::{Query, SemaItem};
pub use types::{
    ItemId, ResolvedEnum, ResolvedFunction, ResolvedImpl, ResolvedStruct, ResolvedTrait,
};
pub use workspaces::Workspace;

pub struct Config {
    pub manifest_path: std::path::PathBuf,
}

fn load(
    manifest: &std::path::Path,
) -> anyhow::Result<(ra_ap_ide_db::RootDatabase, ra_ap_vfs::Vfs)> {
    let load_config = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
        num_worker_threads: 5,
        proc_macro_processes: 1,
    };

    let cargo_config = CargoConfig {
        target_dir_config: TargetDirectoryConfig::Directory(
            Utf8PathBuf::from_path_buf(std::env::var("OUT_DIR")?.into())
                .map_err(|_| anyhow::anyhow!("OUT_DIR path is not UTF-8"))?
                .join("sema-target"),
        ),
        no_deps: true,
        ..CargoConfig::default()
    };

    let utf8_path = Utf8PathBuf::from_path_buf(manifest.to_path_buf())
        .map_err(|_| anyhow::anyhow!("manifest path is not UTF-8"))?;
    let abs_path = AbsPathBuf::assert(utf8_path);
    let manifest_path = ManifestPath::try_from(abs_path)
        .map_err(|_| anyhow::anyhow!("manifest path must not be root"))?;

    let (db, vfs, _proc_macro_server) = load_workspace_at(
        manifest_path.as_ref(),
        &cargo_config,
        &load_config,
        &|msg| eprintln!("sema: {msg}"),
    )
    .map_err(|e| anyhow::anyhow!("workspace load failed: {}", e))?;

    Ok((db, vfs))
}

pub fn analysis(config: Config) -> anyhow::Result<Workspace> {
    let (db, vfs) = load(&config.manifest_path)?;
    let raw = analyse::collect(&db);
    let workspace = bridge::build_workspace(raw, &db, &vfs);

    Ok(workspace)
}
