# feature-analyst
feature-analyst is a tool written in rust to analyse the feature usage in your rust project. 

It provides with a list of Enabled features in your dependency tree with names of crates enabling those features and a list of all the disabled features in your dependency tree.

## Usage
    USAGE:
    feature_analyst [FLAGS] [OPTIONS]

    FLAGS:
            --all-features           Activate all available features
        -h, --help                   Prints help information
            --no-default-features    Do not activate the `default` feature
            --no-dev-dependencies    Skip dev dependencies.
        -V, --version                Prints version information

    OPTIONS:
            --features <FEATURES>    Space-separated list of features to activate

## Example Output
    Enabled features
    ------------------

    mycrate/default
    mycrate/foo
    dep1/default[mycrate]
    dep1/bar [mycrate]
    dep2/baz [mycrate]
    dep3-1.0.0/qux [mycrate, dep1]
    dep3-1.1.0/quazam [dep2]

    Disabled features
    -------------------

    mycrate/z
    dep2/default
    dep2/y
    dep3-1.0.0/default
    dep3-1.0.0/x
    dep3-1.1.0/default

## Installation
    cargo install feature_analyst
