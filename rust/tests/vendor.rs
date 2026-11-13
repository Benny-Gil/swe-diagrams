// Tests of the vendor packages. The fake zips in spec/vendor hold simple
// shapes, not vendor files, and no test uses the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use swe_diagrams::icons::{resolve, IconOptions};
use swe_diagrams::vendor::{build_index, cache_dir, list, zip_entries, zip_read, Cloud, Package};

fn spec(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(rel)
}

/// An icon directory with a copy of the fake zips, one for each test.
fn cache(tag: &str, zips: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("swe-diagrams-vendor-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("vendor")).unwrap();
    for z in zips {
        fs::copy(spec(&format!("vendor/{z}")), dir.join("vendor").join(z)).unwrap();
    }
    dir
}

fn offline(dir: &Path) -> IconOptions {
    IconOptions { dir: Some(dir.to_path_buf()), offline: true }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_zip_reader_reads_stored_and_deflated_files() {
    let bytes = fs::read(spec("vendor/aws.zip")).unwrap();
    let files = zip_entries(&bytes).unwrap();
    assert_eq!(files.len(), 16);
    for f in &files {
        let body = zip_read(&bytes, f).unwrap();
        assert!(!body.is_empty(), "{}", f.path);
    }
    let ec2 = files.iter().find(|f| f.path.ends_with("Arch_Amazon-EC2_64.svg")).unwrap();
    assert!(zip_read(&bytes, ec2).unwrap().starts_with(b"<svg xmlns="));
}

#[test]
fn the_index_keeps_the_icons_and_drops_the_rest() {
    let zips = [fs::read(spec("vendor/aws.zip")).unwrap()];
    let index = build_index(Cloud::Aws, &zips).unwrap();
    let got: Vec<(u8, &str)> = index.iter().map(|e| (e.rank, e.name.as_str())).collect();
    assert_eq!(
        got,
        [
            (0, "Amazon-Simple-Queue-Service"),
            (0, "AWS-Lambda"),
            (0, "Amazon-EC2"),
            (0, "Amazon-RDS"),
            (0, "Amazon-CloudFront"),
            (0, "Amazon-Simple-Storage-Service"),
            (1, "Elastic-Load-Balancing_Application-Load-Balancer"),
            (3, "AWS-Cloud-logo"),
            (3, "Private-subnet"),
            (3, "Public-subnet"),
            (3, "Region"),
            (3, "Virtual-private-cloud-VPC"),
        ]
    );
}

#[test]
fn the_first_load_writes_the_index_and_a_bad_index_is_built_again() {
    let dir = cache("index", &["azure.zip"]);
    let vendor = cache_dir(&offline(&dir)).unwrap();
    let pkg = Package::load(Cloud::Azure, &vendor, true).unwrap();
    let index = fs::read_to_string(vendor.join("azure.index")).unwrap();
    assert!(index.starts_with("swe-diagrams vendor index 1\n"));
    // The Cosmos DB icon is in two folders. The index keeps the first path.
    assert_eq!(index.matches("Azure-Cosmos-DB").count(), 2);
    assert!(index.contains("databases/10121-icon-service-Azure-Cosmos-DB.svg"));
    assert_eq!(pkg.entries.len(), 6);

    fs::write(vendor.join("azure.index"), "broken").unwrap();
    let again = Package::load(Cloud::Azure, &vendor, true).unwrap();
    assert_eq!(again.entries, pkg.entries);
    assert_eq!(fs::read_to_string(vendor.join("azure.index")).unwrap(), index);
}

#[test]
fn resolve_finds_short_names_in_each_package() {
    let dir = cache("resolve", &["aws.zip", "gcp-core.zip", "gcp-category.zip", "azure.zip"]);
    // A logo name in the same list still comes from the logo cache.
    fs::copy(spec("icons/orbit.svg"), dir.join("orbit.svg")).unwrap();
    let wanted = [
        "aws/s3",
        "aws/EC2s",
        "aws/alb",
        "aws/Amazon_Simple-Queue Service",
        "gcp/gcs",
        "gcp/bq",
        "gcp/pubsub",
        "azure/aks",
        "azure/vnet",
        "azure/cosmos",
        "azure/blob",
        "aws/nothing",
        "gcp/lb",
        "orbit",
    ];
    let got = resolve(&names(&wanted), &offline(&dir));
    for (name, icon) in wanted.iter().zip(&got) {
        let hit = !matches!(*name, "aws/nothing" | "gcp/lb");
        assert_eq!(icon.is_some(), hit, "{name}");
    }
    let aws = got[0].as_ref().unwrap();
    assert!(aws.uri.starts_with("data:image/svg+xml;base64,"));
    assert_eq!(aws.source, "AWS Architecture Icons");
    assert!(aws.square);
    // A resource icon has no square of its own.
    assert!(!got[2].as_ref().unwrap().square);
    assert_eq!(got[4].as_ref().unwrap().source, "Google Cloud icons");
    assert!(!got[4].as_ref().unwrap().square);
    assert_eq!(got[7].as_ref().unwrap().source, "Azure icons");
}

#[test]
fn a_missing_package_offline_gives_no_icon_and_writes_nothing() {
    let dir = cache("missing", &[]);
    let got = resolve(&names(&["aws/s3", "gcp/bq"]), &offline(&dir));
    assert_eq!(got, [None, None]);
    assert_eq!(fs::read_dir(dir.join("vendor")).unwrap().count(), 0);
    let err = list(Cloud::Aws, "", &offline(&dir)).unwrap_err();
    assert!(err.starts_with("no AWS Architecture Icons package in "), "{err}");
}

#[test]
fn gcp_needs_both_packages() {
    let dir = cache("half", &["gcp-core.zip"]);
    assert_eq!(resolve(&names(&["gcp/bq"]), &offline(&dir)), [None]);
}

#[test]
fn list_puts_the_best_match_first_and_shows_short_names() {
    let dir = cache("list", &["aws.zip", "gcp-core.zip", "gcp-category.zip"]);
    let got = list(Cloud::Aws, "s3", &offline(&dir)).unwrap();
    assert_eq!(got, ["aws/amazon-simple-storage-service  service", "aws/s3  = aws/amazon-simple-storage-service"]);
    let got = list(Cloud::Gcp, "pubsub", &offline(&dir)).unwrap();
    assert_eq!(got, ["gcp/integration-services  category", "gcp/pubsub  = gcp/integration-services"]);
    let all = list(Cloud::Aws, "", &offline(&dir)).unwrap();
    assert!(all.contains(&"aws/region  group".to_string()));
}

fn icons_cli(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_swe-diagrams"))
        .arg("icons")
        .args(args)
        .env("SWE_DIAGRAMS_ICONS", dir)
        .env("SWE_DIAGRAMS_OFFLINE", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let s = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (out.status.code(), s(out.stdout), s(out.stderr))
}

#[test]
fn the_icons_command_prints_the_names_that_match() {
    let dir = cache("cli", &["azure.zip"]);
    let (code, out, err) = icons_cli(&dir, &["azure", "vnet"]);
    assert_eq!((code, err.as_str()), (Some(0), ""));
    assert_eq!(out, "azure/virtual-networks  service\nazure/vnet  = azure/virtual-networks\n");
    let (code, out, _) = icons_cli(&dir, &["azure", "zzz"]);
    assert_eq!((code, out.as_str()), (Some(0), ""));
    let (code, _, err) = icons_cli(&dir, &["aws"]);
    assert_eq!(code, Some(1));
    assert!(err.starts_with("error: no AWS Architecture Icons package"), "{err}");
    let (code, _, err) = icons_cli(&dir, &["oracle"]);
    assert_eq!(code, Some(1));
    assert!(err.starts_with("usage: swe-diagrams icons"), "{err}");
}

#[test]
fn each_vendor_fixture_gives_a_png_in_both_themes() {
    // The PNG reader needs well-formed XML, so an open group tag fails here.
    let dir = cache("png", &["aws.zip", "azure.zip", "gcp-core.zip", "gcp-category.zip"]);
    for name in ["vendor-aws", "vendor-azure", "vendor-gcp"] {
        let path = spec(&format!("fixtures/{name}.sd"));
        let r = swe_diagrams::render_path(&fs::read_to_string(&path).unwrap(), Some(&path), &offline(&dir)).unwrap();
        for theme in [swe_diagrams::Theme::Light, swe_diagrams::Theme::Dark] {
            let svg = swe_diagrams::apply_theme(&r.svg, theme);
            swe_diagrams::png(&svg, 1.0).unwrap_or_else(|e| panic!("{name} {theme:?}: {e}"));
        }
    }
}
