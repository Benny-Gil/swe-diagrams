// Tests of the files that the crates.io package and the npm package need.

use std::fs;
use std::path::Path;

fn repo(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn crate_license_is_a_copy_of_the_root_license() {
    // The crate cannot include files outside rust/, so rust/LICENSE is a copy.
    assert_eq!(repo("rust/LICENSE"), repo("LICENSE"), "copy LICENSE to rust/LICENSE");
}

#[test]
fn npm_version_is_the_crate_version() {
    // The npm launcher downloads the release of its own version.
    let npm = repo("npm/package.json");
    let want = format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION"));
    assert!(npm.contains(&want), "npm/package.json must have {want}; run scripts/bump-version.sh");
}
