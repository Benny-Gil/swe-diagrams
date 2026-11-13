//! The icons of the cloud vendors: `@aws/<name>`, `@gcp/<name>`, and
//! `@azure/<name>`. The first use downloads the zip package of the vendor
//! into `<icon dir>/vendor/` and writes an index of the names in it.
//! The packages never go into the repo or the binary, because the vendors
//! allow the icons in diagrams but do not clearly allow redistribution.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::icons::{base64, default_dir, put, Icon, IconOptions};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cloud {
    Aws,
    Gcp,
    Azure,
}

pub const CLOUDS: [Cloud; 3] = [Cloud::Aws, Cloud::Gcp, Cloud::Azure];

impl Cloud {
    /// The prefix in `@aws/s3`.
    pub fn name(self) -> &'static str {
        match self {
            Cloud::Aws => "aws",
            Cloud::Gcp => "gcp",
            Cloud::Azure => "azure",
        }
    }

    pub fn from_name(name: &str) -> Option<Cloud> {
        CLOUDS.into_iter().find(|c| c.name().eq_ignore_ascii_case(name))
    }

    /// The name of the package in the credit line under the canvas.
    pub fn credit(self) -> &'static str {
        match self {
            Cloud::Aws => "AWS Architecture Icons",
            Cloud::Gcp => "Google Cloud icons",
            Cloud::Azure => "Azure icons",
        }
    }

    /// The vendor page that links the packages. The links change with each
    /// release, so the program reads them from the page.
    fn page(self) -> &'static str {
        match self {
            Cloud::Aws => "https://aws.amazon.com/architecture/icons/",
            Cloud::Gcp => "https://cloud.google.com/icons",
            Cloud::Azure => "https://learn.microsoft.com/en-us/azure/architecture/icons/",
        }
    }

    /// The packages: the cache file stem, and a text in the link of the package.
    fn parts(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Cloud::Aws => &[("aws", "/Icon-package_")],
            Cloud::Gcp => &[("gcp-core", "/core-products-icons.zip"), ("gcp-category", "/category-icons.zip")],
            Cloud::Azure => &[("azure", "/Azure_Public_Service_Icons")],
        }
    }

    /// The first words of a file name that a person can leave out.
    fn prefixes(self) -> &'static [&'static str] {
        match self {
            Cloud::Aws => &["amazon", "aws"],
            Cloud::Gcp => &["cloud", "google"],
            Cloud::Azure => &["azure", "microsoft"],
        }
    }

    /// Short names, and the normalized name that they stand for. A Google
    /// product with no icon of its own uses the icon of its category.
    fn aliases(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Cloud::Aws => &AWS_ALIASES,
            Cloud::Gcp => &GCP_ALIASES,
            Cloud::Azure => &AZURE_ALIASES,
        }
    }
}

const AWS_ALIASES: [(&str, &str); 20] = [
    ("alb", "elasticloadbalancingapplicationloadbalancer"),
    ("apigw", "apigateway"),
    ("ddb", "dynamodb"),
    ("dynamo", "dynamodb"),
    ("ebs", "elasticblockstore"),
    ("ecr", "elasticcontainerregistry"),
    ("ecs", "elasticcontainerservice"),
    ("eks", "elastickubernetesservice"),
    ("elb", "elasticloadbalancing"),
    ("iam", "identityandaccessmanagement"),
    ("kms", "keymanagementservice"),
    ("nlb", "elasticloadbalancingnetworkloadbalancer"),
    ("redis", "elasticache"),
    ("s3", "simplestorageservice"),
    ("secrets", "secretsmanager"),
    ("ses", "simpleemailservice"),
    ("sfn", "stepfunctions"),
    ("sns", "simplenotificationservice"),
    ("sqs", "simplequeueservice"),
    ("vpc", "virtualprivatecloud"),
];

const GCP_ALIASES: [(&str, &str); 41] = [
    ("aiplatform", "aimachinelearning"),
    ("appengine", "serverlesscomputing"),
    ("armor", "securityidentity"),
    ("artifactregistry", "developertools"),
    ("bigtable", "databases"),
    ("bq", "bigquery"),
    ("build", "devops"),
    ("cdn", "networking"),
    ("composer", "dataanalytics"),
    ("dataflow", "dataanalytics"),
    ("dataproc", "dataanalytics"),
    ("datastore", "databases"),
    ("deploy", "devops"),
    ("dns", "networking"),
    ("eventarc", "integrationservices"),
    ("filestore", "storage"),
    ("firestore", "databases"),
    ("functions", "serverlesscomputing"),
    ("gce", "computeengine"),
    ("gcs", "cloudstorage"),
    ("gemini", "aimachinelearning"),
    ("iam", "securityidentity"),
    ("iap", "securityidentity"),
    ("interconnect", "networking"),
    ("kms", "securityidentity"),
    ("lb", "networking"),
    ("loadbalancing", "networking"),
    ("logging", "observability"),
    ("memorystore", "databases"),
    ("monitoring", "observability"),
    ("nat", "networking"),
    ("pubsub", "integrationservices"),
    ("run", "cloudrun"),
    ("scheduler", "integrationservices"),
    ("secretmanager", "securityidentity"),
    ("sql", "cloudsql"),
    ("tasks", "integrationservices"),
    ("trace", "observability"),
    ("vm", "computeengine"),
    ("vpc", "networking"),
    ("workflows", "integrationservices"),
];

const AZURE_ALIASES: [(&str, &str); 14] = [
    ("aks", "kubernetesservices"),
    ("apim", "apimanagementservices"),
    ("appservice", "appservices"),
    ("blob", "storageaccounts"),
    ("cosmos", "cosmosdb"),
    ("eventhub", "eventhubs"),
    ("functions", "functionapps"),
    ("keyvault", "keyvaults"),
    ("rg", "resourcegroups"),
    ("sql", "sqldatabase"),
    ("storage", "storageaccounts"),
    ("vm", "virtualmachine"),
    ("vmss", "vmscalesets"),
    ("vnet", "virtualnetworks"),
];

/// Splits `aws/s3` into the cloud and the name. Other names give `None`.
pub fn split(name: &str) -> Option<(Cloud, &str)> {
    let (cloud, rest) = name.split_once('/')?;
    Some((Cloud::from_name(cloud)?, rest))
}

/// The kinds of the files in a package, in match order.
pub const RANKS: [&str; 4] = ["service", "resource", "category", "group"];

/// One icon file of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Index in `RANKS`. A lower rank wins a tie.
    pub rank: u8,
    /// Index in the packages of the cloud.
    pub part: usize,
    /// The name in the file name, such as `Amazon-Simple-Storage-Service`.
    pub name: String,
    /// The path in the zip.
    pub path: String,
}

/// The name and the rank of a zip path, or `None` when the file is not an icon
/// that the index keeps. Each package has its own folder layout.
pub fn entry_of(cloud: Cloud, part: usize, path: &str) -> Option<(u8, String)> {
    if path.starts_with("__MACOSX") || path.contains("_Dark") {
        return None;
    }
    let segs: Vec<&str> = path.split('/').collect();
    let file = segs[segs.len() - 1];
    let stem = file.strip_suffix(".svg").or_else(|| file.strip_suffix(".SVG"))?;
    let parent = |up: usize| segs.len().checked_sub(1 + up).map(|k| segs[k]);
    let name = match cloud {
        Cloud::Aws => {
            if let Some(n) = stem.strip_prefix("Arch-Category_") {
                (2, n.strip_suffix("_64")?)
            } else if let Some(n) = stem.strip_prefix("Arch_") {
                // The service icons come in four sizes. The 64 size has the most detail.
                (parent(1)? == "64").then_some(())?;
                (0, n.strip_suffix("_64")?)
            } else if let Some(n) = stem.strip_prefix("Res_") {
                (1, n.strip_suffix("_Light").unwrap_or(n).strip_suffix("_48")?)
            } else if segs.iter().any(|s| s.starts_with("Architecture-Group-Icons")) {
                (3, stem.strip_suffix("_32")?)
            } else {
                return None;
            }
        }
        Cloud::Gcp => {
            // `Unique Icons/<name>/SVG/<file>.svg` and `Category Icons/<name>/SVG/<file>.svg`.
            parent(1)?.eq_ignore_ascii_case("svg").then_some(())?;
            // A `/` in a category name is `_` in the folder name.
            let name = parent(2)?.replace(" _ ", " / ");
            return Some((if part == 0 { 0 } else { 2 }, name.trim().to_string()));
        }
        Cloud::Azure => (0, stem.split_once("-icon-service-")?.1),
    };
    Some((name.0, name.1.trim().to_string()))
}

/// Lower case, and only `a-z` and `0-9`.
fn norm(s: &str) -> String {
    s.chars().map(|c| c.to_ascii_lowercase()).filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// The normalized name, and the normalized name without the vendor prefix.
fn keys(cloud: Cloud, name: &str) -> (String, String) {
    let full = norm(name);
    let word = name.split(['-', '_', ' ']).next().unwrap_or("");
    let short = if cloud.prefixes().iter().any(|p| p.eq_ignore_ascii_case(word)) && word.len() < name.len() {
        norm(&name[word.len() + 1..])
    } else {
        full.clone()
    };
    (full, short)
}

/// The word without a plural `s`.
fn one(s: &str) -> &str {
    s.strip_suffix('s').filter(|r| r.len() >= 2).unwrap_or(s)
}

/// The best entry for a name. An exact name beats a name without the
/// vendor prefix, and both beat a match that ignores a plural `s`. A name
/// that starts with `:` finds only a group icon.
pub fn find<'a>(cloud: Cloud, entries: &'a [Entry], query: &str) -> Option<&'a Entry> {
    let group_only = query.starts_with(':');
    let mut q = norm(query);
    if let Some((_, real)) = cloud.aliases().iter().find(|(short, _)| *short == q) {
        q = real.to_string();
    }
    if q.is_empty() {
        return None;
    }
    entries
        .iter()
        .filter(|e| !group_only || e.rank == 3)
        .filter_map(|e| {
            let (full, short) = keys(cloud, &e.name);
            let score = if full == q {
                0
            } else if short == q {
                1
            } else if one(&full) == one(&q) || one(&short) == one(&q) {
                2
            } else {
                return None;
            };
            Some(((score, e.rank), e))
        })
        .min_by_key(|(key, _)| *key)
        .map(|(_, e)| e)
}

/// A name that a node can use: lower case with `-`, such as `amazon-simple-storage-service`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

const INDEX_HEAD: &str = "swe-diagrams vendor index 1\n";

/// The entries of all packages of a cloud. The index keeps the first path of
/// each name, in path order, so the result does not depend on the zip order.
pub fn build_index(cloud: Cloud, zips: &[Vec<u8>]) -> Option<Vec<Entry>> {
    let mut out = Vec::new();
    for (part, bytes) in zips.iter().enumerate() {
        for z in zip_entries(bytes)? {
            if let Some((rank, name)) = entry_of(cloud, part, &z.path) {
                out.push(Entry { rank, part, name, path: z.path });
            }
        }
    }
    out.sort_by(|a, b| (a.rank, a.part, &a.path).cmp(&(b.rank, b.part, &b.path)));
    let mut seen: Vec<(u8, String)> = Vec::new();
    out.retain(|e| {
        let key = (e.rank, norm(&e.name));
        let fresh = !seen.contains(&key);
        if fresh {
            seen.push(key);
        }
        fresh
    });
    Some(out)
}

fn write_index(entries: &[Entry]) -> String {
    let mut out = String::from(INDEX_HEAD);
    for e in entries {
        out.push_str(&format!("{}\t{}\t{}\t{}\n", e.rank, e.part, e.name, e.path));
    }
    out
}

fn read_index(text: &str) -> Option<Vec<Entry>> {
    let body = text.strip_prefix(INDEX_HEAD)?;
    body.lines()
        .map(|line| {
            let mut f = line.splitn(4, '\t');
            let rank = f.next()?.parse().ok().filter(|&r: &u8| (r as usize) < RANKS.len())?;
            let part = f.next()?.parse().ok()?;
            Some(Entry { rank, part, name: f.next()?.to_string(), path: f.next()?.to_string() })
        })
        .collect()
}

/// The zips and the index of one cloud.
pub struct Package {
    pub cloud: Cloud,
    pub entries: Vec<Entry>,
    zips: Vec<Vec<u8>>,
}

impl Package {
    /// Reads the package from `dir`. When a zip is not there and `offline`
    /// is false, the package is downloaded first. A missing index is built
    /// from the zips and written to `dir`.
    pub fn load(cloud: Cloud, dir: &Path, offline: bool) -> Option<Package> {
        let read = || -> Option<Vec<Vec<u8>>> {
            cloud.parts().iter().map(|(stem, _)| fs::read(dir.join(format!("{stem}.zip"))).ok()).collect()
        };
        let zips = match read() {
            Some(z) => z,
            None if !offline && download(cloud, dir) => read()?,
            None => return None,
        };
        let file = format!("{}.index", cloud.name());
        let entries = match fs::read_to_string(dir.join(&file)).ok().and_then(|t| read_index(&t)) {
            Some(e) if e.iter().all(|e| e.part < zips.len()) => e,
            _ => {
                let e = build_index(cloud, &zips)?;
                let _ = put(dir, &file, write_index(&e).as_bytes());
                e
            }
        };
        Some(Package { cloud, entries, zips })
    }

    /// The icon for a name, without the `aws/` prefix.
    pub fn icon(&self, query: &str) -> Option<Icon> {
        let e = find(self.cloud, &self.entries, query)?;
        let zip = &self.zips[e.part];
        let z = zip_entries(zip)?.into_iter().find(|z| z.path == e.path)?;
        let body = zip_read(zip, &z)?;
        let head = body.trim_ascii_start();
        (head.starts_with(b"<")).then(|| Icon {
            uri: format!("data:image/svg+xml;base64,{}", base64(&body)),
            dark_tile: false,
            // Only the AWS service, category, and group icons have a square.
            square: self.cloud == Cloud::Aws && e.rank != 1,
            source: self.cloud.credit().to_string(),
        })
    }
}

/// The vendor cache: `<icon dir>/vendor`.
pub fn cache_dir(opts: &IconOptions) -> Option<PathBuf> {
    opts.dir.clone().or_else(default_dir).map(|d| d.join("vendor"))
}

/// One result for each name, in order. The outer `None` means that the name
/// has no vendor prefix. The inner `None` means that the package has no icon
/// for the name, or that the package is not available.
pub fn resolve(names: &[String], opts: &IconOptions) -> Vec<Option<Option<Icon>>> {
    let mut out: Vec<Option<Option<Icon>>> = names.iter().map(|n| split(n).map(|_| None)).collect();
    let Some(dir) = cache_dir(opts) else { return out };
    for cloud in CLOUDS {
        if !names.iter().any(|n| split(n).is_some_and(|(c, _)| c == cloud)) {
            continue;
        }
        let Some(pkg) = Package::load(cloud, &dir, opts.offline) else { continue };
        for (name, slot) in names.iter().zip(out.iter_mut()) {
            if let Some((c, query)) = split(name) {
                if c == cloud {
                    *slot = Some(pkg.icon(query));
                }
            }
        }
    }
    out
}

/// The lines of `swe-diagrams icons <cloud> [filter]`: each icon name that
/// contains the filter, with its kind, then each short name that matches.
pub fn list(cloud: Cloud, filter: &str, opts: &IconOptions) -> Result<Vec<String>, String> {
    let dir = cache_dir(opts).ok_or("no icon directory: set SWE_DIAGRAMS_ICONS or HOME")?;
    let pkg = Package::load(cloud, &dir, opts.offline).ok_or_else(|| {
        format!("no {} package in {}. Run the command without --offline to download it", cloud.credit(), dir.display())
    })?;
    let f = norm(filter);
    let best = find(cloud, &pkg.entries, filter);
    // The entry that the filter finds as a name comes first.
    let mut rows: Vec<(bool, u8, String)> = pkg
        .entries
        .iter()
        .filter(|e| {
            let (full, short) = keys(cloud, &e.name);
            full.contains(&f) || short.contains(&f) || best == Some(*e)
        })
        .map(|e| (best != Some(e), e.rank, format!("{}/{}  {}", cloud.name(), slug(&e.name), RANKS[e.rank as usize])))
        .collect();
    rows.sort();
    rows.dedup();
    let mut out: Vec<String> = rows.into_iter().map(|(_, _, row)| row).collect();
    for (short, real) in cloud.aliases() {
        if !short.contains(&f) {
            continue;
        }
        if let Some(e) = find(cloud, &pkg.entries, short) {
            out.push(format!("{}/{short}  = {}/{}", cloud.name(), cloud.name(), slug(&e.name)));
        } else if f == *short {
            out.push(format!("{}/{short}  = {real}, which the package does not have", cloud.name()));
        }
    }
    Ok(out)
}

/// Downloads the packages of `cloud` into `dir`. A zip goes to a temporary
/// name first, so a parallel run never reads half of a file.
fn download(cloud: Cloud, dir: &Path) -> bool {
    let Some(page) = curl(cloud.page(), None, 30) else { return false };
    let page = String::from_utf8_lossy(&page);
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    for (stem, needle) in cloud.parts() {
        let Some(url) = zip_link(&page, needle) else { return false };
        let tmp = dir.join(format!(".{stem}.zip.{}.tmp", std::process::id()));
        let ok = curl(&url, Some(&tmp), 600).is_some()
            && fs::read(&tmp).ok().is_some_and(|b| zip_entries(&b).is_some())
            && fs::rename(&tmp, dir.join(format!("{stem}.zip"))).is_ok();
        if !ok {
            let _ = fs::remove_file(&tmp);
            return false;
        }
    }
    // The old index names the files of the old package.
    let _ = fs::remove_file(dir.join(format!("{}.index", cloud.name())));
    true
}

/// Runs `curl`. With `to`, the body goes to that file and the result is empty.
fn curl(url: &str, to: Option<&Path>, seconds: u32) -> Option<Vec<u8>> {
    // A URL from a page must not start with `-`, or `curl` reads it as an option.
    if !url.starts_with("https://") {
        return None;
    }
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--proto", "=https", "--proto-redir", "=https", "--max-time", &seconds.to_string()]);
    if let Some(to) = to {
        cmd.arg("-o").arg(to);
    }
    let out = cmd.arg(url).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then_some(out.stdout)
}

/// The first `https://...zip` link in the page that contains `needle`.
pub fn zip_link(page: &str, needle: &str) -> Option<String> {
    page.match_indices("https://").find_map(|(at, _)| {
        let rest = &page[at..];
        let end = rest.find(['"', '\'', '<', '>', ' ', '\t', '\n', '\r', '\\', '&', ')']).unwrap_or(rest.len());
        let url = &rest[..end];
        (url.ends_with(".zip") && url.contains(needle)).then(|| url.to_string())
    })
}

/// A file in a zip, from the central directory.
#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub path: String,
    method: u16,
    packed: usize,
    size: usize,
    offset: usize,
}

/// An icon is small. The limit stops a hostile zip from filling the memory.
const MAX_FILE: usize = 4 << 20;

fn le(b: &[u8], at: usize, n: usize) -> Option<usize> {
    let s = b.get(at..at.checked_add(n)?)?;
    Some(s.iter().rev().fold(0usize, |v, &x| v << 8 | x as usize))
}

/// The files of a zip, or `None` when the bytes are not a zip.
pub fn zip_entries(b: &[u8]) -> Option<Vec<ZipEntry>> {
    // The end record is 22 bytes plus a comment of at most 65535 bytes.
    let last = b.len().checked_sub(22)?;
    let first = last.saturating_sub(0xFFFF);
    let end = (first..=last).rev().find(|&i| b[i..i + 4] == *b"PK\x05\x06")?;
    let count = le(b, end + 10, 2)?;
    let mut at = le(b, end + 16, 4)?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if b.get(at..at + 4)? != b"PK\x01\x02" {
            return None;
        }
        let n = le(b, at + 28, 2)?;
        let path = b.get(at + 46..at + 46 + n)?;
        out.push(ZipEntry {
            path: String::from_utf8_lossy(path).into_owned(),
            method: le(b, at + 10, 2)? as u16,
            packed: le(b, at + 20, 4)?,
            size: le(b, at + 24, 4)?,
            offset: le(b, at + 42, 4)?,
        });
        at += 46 + n + le(b, at + 30, 2)? + le(b, at + 32, 2)?;
    }
    Some(out)
}

/// The bytes of one file. The sizes come from the central directory, because
/// a local header can hold zeros when the zip has data descriptors.
pub fn zip_read(b: &[u8], z: &ZipEntry) -> Option<Vec<u8>> {
    if z.size > MAX_FILE || b.get(z.offset..z.offset + 4)? != b"PK\x03\x04" {
        return None;
    }
    let start = z.offset + 30 + le(b, z.offset + 26, 2)? + le(b, z.offset + 28, 2)?;
    let data = b.get(start..start.checked_add(z.packed)?)?;
    match z.method {
        0 => Some(data.to_vec()),
        8 => miniz_oxide::inflate::decompress_to_vec_with_limit(data, MAX_FILE).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(cloud: Cloud, part: usize, paths: &[&str]) -> Vec<Entry> {
        paths
            .iter()
            .filter_map(|p| {
                entry_of(cloud, part, p).map(|(rank, name)| Entry { rank, part, name, path: p.to_string() })
            })
            .collect()
    }

    #[test]
    fn aws_paths_give_names_and_ranks() {
        let got = |p: &str| entry_of(Cloud::Aws, 0, p);
        let s = |r: u8, n: &str| Some((r, n.to_string()));
        assert_eq!(
            got("A_07/Arch_Storage/64/Arch_Amazon-Simple-Storage-Service_64.svg"),
            s(0, "Amazon-Simple-Storage-Service")
        );
        assert_eq!(got("A_07/Arch_Storage/32/Arch_Amazon-Simple-Storage-Service_32.svg"), None);
        assert_eq!(got("A_07/Arch_Storage/64/Arch_Amazon-Simple-Storage-Service_64.png"), None);
        assert_eq!(got("__MACOSX/A_07/Arch_Storage/64/._Arch_Amazon-EC2_64.svg"), None);
        assert_eq!(
            got("R_07/Res_Net/Res_Elastic-Load-Balancing_Application-Load-Balancer_48.svg"),
            s(1, "Elastic-Load-Balancing_Application-Load-Balancer")
        );
        assert_eq!(got("R_07/Res_General-Icons/Res_48_Light/Res_Server_48_Light.svg"), s(1, "Server"));
        assert_eq!(got("R_07/Res_General-Icons/Res_48_Dark/Res_Server_48_Dark.svg"), None);
        assert_eq!(got("C_07/Arch-Category_64/Arch-Category_Databases_64.svg"), s(2, "Databases"));
        assert_eq!(got("Architecture-Group-Icons_07/Region_32.svg"), s(3, "Region"));
        assert_eq!(got("Architecture-Group-Icons_07/AWS-Cloud_32_Dark.svg"), None);
    }

    #[test]
    fn gcp_and_azure_paths_give_names() {
        assert_eq!(
            entry_of(Cloud::Gcp, 0, "Unique Icons/Cloud Storage/SVG/Cloud_Storage-512-color.svg"),
            Some((0, "Cloud Storage".into()))
        );
        assert_eq!(
            entry_of(Cloud::Gcp, 1, "Category Icons/AI _ Machine Learning/SVG/AI-512-color.svg"),
            Some((2, "AI / Machine Learning".into()))
        );
        assert_eq!(entry_of(Cloud::Gcp, 0, "Unique Icons/GKE/PNG/GKE-512-color.png"), None);
        assert_eq!(entry_of(Cloud::Gcp, 0, "Unique Icons/GKE/SVG/.DS_Store"), None);
        assert_eq!(
            entry_of(
                Cloud::Azure,
                0,
                "Azure_Public_Service_Icons/Icons/compute/10021-icon-service-Virtual-Machine.svg"
            ),
            Some((0, "Virtual-Machine".into()))
        );
        assert_eq!(
            entry_of(Cloud::Azure, 0, "Azure_Public_Service_Icons/Icons/x/1-icon-service-Arc-PostgreSQL .svg"),
            Some((0, "Arc-PostgreSQL".into()))
        );
        assert_eq!(entry_of(Cloud::Azure, 0, "Azure_Public_Service_Icons/Azure_Icons_FAQ.pdf"), None);
    }

    #[test]
    fn find_takes_prefixes_plurals_case_and_aliases() {
        let aws = entries(
            Cloud::Aws,
            0,
            &[
                "S/A/64/Arch_Amazon-Simple-Storage-Service_64.svg",
                "S/A/64/Arch_Amazon-EC2_64.svg",
                "S/A/64/Arch_AWS-Lambda_64.svg",
                "S/A/64/Arch_Amazon-Virtual-Private-Cloud_64.svg",
                "R/R/Res_Elastic-Load-Balancing_Application-Load-Balancer_48.svg",
                "Architecture-Group-Icons_1/Region_32.svg",
                "Architecture-Group-Icons_1/Virtual-private-cloud-VPC_32.svg",
            ],
        );
        let name = |q: &str| find(Cloud::Aws, &aws, q).map(|e| e.name.as_str());
        assert_eq!(name("s3"), Some("Amazon-Simple-Storage-Service"));
        assert_eq!(name("Simple_Storage service"), Some("Amazon-Simple-Storage-Service"));
        assert_eq!(name("EC2"), Some("Amazon-EC2"));
        assert_eq!(name("ec2s"), Some("Amazon-EC2"));
        assert_eq!(name("aws-lambda"), Some("AWS-Lambda"));
        assert_eq!(name("alb"), Some("Elastic-Load-Balancing_Application-Load-Balancer"));
        assert_eq!(name("vpc"), Some("Amazon-Virtual-Private-Cloud"));
        assert_eq!(name(":virtual-private-cloud-vpc"), Some("Virtual-private-cloud-VPC"));
        assert_eq!(name(":ec2"), None);
        assert_eq!(name("region"), Some("Region"));
        assert_eq!(name("nothing"), None);
        assert_eq!(name(""), None);

        let gcp = [
            entries(Cloud::Gcp, 0, &["U/Cloud Storage/SVG/a.svg", "U/BigQuery/SVG/b.svg"]),
            entries(Cloud::Gcp, 1, &["C/Storage/SVG/c.svg", "C/Integration Services/SVG/d.svg"]),
        ]
        .concat();
        let name = |q: &str| find(Cloud::Gcp, &gcp, q).map(|e| e.name.as_str());
        assert_eq!(name("gcs"), Some("Cloud Storage"));
        assert_eq!(name("cloud-storage"), Some("Cloud Storage"));
        // The exact name of a category beats a product name without its prefix.
        assert_eq!(name("storage"), Some("Storage"));
        assert_eq!(name("bq"), Some("BigQuery"));
        assert_eq!(name("pubsub"), Some("Integration Services"));
    }

    #[test]
    fn each_alias_target_is_normalized() {
        for cloud in CLOUDS {
            for (short, real) in cloud.aliases() {
                assert_eq!(norm(short), *short);
                assert_eq!(norm(real), *real);
            }
            assert!(cloud.aliases().windows(2).all(|w| w[0].0 < w[1].0), "{cloud:?} aliases are not sorted");
        }
    }

    #[test]
    fn slug_joins_words_with_hyphens() {
        assert_eq!(slug("Amazon-Simple-Storage-Service"), "amazon-simple-storage-service");
        assert_eq!(slug("AI / Machine Learning"), "ai-machine-learning");
        assert_eq!(slug("Virtual-Machines-(Classic)"), "virtual-machines-classic");
    }

    #[test]
    fn zip_link_finds_the_package_link() {
        let page = r#"<a href="https://x.example/Microsoft-PPTx_1.zip">pptx</a>
            <a href='https://d1.example/release/Icon-package_07312026.5846.zip'>zip</a>
            "https://services.example/misc/google-cloud-legacy-icons.zip","https://services.example/misc/category-icons.zip""#;
        assert_eq!(
            zip_link(page, "/Icon-package_").as_deref(),
            Some("https://d1.example/release/Icon-package_07312026.5846.zip")
        );
        assert_eq!(
            zip_link(page, "/category-icons.zip").as_deref(),
            Some("https://services.example/misc/category-icons.zip")
        );
        assert_eq!(zip_link(page, "/core-products-icons.zip"), None);
    }

    #[test]
    fn index_text_reads_back() {
        let e = vec![Entry { rank: 3, part: 1, name: "Region".into(), path: "G/Region_32.svg".into() }];
        assert_eq!(read_index(&write_index(&e)), Some(e));
        assert_eq!(read_index("other\n"), None);
        assert_eq!(read_index(&format!("{INDEX_HEAD}9\t0\tx\ty\n")), None);
    }

    #[test]
    fn zip_reader_rejects_bytes_that_are_not_a_zip() {
        assert!(zip_entries(b"").is_none());
        assert!(zip_entries(&[0u8; 100]).is_none());
        let mut b = vec![0u8; 22];
        b[..4].copy_from_slice(b"PK\x05\x06");
        assert_eq!(zip_entries(&b).map(|v| v.len()), Some(0));
        // A count of one with no central record.
        b[10] = 1;
        assert!(zip_entries(&b).is_none());
    }
}
