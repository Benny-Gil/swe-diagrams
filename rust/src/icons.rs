//! Logo lookup. A node can name a logo, such as `@postgres`, and this module
//! turns each name into a data URI that the SVG output can embed.
//! A disk cache keeps each logo and each miss, so a name uses the network once.
//! The fetch runs `curl`, so the crate needs no HTTP or TLS code.

use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where the cache is, and whether a cache miss can use the network.
#[derive(Debug, Clone, Default)]
pub struct IconOptions {
    /// The cache directory. `None` uses `default_dir()`.
    pub dir: Option<PathBuf>,
    /// True stops all network use.
    pub offline: bool,
}

/// A logo that is ready to embed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// A `data:image/svg+xml;base64,...` or `data:image/png;base64,...` URI.
    pub uri: String,
    /// The mark is white, so it needs a dark ground.
    pub dark_tile: bool,
    /// "svgl", "devicon", "gilbarbara", or "local".
    pub source: String,
}

const SVG: &str = "image/svg+xml";
const PNG: &str = "image/png";

/// A smaller body is an empty or broken file, not a logo.
const MIN_SVG: usize = 50;

/// One result for each name, in order. `None` means no logo, and the caller
/// draws a generic icon.
pub fn resolve(names: &[String], opts: &IconOptions) -> Vec<Option<Icon>> {
    let slugs: Vec<Option<String>> = names.iter().map(|n| slug(n).map(alias)).collect();
    let Some(dir) = opts.dir.clone().or_else(default_dir) else {
        return vec![None; names.len()];
    };
    let mut distinct: Vec<&str> = slugs.iter().flatten().map(String::as_str).collect();
    distinct.sort_unstable();
    distinct.dedup();
    let dir = dir.as_path();
    let found: Vec<Option<Icon>> = std::thread::scope(|s| {
        // A fetch waits on the network, so each slug gets its own thread.
        let jobs: Vec<_> = distinct
            .iter()
            .map(|&slug| {
                std::thread::Builder::new().spawn_scoped(s, move || lookup(dir, slug, opts.offline)).map_err(|_| slug)
            })
            .collect();
        jobs.into_iter()
            .map(|job| match job {
                Ok(handle) => handle.join().unwrap_or(None),
                // The system gave no thread, so do the work on this one.
                Err(slug) => lookup(dir, slug, opts.offline),
            })
            .collect()
    });
    slugs
        .iter()
        .map(|s| {
            let i = distinct.binary_search(&s.as_deref()?).ok()?;
            found[i].clone()
        })
        .collect()
}

/// `$SWE_DIAGRAMS_ICONS`, else `$XDG_CACHE_HOME/swe-diagrams/icons`, else
/// `$HOME/.cache/swe-diagrams/icons`. An empty variable counts as unset.
pub fn default_dir() -> Option<PathBuf> {
    let var = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    var("SWE_DIAGRAMS_ICONS")
        .or_else(|| var("XDG_CACHE_HOME").map(|d| d.join("swe-diagrams").join("icons")))
        .or_else(|| var("HOME").map(|d| d.join(".cache").join("swe-diagrams").join("icons")))
}

/// The slug goes into file paths and URLs. The small character set and the
/// `..` check stop a name from leaving the cache directory.
fn slug(name: &str) -> Option<String> {
    let s: String = name
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| matches!(c, 'a'..='z' | '0'..='9' | '-' | '_' | '.'))
        .collect();
    (!s.is_empty() && !s.contains("..")).then_some(s)
}

/// Short names that people write, and the slug that the sources use. The
/// cache uses the real slug, so `pg` and `postgres` share one file.
const ALIASES: [(&str, &str); 12] = [
    ("es", "elasticsearch"),
    ("gh", "github"),
    ("golang", "go"),
    ("js", "javascript"),
    ("k8s", "kubernetes"),
    ("mongo", "mongodb"),
    ("next", "nextjs"),
    ("node", "nodejs"),
    ("pg", "postgresql"),
    ("postgres", "postgresql"),
    ("tailwind", "tailwindcss"),
    ("ts", "typescript"),
];

fn alias(slug: String) -> String {
    match ALIASES.iter().find(|(short, _)| *short == slug) {
        Some((_, real)) => real.to_string(),
        None => slug,
    }
}

fn lookup(dir: &Path, slug: &str, offline: bool) -> Option<Icon> {
    for (ext, mime) in [("svg", SVG), ("png", PNG)] {
        match fs::read(dir.join(format!("{slug}.{ext}"))) {
            Ok(body) => return Some(icon(&body, mime, cached_source(dir, slug))),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    if offline || dir.join(format!("{slug}.miss")).exists() {
        return None;
    }
    let mut net = Net::default();
    if let Some((body, source)) = fetch(&mut net, slug) {
        // The `.src` file goes first, so a reader that sees the logo also sees its source.
        let _ =
            put(dir, &format!("{slug}.src"), source.as_bytes()).and_then(|()| put(dir, &format!("{slug}.svg"), &body));
        return Some(icon(&body, SVG, source.to_string()));
    }
    // A failed transfer does not prove that no logo exists, so only a clear
    // answer from each server writes the `.miss` file.
    if !net.broken && !net.no_curl {
        let _ = put(dir, &format!("{slug}.miss"), b"");
    }
    None
}

fn icon(body: &[u8], mime: &str, source: String) -> Icon {
    let dark_tile = mime == SVG && white_mark(&String::from_utf8_lossy(body));
    Icon { uri: format!("data:{mime};base64,{}", base64(body)), dark_tile, source }
}

/// A file that a person puts in the directory has no `.src` file.
fn cached_source(dir: &Path, slug: &str) -> String {
    fs::read_to_string(dir.join(format!("{slug}.src")))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "local".to_string())
}

/// The write goes to a temporary name first, so a parallel run never reads
/// half of a file.
fn put(dir: &Path, file: &str, body: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{file}.{}.tmp", std::process::id()));
    fs::write(&tmp, body)?;
    fs::rename(&tmp, dir.join(file)).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// The sources in order. The first body that is an SVG wins.
fn fetch(net: &mut Net, slug: &str) -> Option<(Vec<u8>, &'static str)> {
    let search = format!("https://api.svgl.app?search={}", url_encode(&slug.replace('-', " ")));
    let svgl = net.get(&search).and_then(|body| svgl_route(&body, slug)).and_then(|route| net.get(&route));
    if let Some(body) = svgl.filter(|b| is_svg(b)) {
        return Some((body, "svgl"));
    }
    let urls = [
        (
            format!("https://raw.githubusercontent.com/devicons/devicon/master/icons/{slug}/{slug}-original.svg"),
            "devicon",
        ),
        (format!("https://cdn.jsdelivr.net/gh/gilbarbara/logos/logos/{slug}-icon.svg"), "gilbarbara"),
        (format!("https://cdn.jsdelivr.net/gh/gilbarbara/logos/logos/{slug}.svg"), "gilbarbara"),
    ];
    urls.into_iter().find_map(|(url, source)| net.get(&url).filter(|b| is_svg(b)).map(|b| (b, source)))
}

/// Runs `curl` and records how the transfers fail.
#[derive(Default)]
struct Net {
    /// A transfer failed without a clear "not found" from the server.
    broken: bool,
    /// `curl` did not start. After that, the lookup acts as offline.
    no_curl: bool,
}

impl Net {
    fn get(&mut self, url: &str) -> Option<Vec<u8>> {
        // A URL from the svgl body must not start with `-`, or `curl` reads it as an option.
        if self.no_curl || !url.starts_with("https://") {
            return None;
        }
        let out = Command::new("curl")
            .args(["-fsSL", "--max-time", "8", "--proto", "=https", url])
            .stdin(Stdio::null())
            .output();
        match out {
            Ok(out) if out.status.success() => Some(out.stdout),
            Ok(out) => {
                self.broken |= !http_error(&out.stderr)
                    .is_some_and(|code| (400..500).contains(&code) && code != 408 && code != 429);
                None
            }
            Err(_) => {
                self.no_curl = true;
                None
            }
        }
    }
}

/// The status in `curl: (22) The requested URL returned error: 404`. Some
/// `curl` versions give exit code 56 for this error over HTTP/2, so the exit
/// code cannot show that the server answered.
fn http_error(stderr: &[u8]) -> Option<u16> {
    let text = String::from_utf8_lossy(stderr);
    let key = "returned error: ";
    let rest = &text[text.find(key)? + key.len()..];
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

/// A server can send an error page with status 200, so the body must look
/// like an SVG document.
fn is_svg(body: &[u8]) -> bool {
    let text = body.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(body).trim_ascii_start();
    let head = &text[..text.len().min(1024)];
    let has = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w.eq_ignore_ascii_case(needle));
    body.len() >= MIN_SVG
        && head.first() == Some(&b'<')
        && !has(head, b"<html")
        && (has(head, b"<svg") || head.starts_with(b"<?xml") && has(text, b"<svg"))
}

/// True when each paint value is white. A white mark is not visible on a white tile.
fn white_mark(svg: &str) -> bool {
    let low = svg.to_ascii_lowercase();
    let mut white = false;
    for value in paints(&low) {
        match value {
            "" | "none" | "transparent" => {}
            _ if value.starts_with("url(") => {}
            "#fff" | "#ffffff" | "white" => white = true,
            _ => return false,
        }
    }
    white
}

/// The values of `fill="..."`, `stroke="..."`, `fill:...` and `stroke:...` in
/// lower-case SVG text.
fn paints(low: &str) -> Vec<&str> {
    let b = low.as_bytes();
    let mut out = Vec::new();
    for key in ["fill", "stroke"] {
        for (at, _) in low.match_indices(key) {
            // This skips words that contain the key, such as `refill`.
            if at > 0 && (b[at - 1].is_ascii_alphanumeric() || matches!(b[at - 1], b'-' | b'_')) {
                continue;
            }
            // A name such as `fill-rule` has no `=` or `:` after the key.
            let rest = low[at + key.len()..].trim_start();
            let value = if let Some(r) = rest.strip_prefix('=') {
                let r = r.trim_start();
                let Some(q) = r.chars().next().filter(|c| matches!(c, '"' | '\'')) else { continue };
                let r = &r[1..];
                &r[..r.find(q).unwrap_or(r.len())]
            } else if let Some(r) = rest.strip_prefix(':') {
                &r[..r.find([';', '"', '\'', '}', '<', '>', '\n']).unwrap_or(r.len())]
            } else {
                continue;
            };
            out.push(value.trim().trim_end_matches("!important").trim_end());
        }
    }
    out
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Standard alphabet, with padding.
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (k, &b)| n | (b as u32) << (16 - 8 * k));
        for k in 0..4 {
            out.push(if k <= chunk.len() { ABC[(n >> (18 - 6 * k)) as usize & 63] as char } else { '=' });
        }
    }
    out
}

/// The search is fuzzy, so only an exact match of the normalized title is a hit.
fn svgl_route(body: &[u8], slug: &str) -> Option<String> {
    let want = norm(slug);
    let short = want.strip_suffix("js").filter(|s| !s.is_empty());
    if want.is_empty() {
        return None;
    }
    let Json::Arr(items) = Json::parse(std::str::from_utf8(body).ok()?)? else { return None };
    items.iter().find_map(|item| {
        let title = norm(item.get("title")?.str()?);
        if title != want && Some(title.as_str()) != short {
            return None;
        }
        let route = item.get("route")?;
        // The tiles are white, so the light variant is correct.
        route.str().or_else(|| route.get("light")?.str()).map(str::to_string)
    })
}

/// Lower case, and only `a-z` and `0-9`.
fn norm(s: &str) -> String {
    s.chars().map(|c| c.to_ascii_lowercase()).filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// A small JSON tree. The title match needs only strings, arrays and objects,
/// so null, true, false and numbers keep no value.
enum Json {
    Scalar,
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn parse(text: &str) -> Option<Json> {
        let mut r = Reader { s: text.as_bytes(), i: 0, depth: 0 };
        let value = r.value()?;
        r.ws();
        (r.i == r.s.len()).then_some(value)
    }

    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// The reader stops at this depth, so a hostile body cannot overflow the stack.
const MAX_DEPTH: u32 = 64;

struct Reader<'a> {
    s: &'a [u8],
    i: usize,
    depth: u32,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while matches!(self.s.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.ws();
        let hit = self.s.get(self.i) == Some(&byte);
        if hit {
            self.i += 1;
        }
        hit
    }

    fn value(&mut self) -> Option<Json> {
        self.ws();
        match *self.s.get(self.i)? {
            b'"' => self.string().map(Json::Str),
            b'[' => {
                self.open()?;
                let mut items = Vec::new();
                if !self.eat(b']') {
                    loop {
                        items.push(self.value()?);
                        if self.eat(b']') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                self.depth -= 1;
                Some(Json::Arr(items))
            }
            b'{' => {
                self.open()?;
                let mut fields = Vec::new();
                if !self.eat(b'}') {
                    loop {
                        self.ws();
                        let key = self.string()?;
                        if !self.eat(b':') {
                            return None;
                        }
                        fields.push((key, self.value()?));
                        if self.eat(b'}') {
                            break;
                        }
                        if !self.eat(b',') {
                            return None;
                        }
                    }
                }
                self.depth -= 1;
                Some(Json::Obj(fields))
            }
            b't' => self.word(b"true"),
            b'f' => self.word(b"false"),
            b'n' => self.word(b"null"),
            _ => self.number(),
        }
    }

    fn open(&mut self) -> Option<()> {
        self.i += 1;
        self.depth += 1;
        (self.depth <= MAX_DEPTH).then_some(())
    }

    fn word(&mut self, w: &[u8]) -> Option<Json> {
        if !self.s[self.i..].starts_with(w) {
            return None;
        }
        self.i += w.len();
        Some(Json::Scalar)
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while matches!(self.s.get(self.i), Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')) {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).ok()?.parse::<f64>().ok().map(|_| Json::Scalar)
    }

    fn string(&mut self) -> Option<String> {
        if self.s.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let b = *self.s.get(self.i)?;
            self.i += 1;
            match b {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode()?,
                        _ => return None,
                    };
                    out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                }
                0..=0x1f => return None,
                _ => out.push(b),
            }
        }
    }

    /// The part after `\u`. Two escaped surrogates give one character.
    fn unicode(&mut self) -> Option<char> {
        let hi = self.hex4()?;
        if (0xD800..0xDC00).contains(&hi) && self.s[self.i..].starts_with(b"\\u") {
            let back = self.i;
            self.i += 2;
            let lo = self.hex4()?;
            if (0xDC00..0xE000).contains(&lo) {
                return char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00));
            }
            self.i = back;
        }
        Some(char::from_u32(hi).unwrap_or('\u{FFFD}'))
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.i..self.i + 4)?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_maps_a_short_name_after_the_slug_rule() {
        let real = |name: &str| slug(name).map(alias);
        assert_eq!(real("postgres").as_deref(), Some("postgresql"));
        assert_eq!(real("@PG").as_deref(), Some("postgresql"));
        assert_eq!(real("K8s!").as_deref(), Some("kubernetes"));
        assert_eq!(real("golang").as_deref(), Some("go"));
        assert_eq!(real("next").as_deref(), Some("nextjs"));
        // A real slug and a name that only contains an alias stay as they are.
        assert_eq!(real("postgresql").as_deref(), Some("postgresql"));
        assert_eq!(real("nodes").as_deref(), Some("nodes"));
        assert_eq!(real("../pg"), None);
        // Each target obeys the slug rule, and no target is itself an alias.
        for (short, target) in ALIASES {
            assert_eq!(slug(short).as_deref(), Some(short));
            assert_eq!(slug(target).as_deref(), Some(target));
            assert_eq!(alias(target.to_string()), target);
        }
    }

    const SEARCH: &str = r#"[
        {"id": 1, "title": "PostgreSQL", "category": "Database",
         "route": "https://svgl.app/library/postgresql.svg", "url": "https://www.postgresql.org/"},
        {"id": 2, "title": "Next.js", "category": ["Framework", "Vercel"],
         "route": {"light": "https://svgl.app/library/nextjs_icon_dark.svg",
                   "dark": "https://svgl.app/library/nextjs_icon_light.svg"}},
        {"id": 3, "title": "Vue", "route": "https://svgl.app/library/vue.svg", "brand": null},
        {"id": 4.5e0, "title": "Google \"Cloud\"", "route": "https:\/\/svgl.app\/library\/google-cloud.svg",
         "wordmark": false, "fresh": true}
    ]"#;

    fn route(slug: &str) -> Option<String> {
        svgl_route(SEARCH.as_bytes(), slug)
    }

    #[test]
    fn svgl_takes_only_an_exact_title() {
        assert_eq!(route("postgresql").as_deref(), Some("https://svgl.app/library/postgresql.svg"));
        // The search gives "PostgreSQL" for "postgres", but that is a loose match.
        assert_eq!(route("postgres"), None);
        assert_eq!(route("vue").as_deref(), Some("https://svgl.app/library/vue.svg"));
        assert_eq!(route("vuejs").as_deref(), Some("https://svgl.app/library/vue.svg"));
        assert_eq!(route("google-cloud").as_deref(), Some("https://svgl.app/library/google-cloud.svg"));
        assert_eq!(route("svelte"), None);
        assert_eq!(route("-"), None);
    }

    #[test]
    fn svgl_takes_the_light_route() {
        assert_eq!(route("nextjs").as_deref(), Some("https://svgl.app/library/nextjs_icon_dark.svg"));
    }

    #[test]
    fn svgl_rejects_a_body_that_is_not_an_array() {
        assert_eq!(svgl_route(b"{\"title\": \"Vue\", \"route\": \"https://x\"}", "vue"), None);
        assert_eq!(svgl_route(b"[{\"title\": \"Vue\", \"route\": ", "vue"), None);
        assert_eq!(svgl_route(b"<html>rate limit</html>", "vue"), None);
        assert_eq!(svgl_route(b"[] []", "vue"), None);
    }

    #[test]
    fn json_reads_escapes() {
        let Some(Json::Str(s)) = Json::parse(r#""a\"b\\c\/d\n\té😀\ud800x""#) else { panic!() };
        assert_eq!(s, "a\"b\\c/d\n\té😀\u{FFFD}x");
        assert!(Json::parse("\"a\u{1}b\"").is_none());
        assert!(Json::parse(r#""\q""#).is_none());
    }

    #[test]
    fn json_reads_scalars_and_stops_deep_nesting() {
        let Some(Json::Arr(items)) = Json::parse(" [1, -2.5e3, true, false, null, {}, []] ") else { panic!() };
        assert_eq!(items.len(), 7);
        assert!(Json::parse("[tru]").is_none());
        assert!(Json::parse("[1,]").is_none());
        let ok = format!("{}{}", "[".repeat(64), "]".repeat(64));
        let deep = format!("{}{}", "[".repeat(65), "]".repeat(65));
        assert!(Json::parse(&ok).is_some());
        assert!(Json::parse(&deep).is_none());
    }

    #[test]
    fn svg_test_rejects_short_and_html_bodies() {
        let pad = " ".repeat(50);
        assert!(is_svg(format!("<svg xmlns=\"http://www.w3.org/2000/svg\">{pad}</svg>").as_bytes()));
        assert!(is_svg(format!("\u{feff}\n<?xml version=\"1.0\"?><!-- {pad} --><SVG></SVG>").as_bytes()));
        assert!(!is_svg(b"<svg/>"));
        assert!(!is_svg(format!("<!doctype html><html><svg>{pad}</svg></html>").as_bytes()));
        assert!(!is_svg(format!("{{\"svg\": \"<svg>{pad}\"}}").as_bytes()));
        assert!(!is_svg(format!("<?xml version=\"1.0\"?><Error>{pad}</Error>").as_bytes()));
    }

    #[test]
    fn url_encode_keeps_unreserved_bytes() {
        assert_eq!(url_encode("google cloud"), "google%20cloud");
        assert_eq!(url_encode("a-b_c.d~e/f"), "a-b_c.d~e%2Ff");
    }

    #[test]
    fn http_error_reads_the_status_from_curl() {
        assert_eq!(http_error(b"curl: (22) The requested URL returned error: 404\n"), Some(404));
        assert_eq!(http_error(b"curl: (56) The requested URL returned error: 404\n"), Some(404));
        assert_eq!(http_error(b"curl: (22) The requested URL returned error: 503 Service Unavailable\n"), Some(503));
        assert_eq!(http_error(b"curl: (6) Could not resolve host: api.svgl.app\n"), None);
        assert_eq!(http_error(b""), None);
    }

    #[test]
    fn get_refuses_a_url_that_curl_can_read_as_an_option() {
        let mut net = Net::default();
        assert_eq!(net.get("-o/tmp/x"), None);
        assert_eq!(net.get("http://example.com"), None);
        assert!(!net.broken && !net.no_curl);
    }
}
