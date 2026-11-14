//! Unprivileged, bounded extraction of a downloaded GRUB theme.
//! Never unpack untrusted archives inside the Polkit/root helper.
use crate::core;
use flate2::read::GzDecoder;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

const MAX_ARCHIVE_BYTES: u64 = 80_000_000;
const MAX_ITEMS: usize = 2_000;
const MAX_FILES: usize = 500;
const MAX_EXTRACTED_BYTES: u64 = 40_000_000;
const MAX_THEME_TEXT: u64 = 512_000;
const ALLOWED_SUFFIXES: &[&str] = &["txt", "pf2", "png", "jpg", "jpeg", "tga", "fnt", "bmp"];

#[derive(Clone, Copy)]
enum Format { Zip, TarGz, TarXz }

#[derive(Debug)]
pub struct PreparedTheme {
    pub folder: PathBuf,
    pub name: String,
    pub files: usize,
    pub bytes: u64,
    staging: PathBuf,
}
impl Drop for PreparedTheme {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.staging); }
}

fn format_for(path: &Path) -> Result<Format, String> {
    let name = path.file_name().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
    if name.ends_with(".zip") { Ok(Format::Zip) }
    else if name.ends_with(".tar.gz") { Ok(Format::TarGz) }
    else if name.ends_with(".tar.xz") { Ok(Format::TarXz) }
    else { Err("Supported archives: ZIP, TAR.GZ and TAR.XZ".into()) }
}

fn checked_path(raw: &Path) -> Result<PathBuf, String> {
    // Explicitly reject Windows-style separators even on Unix: the archive
    // must not gain a different interpretation on another system.
    let text = raw.to_str().ok_or("Archive contains a non-UTF-8 path")?;
    if text.is_empty() || text.contains('\\') || text.contains('\0') || text.contains(':') {
        return Err("Archive contains an unsafe filename".into());
    }
    let mut result = PathBuf::new();
    for (index, component) in raw.components().enumerate() {
        match component {
            Component::Normal(name) => result.push(name),
            Component::CurDir if index == 0 => {},
            _ => return Err(format!("Archive contains an unsafe path: {text}")),
        }
    }
    if result.as_os_str().is_empty() && text != "." && text != "./" {
        return Err("Archive contains an empty path".into());
    }
    Ok(result)
}

fn checked_zip_type(name: &str, mode: Option<u32>, directory: bool) -> Result<(), String> {
    if let Some(mode) = mode {
        let kind = mode & 0o170000;
        if kind != 0 && kind != if directory { 0o040000 } else { 0o100000 } {
            return Err(format!("Archive contains a symlink or special entry: {name}"));
        }
    }
    Ok(())
}

fn reader_for(format: Format, source: &Path) -> Result<Box<dyn Read>, String> {
    let file = File::open(source).map_err(|e| format!("Cannot open theme archive: {e}"))?;
    Ok(match format {
        Format::TarGz => Box::new(GzDecoder::new(file)),
        Format::TarXz => Box::new(XzDecoder::new(file)),
        Format::Zip => Box::new(file),
    })
}

// Every entry is inspected before any file is installed. The callback runs
// twice: first to locate theme.txt, then to extract only that theme subtree.
fn visit_archive(
    source: &Path, format: Format,
    mut visit: impl FnMut(&Path, bool, u64, &mut dyn Read) -> Result<(), String>,
) -> Result<(), String> {
    let mut seen = 0_usize;
    match format {
        Format::Zip => {
            let file = File::open(source).map_err(|e| format!("Cannot open ZIP: {e}"))?;
            let mut zip = ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {e}"))?;
            if zip.len() > MAX_ITEMS { return Err("Archive contains too many entries".into()); }
            for index in 0..zip.len() {
                let mut item = zip.by_index(index).map_err(|e| format!("Cannot read ZIP entry: {e}"))?;
                let path = checked_path(Path::new(item.name()))?;
                let directory = item.is_dir();
                checked_zip_type(item.name(), item.unix_mode(), directory)?;
                if path.as_os_str().is_empty() && directory { continue; }
                visit(&path, directory, item.size(), &mut item)?;
            }
        }
        Format::TarGz | Format::TarXz => {
            let mut archive = Archive::new(reader_for(format, source)?);
            let items = archive.entries().map_err(|e| format!("Invalid TAR archive: {e}"))?;
            for entry in items {
                seen += 1;
                if seen > MAX_ITEMS { return Err("Archive contains too many entries".into()); }
                let mut entry = entry.map_err(|e| format!("Cannot read TAR entry: {e}"))?;
                let ty = entry.header().entry_type();
                if !ty.is_dir() && !ty.is_file() {
                    return Err("Archive contains a symlink, hard link or special entry".into());
                }
                let path = checked_path(&entry.path().map_err(|e| format!("Invalid TAR path: {e}"))?)?;
                let size = entry.size();
                if path.as_os_str().is_empty() && ty.is_dir() { continue; }
                visit(&path, ty.is_dir(), size, &mut entry)?;
            }
        }
    }
    Ok(())
}

fn discover_theme(source: &Path, format: Format) -> Result<PathBuf, String> {
    let mut locations = BTreeSet::new();
    let mut total_bytes = 0_u64;
    visit_archive(source, format, |path, directory, size, _| {
        if !directory {
            total_bytes = total_bytes.checked_add(size).ok_or("Archive size overflow")?;
            if total_bytes > MAX_ARCHIVE_BYTES * 2 {
                return Err("Archive declares too much uncompressed data".into());
            }
            if path.file_name().and_then(|n| n.to_str()) == Some("theme.txt") {
                if size == 0 || size > MAX_THEME_TEXT { return Err("Invalid theme.txt size".into()); }
                locations.insert(path.parent().unwrap_or(Path::new("")).to_path_buf());
            }
        }
        Ok(())
    })?;
    match locations.len() {
        0 => Err("No theme.txt found inside the archive".into()),
        1 => Ok(locations.into_iter().next().unwrap()),
        _ => Err("Archive contains multiple themes (theme.txt files). Extract it manually and choose the intended folder.".into()),
    }
}

fn theme_name(source: &Path, prefix: &Path) -> String {
    let raw = if prefix.as_os_str().is_empty() {
        let basename = source.file_name().and_then(|n| n.to_str()).unwrap_or("theme");
        basename.split('.').next().unwrap_or("theme")
    } else { prefix.file_name().and_then(|n| n.to_str()).unwrap_or("theme") };
    let normalized = raw.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') { c } else { '-' })
        .collect::<String>();
    let normalized = normalized.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if core::valid_simple_name(normalized) { normalized.to_string() } else { "imported-theme".into() }
}

fn create_staging() -> Result<PathBuf, String> {
    let mut entropy = [0_u8; 16];
    File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut entropy))
        .map_err(|e| format!("Cannot create secure import staging: {e}"))?;
    let hex = entropy.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let root = std::env::temp_dir().join(format!("grub-studio-theme-{}-{hex}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&root)
        .map_err(|e| format!("Cannot create private theme import directory: {e}"))?;
    Ok(root)
}

fn allowed_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| ALLOWED_SUFFIXES.contains(&s.to_ascii_lowercase().as_str()))
}

fn verify_theme_txt(folder: &Path) -> Result<(), String> {
    let path = folder.join("theme.txt");
    let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read extracted theme.txt: {e}"))?;
    if !content.lines().any(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with('#') && line.contains(':')
    }) {
        return Err("theme.txt does not contain any GRUB theme settings".into());
    }
    Ok(())
}

pub fn prepare(source: &Path) -> Result<PreparedTheme, String> {
    let format = format_for(source)?;
    let info = fs::symlink_metadata(source).map_err(|e| format!("Cannot inspect archive: {e}"))?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > MAX_ARCHIVE_BYTES {
        return Err("Archive is not a regular file or exceeds 80 MB".into());
    }
    let prefix = discover_theme(source, format)?;
    let name = theme_name(source, &prefix);
    let staging = create_staging()?;
    let folder = staging.join(&name);
    let outcome = (|| {
        fs::create_dir(&folder).map_err(|e| format!("Cannot create theme folder: {e}"))?;
        let mut files = 0_usize;
        let mut bytes = 0_u64;
        let mut seen = BTreeSet::new();
        visit_archive(source, format, |path, directory, size, input| {
            let relative = if prefix.as_os_str().is_empty() { path } else {
                match path.strip_prefix(&prefix) { Ok(p) => p, Err(_) => return Ok(()) }
            };
            if relative.as_os_str().is_empty() { return Ok(()); }
            // Themes sometimes ship README files or installer scripts: do not
            // execute or stage these. Only supported GRUB resources are copied.
            if !directory && !allowed_file(relative) { return Ok(()); }
            let components = relative.components().collect::<Vec<_>>();
            if components.len() > 9 { return Err("Theme directory nesting exceeds 8 levels".into()); }
            for component in &components {
                if let Component::Normal(part) = component {
                    if !core::valid_simple_name(&part.to_string_lossy()) {
                        return Err(format!("Theme contains unsupported filename: {}", relative.display()));
                    }
                }
            }
            if !seen.insert(relative.to_path_buf()) {
                return Err(format!("Archive contains duplicate path: {}", relative.display()));
            }
            let target = folder.join(relative);
            if directory {
                fs::create_dir_all(&target).map_err(|e| format!("Cannot create theme subdirectory: {e}"))?;
                return Ok(());
            }
            files += 1;
            if files > MAX_FILES { return Err("Theme contains more than 500 files".into()); }
            bytes = bytes.checked_add(size).ok_or("Theme size overflow")?;
            if bytes > MAX_EXTRACTED_BYTES { return Err("Theme exceeds 40 MB uncompressed".into()); }
            if relative == Path::new("theme.txt") && size > MAX_THEME_TEXT {
                return Err("theme.txt is too large".into());
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Cannot prepare theme subdirectory: {e}"))?;
            }
            let mut output = OpenOptions::new().create_new(true).write(true).mode(0o600)
                .open(&target).map_err(|e| format!("Cannot extract theme asset: {e}"))?;
            let mut limited = input.take(size.saturating_add(1));
            let written = std::io::copy(&mut limited, &mut output)
                .map_err(|e| format!("Cannot decompress theme asset: {e}"))?;
            if written != size { return Err("Archive entry size does not match the extracted data".into()); }
            output.set_permissions(fs::Permissions::from_mode(0o644))
                .map_err(|e| format!("Cannot set extracted theme permissions: {e}"))?;
            Ok(())
        })?;
        verify_theme_txt(&folder)?;
        Ok::<_, String>((files, bytes))
    })();
    match outcome {
        Ok((files, bytes)) => Ok(PreparedTheme { folder, name, files, bytes, staging }),
        Err(err) => { let _ = fs::remove_dir_all(&staging); Err(err) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::SimpleFileOptions;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("grub-studio-archive-test-{}-{nonce}", std::process::id()));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn path(&self, name: &str) -> PathBuf { self.0.join(name) }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, content) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }
    fn write_tar(path: &Path, xz: bool) {
        let mut tar = tar::Builder::new(Vec::new());
        for (name, content) in [("project/themes/legion/theme.txt", b"desktop-image: wall.png\n".as_slice()),
                                ("project/themes/legion/wall.png", b"PNG".as_slice()),
                                ("project/install.sh", b"unsafe".as_slice())] {
            let mut hdr = tar::Header::new_gnu();
            hdr.set_size(content.len() as u64);
            hdr.set_mode(0o644);
            hdr.set_cksum();
            tar.append_data(&mut hdr, name, Cursor::new(content)).unwrap();
        }
        tar.finish().unwrap();
        let data = tar.into_inner().unwrap();
        let file = File::create(path).unwrap();
        if xz {
            let mut encoder = xz2::write::XzEncoder::new(file, 1);
            encoder.write_all(&data).unwrap();
            encoder.finish().unwrap();
        } else {
            let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
            encoder.write_all(&data).unwrap();
            encoder.finish().unwrap();
        }
    }
    #[test]
    fn imports_zip_from_nested_directory_and_cleans_temp() {
        let fixture = Fixture::new();
        let archive = fixture.path("fancy.zip");
        write_zip(&archive, &[("wrapper/grub/legion/theme.txt", b"desktop-image: wall.png\n"),
            ("wrapper/grub/legion/background.png", b"PNG"),
            ("wrapper/grub/legion/install.sh", b"#!/bin/sh\n"),
            ("wrapper/README.md", b"readme")]);
        let theme = prepare(&archive).unwrap();
        assert_eq!(theme.name, "legion");
        assert_eq!(theme.files, 2);
        assert!(theme.folder.join("theme.txt").exists());
        assert!(!theme.folder.join("install.sh").exists());
        let staged = theme.folder.clone();
        drop(theme);
        assert!(!staged.exists());
    }
    #[test]
    fn accepts_gzip_and_xz_tar_themes() {
        let fixture = Fixture::new();
        for (name, xz) in [("theme.tar.gz", false), ("theme.tar.xz", true)] {
            let archive = fixture.path(name);
            write_tar(&archive, xz);
            let theme = prepare(&archive).unwrap();
            assert_eq!(theme.name, "legion");
            assert_eq!(theme.files, 2);
            assert!(theme.folder.join("wall.png").is_file());
        }
    }
    #[test]
    fn blocks_path_traversal_and_multiple_themes() {
        let fixture = Fixture::new();
        let traversal = fixture.path("traversal.zip");
        write_zip(&traversal, &[("../outside/theme.txt", b"desktop-image: wall.png\n")]);
        assert!(prepare(&traversal).unwrap_err().contains("unsafe path"));
        assert!(!fixture.0.join("outside").exists());
        let ambiguous = fixture.path("multiple.zip");
        write_zip(&ambiguous, &[("red/theme.txt", b"title-text: Red\n"),
            ("blue/theme.txt", b"title-text: Blue\n")]);
        assert!(prepare(&ambiguous).unwrap_err().contains("multiple themes"));
    }
    #[test]
    fn blocks_oversized_and_invalid_theme_data() {
        let fixture = Fixture::new();
        let empty = fixture.path("empty.zip");
        write_zip(&empty, &[("theme.txt", b"# comment only\n")]);
        assert!(prepare(&empty).unwrap_err().contains("settings"));
        let oversized = fixture.path("oversized.zip");
        write_zip(&oversized, &[("theme.txt", b"desktop-image: big.png\n"),
            ("big.png", &vec![b'X'; (MAX_EXTRACTED_BYTES + 1) as usize])]);
        assert!(prepare(&oversized).unwrap_err().contains("40 MB"));
    }
}
