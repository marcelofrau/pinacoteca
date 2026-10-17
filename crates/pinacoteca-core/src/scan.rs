//! Folder scanning for the catalog.
//!
//! The library is built by walking folders and discovering image files. Scanning
//! is deliberately separate from decoding: a scan only reads cheap filesystem
//! metadata, so it can stream results to the UI while decoding happens later on
//! worker threads (ADR-0005).
//!
//! Original files are only ever read, never modified.

use std::path::{Path, PathBuf};

/// Image file extensions the catalog recognises, lowercase, without the dot.
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// A photo discovered on disk, before any decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPhoto {
    /// Absolute or root-relative path of the image file.
    pub path: PathBuf,
    /// File size in bytes.
    pub len: u64,
    /// Modification time, seconds since the Unix epoch.
    pub modified_secs: i64,
    /// Modification time, nanosecond part.
    pub modified_nanos: u32,
}

/// Summary of a finished scan.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScanStats {
    /// Number of image files discovered.
    pub photos: usize,
    /// Number of directories visited.
    pub directories: usize,
    /// Number of files that could not be read (permission, I/O).
    pub errors: usize,
}

/// Whether a path has a recognised image extension (case-insensitive).
pub fn supports_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            IMAGE_EXTENSIONS.contains(&ext.as_str())
        })
}

/// Whether a directory should be skipped during a scan.
///
/// Hidden directories (leading `.`) and Picasa's own `Originals` folder are
/// skipped: the former are not user photos, the latter holds edited originals
/// rather than the photos themselves.
fn skip_directory(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("Originals")
}

/// Scans `root` recursively, calling `on_photo` for every image file found.
///
/// Results stream as they are discovered, so a caller can update the UI
/// incrementally. Files that cannot be stat'ed are counted in
/// [`ScanStats::errors`] and skipped, not fatal.
pub fn scan_folder(
    root: &Path,
    on_photo: &mut impl FnMut(DiscoveredPhoto),
) -> std::io::Result<ScanStats> {
    let mut stats = ScanStats::default();
    walk(root, on_photo, &mut stats)?;
    Ok(stats)
}

fn walk(
    dir: &Path,
    on_photo: &mut impl FnMut(DiscoveredPhoto),
    stats: &mut ScanStats,
) -> std::io::Result<()> {
    stats.directories += 1;

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                stats.errors += 1;
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                stats.errors += 1;
                continue;
            }
        };

        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !skip_directory(&name) {
                walk(&path, on_photo, stats)?;
            }
        } else if file_type.is_file() && supports_extension(&path) {
            match entry.metadata() {
                Ok(metadata) => {
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .unwrap_or_default();
                    on_photo(DiscoveredPhoto {
                        path,
                        len: metadata.len(),
                        modified_secs: modified.as_secs() as i64,
                        modified_nanos: modified.subsec_nanos(),
                    });
                    stats.photos += 1;
                }
                Err(_) => stats.errors += 1,
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pinacoteca-scan-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn recognises_image_extensions_case_insensitively() {
        assert!(supports_extension(Path::new("a.jpg")));
        assert!(supports_extension(Path::new("a.JPEG")));
        assert!(supports_extension(Path::new("a.PNG")));
        assert!(!supports_extension(Path::new("a.gif")));
        assert!(!supports_extension(Path::new("a.txt")));
        assert!(!supports_extension(Path::new("noext")));
    }

    #[test]
    fn scans_recursively_and_streams_photos() {
        let root = temp_dir("recursive");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::create_dir_all(root.join("Originals")).unwrap();
        std::fs::write(root.join("a.jpg"), b"x").unwrap();
        std::fs::write(root.join("sub/b.PNG"), b"xx").unwrap();
        std::fs::write(root.join("sub/c.txt"), b"ignore").unwrap();
        std::fs::write(root.join(".hidden/d.jpg"), b"ignore").unwrap();
        std::fs::write(root.join("Originals/e.jpg"), b"ignore").unwrap();

        let mut found = Vec::new();
        let stats = scan_folder(&root, &mut |photo| found.push(photo)).unwrap();

        let mut names: Vec<String> = found
            .iter()
            .map(|photo| {
                photo
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        assert_eq!(names, vec!["a.jpg", "b.PNG"]);
        assert_eq!(stats.photos, 2);
        assert_eq!(stats.errors, 0);

        // Size is captured from metadata.
        let a = found.iter().find(|p| p.path.ends_with("a.jpg")).unwrap();
        assert_eq!(a.len, 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_root_is_not_an_error() {
        let mut count = 0;
        let stats = scan_folder(Path::new("definitely/not/here"), &mut |_| count += 1).unwrap();
        assert_eq!(count, 0);
        assert_eq!(stats.photos, 0);
    }
}
