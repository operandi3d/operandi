//! Operandi's one safe file writer (Astra P9-003, P7-009; Jason
//! 2026-11-22: "a save either fully replaces the old file or leaves it
//! untouched").
//!
//! Every save of a scene, image, export or bake goes through
//! [`write_atomic`] or [`write_atomic_with`]:
//!
//! 1. The new contents go to a temporary file beside the destination, with
//!    a name no other writer uses (process id and a counter) and created
//!    with `create_new`, so an existing file, ours or anyone's, is never
//!    taken over.
//! 2. The temporary is flushed to disk (`sync_all`) before it replaces the
//!    destination, so a power cut can't leave a renamed but empty file.
//! 3. It is renamed over the destination in one step, then the folder is
//!    synced so the rename itself is on disk.
//! 4. On any error only our own temporary is removed; the destination is
//!    as it was.
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary name beside `path` that no other call or process uses. The
/// destination's own extension stays last, for tools watching the folder.
fn temporary_beside(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} names no file", path.display()))?
        .to_string_lossy();
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(".{name}.{}-{n}.operandi-tmp", std::process::id())))
}

/// Writes `data` to `path` atomically (see the crate comment).
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    write_atomic_with(path, |temp| {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(temp)
            .map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())
    })
}

/// `write_atomic` for a writer that fills the file itself, given its path
/// (an image encoder, a C library). The temporary exists, empty, when
/// `fill` runs; `fill` may replace it.
pub fn write_atomic_with(
    path: &Path,
    fill: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let temp = temporary_beside(path)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| {
            format!(
                "Cannot create a temporary file beside {}: {e}",
                path.display()
            )
        })?;
    let result = (|| {
        fill(&temp)?;
        File::open(&temp)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
        fs::rename(&temp, path).map_err(|e| format!("Cannot replace {}: {e}", path.display()))?;
        sync_folder(path);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Puts the folder's entry for `path` on disk (POSIX needs it for the
/// rename to survive a power cut; elsewhere it isn't possible or needed).
fn sync_folder(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        let dir = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Reads all of `path`, refusing a file over `limit` bytes before reading
/// it, and a file the memory can't hold with an error instead of stopping
/// the program (hidden-limits A2, B7: the old fixed 4 GiB cap refused real
/// scenes; this limit only guards against a damaged size).
pub fn read_whole(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?
        .len();
    if size > limit {
        return Err(format!(
            "{} is {}, more than the {} Operandi reads in one file",
            path.display(),
            size_text(size),
            size_text(limit)
        ));
    }
    let mut data = Vec::new();
    data.try_reserve_exact(size as usize).map_err(|_| {
        format!(
            "Reading {} needs about {} of memory, which isn't free",
            path.display(),
            size_text(size)
        )
    })?;
    // `take` keeps a file that grows while it's read within the limit.
    file.take(limit.saturating_add(1))
        .read_to_end(&mut data)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if data.len() as u64 > limit {
        return Err(format!(
            "{} grew past {} while it was read",
            path.display(),
            size_text(limit)
        ));
    }
    Ok(data)
}

/// "12.3 GB" style sizes for messages.
pub fn size_text(bytes: u64) -> String {
    const GIB: f64 = (1u64 << 30) as f64;
    const MIB: f64 = (1u64 << 20) as f64;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1} GB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MB", b / MIB)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("operandi-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".operandi-tmp"))
            .collect()
    }

    #[test]
    fn replaces_the_file_and_leaves_no_temporary() {
        let dir = folder("replace");
        let path = dir.join("a.png");
        fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(leftovers(&dir).is_empty());
    }

    #[test]
    fn a_failed_write_keeps_the_old_file_and_removes_only_its_own_temporary() {
        let dir = folder("fail");
        let path = dir.join("a.png");
        fs::write(&path, b"old").unwrap();
        // Someone else's temporary with the old fixed name stays alone.
        let theirs = dir.join(".a.png.operandi-tmp");
        fs::write(&theirs, b"theirs").unwrap();
        let e = write_atomic_with(&path, |_| Err("encoder failed".into())).unwrap_err();
        assert_eq!(e, "encoder failed");
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(fs::read(&theirs).unwrap(), b"theirs");
        assert_eq!(leftovers(&dir), vec![".a.png.operandi-tmp".to_string()]);
    }

    #[test]
    fn read_whole_checks_its_limit_at_the_edges() {
        let dir = folder("read");
        let path = dir.join("scene.opscene");
        fs::write(&path, vec![b' '; 100]).unwrap();
        assert_eq!(read_whole(&path, 101).unwrap().len(), 100);
        assert_eq!(read_whole(&path, 100).unwrap().len(), 100);
        let e = read_whole(&path, 99).unwrap_err();
        assert!(e.contains("more than the 99 bytes"), "{e}");
    }

    #[test]
    fn two_writers_never_share_a_temporary() {
        let path = Path::new("/x/a.exr");
        let a = temporary_beside(path).unwrap();
        let b = temporary_beside(path).unwrap();
        assert_ne!(a, b);
        assert!(a.to_string_lossy().ends_with(".operandi-tmp"));
    }
}
