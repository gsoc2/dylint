use crate::error::warn;
use anyhow::{anyhow, bail, ensure, Result};
use cargo::core::{
    source::MaybePackage, Dependency, Features, Package as CargoPackage, QueryKind, Source,
};
pub use cargo::{
    core::{PackageId, SourceId},
    util::Config,
};
use cargo_metadata::Metadata;
use std::path::PathBuf;

mod toml;
pub use self::toml::DetailedTomlDependency;

pub fn dependency_source_id_and_root(
    opts: &crate::Dylint,
    metadata: &Metadata,
    config: &Config,
    details: &DetailedTomlDependency,
) -> Result<(SourceId, PathBuf)> {
    let name_in_toml = "library";

    let mut deps = vec![];
    let root = PathBuf::from(&metadata.workspace_root);
    let source_id = SourceId::for_path(&root)?;
    let mut nested_paths = vec![];
    let mut warnings = vec![];
    let features = Features::new(&[], config, &mut warnings, source_id.is_path())?;
    let mut cx = toml::Context::new(
        &mut deps,
        source_id,
        &mut nested_paths,
        config,
        &mut warnings,
        None,
        &root,
        &features,
    );

    let kind = None;

    let dep = details.to_dependency(name_in_toml, &mut cx, kind)?;

    if !warnings.is_empty() {
        warn(opts, &warnings.join("\n"));
    }

    let source_id = dep.source_id();

    let root = dependency_root(config, &dep)?;

    Ok((source_id, root))
}

fn dependency_root(config: &Config, dep: &Dependency) -> Result<PathBuf> {
    let source_id = dep.source_id();

    if source_id.is_path() {
        if let Some(path) = source_id.local_path() {
            Ok(path)
        } else {
            bail!("Path source should have a local path: {}", source_id)
        }
    } else if source_id.is_git() {
        git_dependency_root(config, dep)
    } else {
        bail!("Only git and path entries are supported: {}", source_id)
    }
}

fn git_dependency_root(config: &Config, dep: &Dependency) -> Result<PathBuf> {
    let _lock = config.acquire_package_cache_lock()?;

    #[allow(clippy::default_trait_access)]
    let mut source = dep.source_id().load(config, &Default::default())?;

    let package_id = sample_package_id(dep, &mut *source)?;

    if let MaybePackage::Ready(package) = source.download(package_id)? {
        git_dependency_root_from_package(config, &*source, &package)
    } else {
        bail!(format!("`{}` is not ready", package_id.name()))
    }
}

#[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
#[cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]
fn sample_package_id(dep: &Dependency, source: &mut dyn Source) -> Result<PackageId> {
    let mut package_id: Option<PackageId> = None;

    while {
        let poll = source.query(dep, QueryKind::Fuzzy, &mut |summary| {
            if package_id.is_none() {
                package_id = Some(summary.package_id());
            }
        })?;
        if poll.is_pending() {
            source.block_until_ready()?;
            package_id.is_none()
        } else {
            false
        }
    } {}

    package_id.ok_or_else(|| anyhow!("Found no packages in `{}`", dep.source_id()))
}

fn git_dependency_root_from_package<'a>(
    config: &'a Config,
    source: &(dyn Source + 'a),
    package: &CargoPackage,
) -> Result<PathBuf> {
    let package_root = package.root();

    if source.source_id().is_git() {
        let git_path = config.git_path();
        let git_path = config.assert_package_cache_locked(&git_path);
        ensure!(
            package_root.starts_with(git_path.join("checkouts")),
            "Unexpected path: {}",
            package_root.to_string_lossy()
        );
        let n = git_path.components().count() + 3;
        Ok(package_root.components().take(n).collect())
    } else if source.source_id().is_path() {
        unreachable!()
    } else {
        unimplemented!()
    }
}
