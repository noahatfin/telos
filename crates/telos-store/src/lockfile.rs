use crate::error::StoreError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A lock file for atomic writes to a target path.
///
/// Creates `<target>.lock`, writes data to it, then atomically renames
/// to `<target>` on commit. The lock file is removed on drop if not committed.
pub struct Lockfile {
    target: PathBuf,
    lock_path: PathBuf,
    file: Option<fs::File>,
}

impl Lockfile {
    /// Acquire a lock for the given target path.
    pub fn acquire(target: impl AsRef<Path>) -> Result<Self, StoreError> {
        let target = target.as_ref().to_path_buf();
        let lock_path = target.with_extension(
            target
                .extension()
                .map(|e| format!("{}.lock", e.to_string_lossy()))
                .unwrap_or_else(|| "lock".to_string()),
        );

        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => Ok(Self {
                target,
                lock_path,
                file: Some(file),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(StoreError::LockConflict(lock_path.display().to_string()))
            }
            Err(e) => Err(StoreError::Io(e)),
        }
    }

    /// Write data to the lock file.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), StoreError> {
        if let Some(ref mut file) = self.file {
            file.write_all(data)?;
            file.flush()?;
            Ok(())
        } else {
            Err(StoreError::LockConflict(
                "lock file already committed or dropped".into(),
            ))
        }
    }

    /// Atomically commit: rename lock file to target.
    pub fn commit(mut self) -> Result<(), StoreError> {
        // Drop the file handle first so it's flushed and closed
        self.file.take();
        fs::rename(&self.lock_path, &self.target)?;
        Ok(())
    }
}

impl Drop for Lockfile {
    fn drop(&mut self) {
        // If not committed, remove the lock file
        if self.file.is_some() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_write_commit() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");

        let mut lock = Lockfile::acquire(&target).unwrap();
        lock.write_all(b"hello").unwrap();
        lock.commit().unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
        assert!(!target.with_extension("txt.lock").exists());
    }

    #[test]
    fn lock_dropped_without_commit() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");

        {
            let mut lock = Lockfile::acquire(&target).unwrap();
            lock.write_all(b"hello").unwrap();
            // drop without commit
        }

        assert!(!target.exists());
    }

    #[test]
    fn double_lock_fails() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");

        let _lock1 = Lockfile::acquire(&target).unwrap();
        let result = Lockfile::acquire(&target);
        assert!(result.is_err());
    }
}
