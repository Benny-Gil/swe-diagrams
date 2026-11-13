//! The output formats of the command line.

use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Svg,
    Html,
    Png,
    Txt,
    Gif,
    Mp4,
}

impl Format {
    pub fn from_name(name: &str) -> Option<Format> {
        match name {
            "svg" => Some(Format::Svg),
            "html" => Some(Format::Html),
            "png" => Some(Format::Png),
            "txt" => Some(Format::Txt),
            "gif" => Some(Format::Gif),
            "mp4" => Some(Format::Mp4),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Html => "html",
            Format::Png => "png",
            Format::Txt => "txt",
            Format::Gif => "gif",
            Format::Mp4 => "mp4",
        }
    }

    /// The format of an output file. An unknown extension gives SVG, as in
    /// the first versions.
    pub fn from_path(path: &Path) -> Format {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "htm" => Format::Html,
            e => Format::from_name(e).unwrap_or(Format::Svg),
        }
    }

    /// A binary format cannot go to a terminal, so it needs a file.
    pub fn binary(self) -> bool {
        matches!(self, Format::Png | Format::Gif | Format::Mp4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_gives_the_format() {
        assert_eq!(Format::from_path(Path::new("a/b.png")), Format::Png);
        assert_eq!(Format::from_path(Path::new("b.HTML")), Format::Html);
        assert_eq!(Format::from_path(Path::new("b.htm")), Format::Html);
        assert_eq!(Format::from_path(Path::new("b.diagram")), Format::Svg);
        assert_eq!(Format::from_path(Path::new("b")), Format::Svg);
    }
}
