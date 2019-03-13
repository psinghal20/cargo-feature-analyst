use cargo::core::package::PackageSet;
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::Method;
use cargo::core::shell::Shell;
use cargo::core::{Package, PackageId, Resolve, Workspace};
use cargo::ops;
use cargo::util::{important_paths, CargoResult};
use cargo::Config;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use structopt::StructOpt;


#[derive(StructOpt)]
struct Opt {
    #[structopt(long = "features", value_name = "FEATURES")]
    /// Space-separated list of features to activate
    features: Option<String>,
    #[structopt(long = "all-features")]
    /// Activate all available features
    all_features: bool,
    #[structopt(long = "no-default-features")]
    /// Do not activate the `default` feature
    no_default_features: bool,
    #[structopt(long = "no-dev-dependencies")]
    /// Skip dev dependencies.
    no_dev_dependencies: bool,
}

fn main() {
    let config = match Config::default() {
        Ok(cfg) => cfg,
        Err(e) => {
            let mut shell = Shell::new();
            cargo::exit_with_error(e.into(), &mut shell)
        }
    };

    let args = Opt::from_args();

    let root = important_paths::find_root_manifest_for_wd(&config.cwd()).unwrap();
    let workspace = Workspace::new(&root, &config).unwrap();
    let package = workspace.current().unwrap();
    let mut registry = registry(&config, &package).unwrap();
    let (packages, resolve) = resolve(
        &mut registry,
        &workspace,
        args.features,
        args.all_features,
        args.no_default_features,
        args.no_dev_dependencies,
    ).unwrap();

    let ids = packages.package_ids().collect::<Vec<_>>();
    let packages = registry.get(&ids).unwrap();

    let mut enabled_features_map: BTreeMap<Feature, Vec<String>> = BTreeMap::new();
    let mut disabled_features: BTreeSet<Feature> = BTreeSet::new();
    
    build_graph(
        &resolve,
        &packages,
        package.package_id(),
        &mut enabled_features_map,
        &mut disabled_features,
    );
    println!("Enabled Features");
    print_seperator();
    for (key, value) in &enabled_features_map {
        println!("{} {:?}", key, value);
    }
    println!("\nDisabled Features");
    print_seperator();
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
    features: Option<String>,
    all_features: bool,
    no_default_features: bool,
    no_dev_dependencies: bool,
) -> CargoResult<(PackageSet<'a>, Resolve)> {
    let features = Method::split_features(&features.into_iter().collect::<Vec<_>>());

    let (packages, resolve) = ops::resolve_ws(workspace)?;

    let method = Method::Required {
        dev_deps: !no_dev_dependencies,
        features: &features,
        all_features,
        uses_default_features: !no_default_features,
    };

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

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Feature {
    parent_crate: String,
    version: String,
    name: String,
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}/{}", self.parent_crate, self.version, self.name)
    }
}

fn build_graph<'a>(
    resolve: &'a Resolve,
    packages: &'a PackageSet<'_>,
    root: PackageId,
    enabled_features_map: &mut BTreeMap<Feature, Vec<String>>,
    disabled_features: &mut BTreeSet<Feature>,
) -> () {
    let mut traversed_pkg: HashSet<PackageId> = HashSet::new();
    let mut pending = vec![root];
    while let Some(pkg_id) = pending.pop() {
        if !traversed_pkg.insert(pkg_id) {
            continue;
        }
        for raw_dep_id in resolve.deps_not_replaced(pkg_id) {
            let dep_id = match resolve.replacement(raw_dep_id) {
                Some(id) => id,
                None => raw_dep_id,
            };
            pending.push(dep_id);
            for feature_name in resolve.features(dep_id).iter() {
                let feature = Feature{
                    parent_crate: dep_id.name().to_string(),
                    version: dep_id.version().to_string(),
                    name: feature_name.clone(),
                };
                let enabling_crates = enabled_features_map.entry(feature).or_insert(Vec::new());
                enabling_crates.push(pkg_id.name().to_string())
            }
            for (feature_name, _) in packages.get_one(dep_id).unwrap().summary().features() {
                let feature = Feature{
                    parent_crate: dep_id.name().to_string(),
                    version: dep_id.version().to_string(),
                    name: feature_name.to_string(),
                };
                match enabled_features_map.get(&feature) {
                    None => disabled_features.insert(feature),
                    _ => false,
                };
            }
        }
    }
}


fn print_seperator() {
    for _ in 1..20 {
        print!("-");
    }
    println!();
}