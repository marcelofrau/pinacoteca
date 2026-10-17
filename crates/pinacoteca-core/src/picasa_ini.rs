//! Reader for Picasa's per-folder `.picasa.ini` metadata files.
//!
//! Picasa kept the authoritative, human-readable metadata next to the photos:
//! one `Picasa.ini` (or `.picasa.ini`) per folder, holding a `[Picasa]` section
//! for the folder itself and one `[<filename>]` section per photo.
//!
//! This module only *reads*. Pinacoteca imports the user state it understands
//! (stars, captions, keywords, hidden flag) and never rewrites the file, so an
//! existing Picasa library keeps working and unknown keys are preserved by
//! simply not being touched.
//!
//! Format facts recovered during the M1 analysis:
//!
//! - plain INI, CRLF line endings, **case-insensitive** keys;
//! - the `albums=` value holds opaque album tokens, not names — the names live
//!   in Picasa's obfuscated `albums_0.db`, so membership cannot be resolved
//!   from the INI alone and is parsed but not imported;
//! - `[Contacts2]` holds people, which this project does not model.

use std::path::Path;

/// Metadata for one folder, from a `.picasa.ini`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PicasaIni {
    /// The `[Picasa]` section, if present.
    pub folder: FolderSection,
    /// One entry per photo section, in file order.
    pub photos: Vec<PhotoSection>,
}

/// The folder-level `[Picasa]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderSection {
    /// Display name Picasa used for the folder.
    pub name: Option<String>,
    /// Folder date as Picasa wrote it (a string; Picasa allowed free text).
    pub date: Option<String>,
}

/// One `[<filename>]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhotoSection {
    /// The section name: the photo's file name, not a full path.
    pub file: String,
    /// `star=`, clamped to 0..=5.
    pub star: Option<u8>,
    /// `caption=`.
    pub caption: Option<String>,
    /// `keywords=`, split on commas.
    pub keywords: Vec<String>,
    /// `hidden=yes`.
    pub hidden: bool,
    /// `rotate=`, Picasa's 0..=3 quarter-turn count.
    pub rotate: Option<u8>,
    /// `albums=`, kept as opaque tokens.
    pub albums: Vec<String>,
}

/// Parses `.picasa.ini` text.
pub fn parse(text: &str) -> PicasaIni {
    let mut ini = PicasaIni::default();
    // `None` until the first section, so stray keys before any `[...]` are
    // ignored rather than attributed to a photo.
    let mut section: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let name = name.trim().to_string();
            if name.eq_ignore_ascii_case("Picasa") {
                section = Some(name);
            } else if name.eq_ignore_ascii_case("Contacts2") {
                // People are out of scope; skip the section entirely.
                section = None;
            } else {
                ini.photos.push(PhotoSection {
                    file: name.clone(),
                    ..PhotoSection::default()
                });
                section = Some(name);
            }
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let Some(current) = section.as_deref() else {
            continue;
        };

        if current.eq_ignore_ascii_case("Picasa") {
            if key.eq_ignore_ascii_case("name") {
                ini.folder.name = Some(value.to_string());
            } else if key.eq_ignore_ascii_case("date") {
                ini.folder.date = Some(value.to_string());
            }
            continue;
        }

        let Some(photo) = ini
            .photos
            .iter_mut()
            .find(|photo| photo.file.eq_ignore_ascii_case(current))
        else {
            continue;
        };

        if key.eq_ignore_ascii_case("star") {
            photo.star = value.parse::<u8>().ok().map(|star| star.min(5));
        } else if key.eq_ignore_ascii_case("caption") {
            if !value.is_empty() {
                photo.caption = Some(value.to_string());
            }
        } else if key.eq_ignore_ascii_case("keywords") {
            photo.keywords = split_list(value);
        } else if key.eq_ignore_ascii_case("hidden") {
            photo.hidden = is_true(value);
        } else if key.eq_ignore_ascii_case("rotate") {
            photo.rotate = value.parse::<u8>().ok().map(|turn| turn % 4);
        } else if key.eq_ignore_ascii_case("albums") {
            photo.albums = split_list(value);
        }
    }

    ini
}

/// Reads and parses a `.picasa.ini`, returning `None` when it does not exist.
pub fn read(path: &Path) -> std::io::Result<Option<PicasaIni>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(parse(&text))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_true(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "yes" | "true" | "1" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[Picasa]\r
name=2001-2002\r
P2category=Folder on disk\r
\r
[IMG_0001.JPG]\r
star=4\r
caption=Family at the beach\r
keywords=family, beach, summer\r
backuphash=12345\r
\r
[IMG_0002.JPG]\r
hidden=yes\r
star=9\r
rotate=5\r
albums=abc123,def456\r
\r
[Contacts2]\r
f4163e31ab8f87ec=Someone\r
";

    #[test]
    fn reads_folder_and_photo_sections() {
        let ini = parse(SAMPLE);
        assert_eq!(ini.folder.name.as_deref(), Some("2001-2002"));
        assert_eq!(ini.folder.date, None);
        assert_eq!(ini.photos.len(), 2);
    }

    #[test]
    fn clamps_and_modularises_numeric_keys() {
        let ini = parse(SAMPLE);
        // star=9 is out of range and clamps to 5.
        assert_eq!(ini.photos[1].star, Some(5));
        // rotate=5 wraps to 1.
        assert_eq!(ini.photos[1].rotate, Some(1));
    }

    #[test]
    fn splits_keywords_and_albums() {
        let ini = parse(SAMPLE);
        assert_eq!(ini.photos[0].keywords, ["family", "beach", "summer"]);
        assert_eq!(ini.photos[1].albums, ["abc123", "def456"]);
    }

    #[test]
    fn reads_caption_and_hidden() {
        let ini = parse(SAMPLE);
        assert_eq!(
            ini.photos[0].caption.as_deref(),
            Some("Family at the beach")
        );
        assert!(ini.photos[1].hidden);
        assert!(!ini.photos[0].hidden);
    }

    #[test]
    fn keys_and_sections_are_case_insensitive() {
        let ini = parse("[PICASA]\nNAME=x\n[img_0009.jpg]\nSTAR=3\n");
        assert_eq!(ini.folder.name.as_deref(), Some("x"));
        assert_eq!(ini.photos[0].file, "img_0009.jpg");
        assert_eq!(ini.photos[0].star, Some(3));
    }

    #[test]
    fn keys_before_any_section_are_ignored() {
        let ini = parse("star=5\n[Picasa]\nname=x\n");
        assert_eq!(ini.folder.name.as_deref(), Some("x"));
        assert!(ini.photos.is_empty());
    }

    #[test]
    fn missing_file_reads_as_none() {
        let missing = std::env::temp_dir().join("pinacoteca-does-not-exist.ini");
        let _ = std::fs::remove_file(&missing);
        assert_eq!(read(&missing).expect("no io error"), None);
    }
}
