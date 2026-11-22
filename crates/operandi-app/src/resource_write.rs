//! App-resource bytes only; callers retain encoding, validation and path policy.
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub(super) fn write_bytes(path: &Path, temp: &Path, bytes: &[u8]) -> io::Result<()> {
    // Exclusive creation establishes cleanup ownership. An occupied temp is untouched.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_failures_cleanup_and_disjoint_roots() {
        let root = std::env::temp_dir().join(format!(
            "operandi-resource-write-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        for name in ["a", "b"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let path = root.join("a/resource.json");
        let temp = root.join("a/resource.tmp");
        fs::write(&path, b"old").unwrap();
        write_bytes(&path, &temp, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!temp.exists());
        fs::write(&temp, b"occupied").unwrap();
        let error = write_bytes(&path, &temp, b"refused").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&temp).unwrap(), b"occupied");
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let other = root.join("b/resource.json");
        let other_temp = root.join("b/resource.tmp");
        write_bytes(&other, &other_temp, b"other root").unwrap();
        assert_eq!(fs::read(&other).unwrap(), b"other root");
        assert!(!other_temp.exists());
        assert_eq!(fs::read(&temp).unwrap(), b"occupied");
        let missing = root.join("missing/resource.json");
        let missing_temp = root.join("missing/resource.tmp");
        assert_eq!(
            write_bytes(&missing, &missing_temp, b"no parent")
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.join("missing").exists());
        let directory = root.join("destination");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("sentinel"), b"protected").unwrap();
        let owned_temp = root.join("owned.tmp");
        let error = write_bytes(&directory, &owned_temp, b"cannot replace directory").unwrap_err();
        assert!(!owned_temp.exists());
        assert_eq!(fs::read(directory.join("sentinel")).unwrap(), b"protected");
        // The helper returns the rename error unchanged.
        fs::write(&owned_temp, b"direct comparison").unwrap();
        let direct = fs::rename(&owned_temp, &directory).unwrap_err();
        assert_eq!(error.kind(), direct.kind());
        assert_eq!(error.to_string(), direct.to_string());
        fs::remove_dir_all(root).unwrap();
    }
}
