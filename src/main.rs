use cargo::core::package::PackageSet;
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::Method;
use cargo::core::shell::Shell;
use cargo::core::{Package, PackageId, Resolve, Workspace};
use cargo::ops;
use cargo::util::{self, important_paths, CargoResult, Cfg, Rustc};
use cargo::Config;
use std::collections::{HashMap, HashSet};
use std::fmt;

fn main() {
    let mut config = match Config::default() {
        Ok(cfg) => cfg,
        Err(e) => {
            let mut shell = Shell::new();
            cargo::exit_with_error(e.into(), &mut shell)
        }
    };

    let root = important_paths::find_root_manifest_for_wd(&config.cwd()).unwrap();
    let workspace = Workspace::new(&root, &config).unwrap();
    let package = workspace.current().unwrap();
    let mut registry = registry(&config, &package).unwrap();
    let (packages, resolve) = resolve(
        &mut registry,
        &workspace,
    ).unwrap();
    let ids = packages.package_ids().collect::<Vec<_>>();
    let packages = registry.get(&ids).unwrap();
    // let root = package.package_id();
    let (enabled_features_map, disabled_features) = build_graph(
        &resolve,
        &packages,
        package.package_id(),
        None,
        // cfgs.as_ref().map(|r| &**r),
    );
    println!("Enabled Features");
    for (key, value) in &enabled_features_map {
        println!("{} {:?}", key, value);
    }
    println!("Disabled Features");
    for feature in &disabled_features {
        println!("{}", feature);
    }
}

fn registry<'a>(config: &'a Config, package: &Package) -> CargoResult<PackageRegistry<'a>> {
    let mut registry = PackageRegistry::new(config)?;
    registry.add_sources(Some(package.package_id().source_id().clone()))?;
    Ok(registry)
}

fn resolve<'a, 'cfg>(
    registry: &mut PackageRegistry<'cfg>,
    workspace: &'a Workspace<'cfg>,
) -> CargoResult<(PackageSet<'a>, Resolve)> {
    let (packages, resolve) = ops::resolve_ws(workspace)?;

    let method = Method::Everything;
    // Method::Required {
    //     dev_deps: !no_dev_dependencies,
    //     features: &features,
    //     all_features,
    //     uses_default_features: !no_default_features,
    // };

    let resolve = ops::resolve_with_previous(
        registry,
        workspace,
        method,
        Some(&resolve),
        None,
        &[],
        true,
        true,
    )?;
    Ok((packages, resolve))
}

fn build_graph<'a>(
    resolve: &'a Resolve,
    packages: &'a PackageSet<'_>,
    root: PackageId,
    target: Option<&str>,
    // cfgs: Option<&[Cfg]>,
) -> (HashMap<Feature, Vec<String>>, HashSet<Feature>) {
    let mut enabled_features_map: HashMap<Feature, Vec<String>> = HashMap::new();
    let mut disabled_features: HashSet<Feature> = HashSet::new();
    let mut pending = vec![root];
    while let Some(pkg_id) = pending.pop() {
        for raw_dep_id in resolve.deps_not_replaced(pkg_id) {
            let dep_id = match resolve.replacement(raw_dep_id) {
                Some(id) => id,
                None => raw_dep_id,
            };
            pending.push(dep_id);
            for feature_name in resolve.features(dep_id).iter() {
                let feature = Feature{
                    name: feature_name.clone(),
                    parent_crate: dep_id.name().to_string(),
                    version: dep_id.version().to_string(),
                };
                let enabling_crates = enabled_features_map.entry(feature).or_insert(Vec::new());
                enabling_crates.push(pkg_id.name().to_string())
            }
            for (feature_name, _) in packages.get_one(dep_id).unwrap().summary().features() {
                let feature = Feature{
                    name: feature_name.to_string(),
                    parent_crate: dep_id.name().to_string(),
                    version: dep_id.version().to_string(),
                };
                match enabled_features_map.get(&feature) {
                    None => disabled_features.insert(feature),
                    _ => false,
                };
            }
        }
    }
    return (enabled_features_map, disabled_features);
}

#[derive(PartialEq, Eq, Hash)]
struct Feature {
    name: String,
    parent_crate: String,
    version: String,
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}/{}", self.parent_crate, self.version, self.name)
    }
}