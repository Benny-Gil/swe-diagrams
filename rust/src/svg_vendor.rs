//! The vendor look of `arch` and `change` inside a `:aws`, `:gcp`, or
//! `:azure` group. AWS icons carry their own square, so a node has no tile,
//! and AWS groups follow the AWS guide: square corners, outline only, and
//! the group icon in the top left corner. Google Cloud and Azure keep the
//! tiles, with the vendor blue on the outer group.

use std::fmt::Write;

use crate::icons::Icon;
use crate::model::{Item, GROUP_KINDS};
use crate::svg::{esc, st};
use crate::vendor::Cloud;

/// The side of an AWS icon that has no tile. The tile is 60.
pub const BARE: i64 = 56;

/// The style rules, light then dark.
pub const STYLE: (&str, &str) = (
    concat!(
        ".wg{fill:none;stroke-opacity:1;stroke-width:1.5}",
        ".wc{stroke:#232f3e}",
        ".wr{stroke:#00a4a6;stroke-dasharray:6 4}",
        ".wp{stroke:#8c4fff}",
        ".wu{stroke:#7aa116}",
        ".wi{stroke:#00a4a6}",
        ".wn{stroke:#7d8998;stroke-dasharray:6 4}",
        ".wb{fill:#e8f0fe;fill-opacity:.6;stroke:#4285f4}",
        ".wz{fill:#e5f1fb;fill-opacity:.6;stroke:#0078d4}",
        ".wo{stroke:#9aa1a9}",
        ".wt{fill:#192029}",
        ".wtb{fill:#1a73e8}",
        ".wtz{fill:#0062ad}",
    ),
    concat!(
        ".wc{stroke:#aab2bd}",
        ".wr,.wi{stroke:#2fc5c7}",
        ".wp{stroke:#a98bff}",
        ".wu{stroke:#9bc53a}",
        ".wn{stroke:#8a96a5}",
        ".wb{fill:#1a2a3f;fill-opacity:.35;stroke:#8ab4f8}",
        ".wz{fill:#132a3f;fill-opacity:.35;stroke:#4ba3f0}",
        ".wo{stroke:#6c737c}",
        ".wt{fill:#e8ebf1}",
        ".wtb{fill:#8ab4f8}",
        ".wtz{fill:#6cb6f5}",
    ),
);

/// The cloud of item `i`: the kind of the nearest vendor group that holds
/// the item, or of the item itself.
pub fn cloud_of(items: &[Item], i: usize) -> Option<Cloud> {
    let mut at = Some(i);
    while let Some(k) = at {
        if items[k].group {
            if let Some(cloud) = Cloud::from_name(GROUP_KINDS[items[k].group_kind]) {
                return Some(cloud);
            }
        }
        at = items[k].parent;
    }
    None
}

/// True when node `i` is in an AWS group, so its icon has no tile.
pub fn bare_tile(items: &[Item], i: usize) -> bool {
    !items[i].group && cloud_of(items, i) == Some(Cloud::Aws)
}

/// The vendor icon name of an AWS group with no `@` icon, such as
/// `aws/:region`. The `:` finds only the group icons of the package.
pub fn group_icon(items: &[Item], i: usize) -> Option<&'static str> {
    if !items[i].group || cloud_of(items, i) != Some(Cloud::Aws) {
        return None;
    }
    match GROUP_KINDS[items[i].group_kind] {
        "aws" => Some("aws/:aws-cloud-logo"),
        "region" => Some("aws/:region"),
        "vpc" | "vnet" => Some("aws/:virtual-private-cloud-vpc"),
        "public" => Some("aws/:public-subnet"),
        "private" | "subnet" => Some("aws/:private-subnet"),
        _ => None,
    }
}

/// The style class of the box of group `i`, and the corner radius. `None`
/// means that the group is not in a vendor group.
fn look(items: &[Item], i: usize) -> Option<(&'static str, i64)> {
    if !items[i].group {
        return None;
    }
    let kind = GROUP_KINDS[items[i].group_kind];
    Some(match cloud_of(items, i)? {
        Cloud::Aws => match kind {
            "aws" => ("wc", 0),
            "region" | "az" => ("wr", 0),
            "vpc" | "vnet" => ("wp", 0),
            "public" => ("wu", 0),
            "private" | "subnet" => ("wi", 0),
            _ => ("wn", 0),
        },
        Cloud::Gcp if kind == "gcp" => ("wb", 12),
        Cloud::Azure if kind == "azure" => ("wz", 12),
        _ => ("wo", 12),
    })
}

/// The box of group `i`, or `None` when the group has the normal look.
pub fn group_box(items: &[Item], i: usize, quiet: bool, x: i64, y: i64, w: i64, h: i64) -> Option<String> {
    let (class, rx) = look(items, i)?;
    let q = if quiet { " q" } else { "" };
    Some(format!(
        "<rect class=\"g wg {class}{}{q}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/>",
        st(items[i].status)
    ))
}

/// The icon and the label of group `i`, or `None` when the group has the
/// normal look. The caller draws the status badge.
pub fn group_label(items: &[Item], i: usize, icon: Option<&Icon>, quiet: bool, x: i64, y: i64) -> Option<String> {
    let (class, rx) = look(items, i)?;
    let it = &items[i];
    let mut out = String::new();
    let mut tx = x + 12;
    let mut ty = y + 24;
    let has_icon = !it.icon.is_empty() || group_icon(items, i).is_some();
    if rx == 0 {
        // The AWS guide puts the icon in the top left corner of the box.
        if let Some(ic) = icon {
            let _ = write!(out, "<image href=\"{}\" x=\"{x}\" y=\"{y}\" width=\"24\" height=\"24\"/>", ic.uri);
        }
        if has_icon {
            tx = x + 32;
        }
        ty = y + 17;
    } else if let Some(ic) = icon {
        // The plate keeps a dark mark visible on the dark theme.
        let _ = write!(
            out,
            "<rect class=\"gp\" x=\"{}\" y=\"{}\" width=\"18\" height=\"18\" rx=\"3\"/><image href=\"{}\" x=\"{tx}\" y=\"{}\" width=\"14\" height=\"14\"/>",
            tx - 2,
            y + 10,
            ic.uri,
            y + 12
        );
        tx += 20;
    }
    let label = match class {
        "wb" => "gl wtb",
        "wz" => "gl wtz",
        _ => "gl wt",
    };
    let q = if quiet { " q" } else { "" };
    let _ = write!(out, "<text class=\"{label}{q}\" x=\"{tx}\" y=\"{ty}\">{}</text>", esc(&it.label));
    Some(out)
}
