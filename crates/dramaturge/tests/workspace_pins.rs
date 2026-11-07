//! The crates of the workspace depend on each other with an exact version.
//!
//! They are released together, and many public types are `#[non_exhaustive]`,
//! so `dramaturge-serde` cannot find a newer `dramaturge-model` variant at
//! compile time. An exact requirement keeps a host from mixing versions. This
//! test keeps a release or a new internal crate from undoing it.

#[test]
fn internal_dependencies_use_an_exact_version() {
    let manifest = include_str!("../../../Cargo.toml");
    let internal: Vec<&str> = manifest
        .lines()
        .filter(|l| l.starts_with("dramaturge") && l.contains("path = \"crates/"))
        .collect();
    assert!(!internal.is_empty(), "no internal dependencies found");
    for line in internal {
        assert!(
            line.contains("version = \"="),
            "an internal dependency without an exact version: {line}"
        );
    }
}
