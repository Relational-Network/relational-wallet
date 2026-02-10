// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Encrypted filesystem operations using Gramine's transparent encryption.
//!
//! ## Security Note
//!
//! This module uses **standard filesystem I/O**. Gramine handles encryption
//! transparently for all files under `/data` (mounted as `type = "encrypted"`).
//!
//! **DO NOT**:
//! - Implement any crypto operations in this module
//! - Access SGX key devices (`/dev/attestation/keys/*`)
//! - Use `fs.insecure__keys.*` manifest options
//!
//! The Rust application treats `/data` as a normal filesystem; Gramine
//! ensures confidentiality, integrity, and tamper resistance.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};

use super::StoragePaths;

/// Error type for encrypted storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// I/O error during file operations
    Io(io::Error),
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
    /// Entity not found
    NotFound(String),
    /// Entity already exists
    AlreadyExists(String),
    /// Storage not initialized
    NotInitialized,
    /// Integrity violation (file tampered or corrupted)
    /// Note: This is detected by Gramine, not by Rust code
    IntegrityViolation(String),
    /// Permission denied (ownership check failed)
    PermissionDenied { user_id: String, resource: String },
    /// Generic serialization error
    SerializationError(String),
    /// Not found with structured info
    /// TODO: Use when implementing structured error responses
    #[allow(dead_code)]
    NotFoundResource { resource: String, id: String },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "I/O error: {e}"),
            StorageError::Json(e) => write!(f, "JSON error: {e}"),
            StorageError::NotFound(entity) => write!(f, "Not found: {entity}"),
            StorageError::AlreadyExists(entity) => write!(f, "Already exists: {entity}"),
            StorageError::NotInitialized => write!(f, "Storage not initialized"),
            StorageError::IntegrityViolation(msg) => write!(f, "Integrity violation: {msg}"),
            StorageError::PermissionDenied { user_id, resource } => {
                write!(f, "Permission denied: user {user_id} cannot access {resource}")
            }
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            StorageError::NotFoundResource { resource, id } => {
                write!(f, "{resource} not found: {id}")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Io(e) => Some(e),
            StorageError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self {
        // Gramine signals integrity failures as I/O errors
        // Common patterns: "Authentication tag mismatch" or similar
        let msg = e.to_string();
        if msg.contains("Authentication") || msg.contains("integrity") || msg.contains("tamper") {
            StorageError::IntegrityViolation(msg)
        } else if e.kind() == io::ErrorKind::NotFound {
            StorageError::NotFound(msg)
        } else {
            StorageError::Io(e)
        }
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Json(e)
    }
}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Encrypted storage manager using Gramine's transparent encryption.
///
/// All operations use standard filesystem I/O. Gramine handles encryption
/// for files under the `/data` mount point.
#[derive(Debug, Clone)]
pub struct EncryptedStorage {
    paths: StoragePaths,
    initialized: bool,
}

impl EncryptedStorage {
    /// Create a new EncryptedStorage instance.
    ///
    /// Does NOT initialize the directory structure. Call `initialize()` first.
    pub fn new(paths: StoragePaths) -> Self {
        Self {
            paths,
            initialized: false,
        }
    }

    /// Create with default paths (/data).
    #[allow(dead_code)]
    pub fn with_default_paths() -> Self {
        Self::new(StoragePaths::default())
    }

    /// Get the storage paths.
    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    /// Check if storage is initialized.
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Initialize the encrypted storage directory structure.
    ///
    /// Creates all required directories under `/data`.
    /// Safe to call multiple times (idempotent).
    pub fn initialize(&mut self) -> StorageResult<()> {
        let dirs = [
            self.paths.wallets_dir(),
            self.paths.bookmarks_dir(),
            self.paths.invites_dir(),
            self.paths.recurring_dir(),
            self.paths.audit_dir(),
        ];

        for dir in dirs {
            fs::create_dir_all(&dir)?;
        }

        self.initialized = true;
        Ok(())
    }

    /// Check if the encrypted filesystem is available and working.
    ///
    /// This performs a write-read-delete test to verify the filesystem
    /// is properly mounted and encryption is working.
    pub fn health_check(&self) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let test_file = self.paths.root().join(".health_check");
        let test_data = b"health_check_data";

        // Write test data
        fs::write(&test_file, test_data)?;

        // Read it back
        let read_data = fs::read(&test_file)?;

        // Clean up
        fs::remove_file(&test_file)?;

        // Verify data integrity
        if read_data != test_data {
            return Err(StorageError::IntegrityViolation(
                "Health check data mismatch".to_string(),
            ));
        }

        Ok(())
    }

    // ========== Generic JSON Operations ==========

    /// Read a JSON file and deserialize it.
    pub fn read_json<T: DeserializeOwned>(&self, path: impl AsRef<Path>) -> StorageResult<T> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        let value = serde_json::from_reader(reader)?;
        Ok(value)
    }

    /// Write a JSON file (atomic write via rename).
    pub fn write_json<T: Serialize>(&self, path: impl AsRef<Path>, value: &T) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first, then rename for atomicity
        let temp_path = path.with_extension("tmp");
        {
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, value)?;
            writer.flush()?;
        }

        // Atomic rename
        fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Check if a file exists.
    ///
    /// Uses `File::open()` instead of `Path::exists()` because Gramine's
    /// encrypted filesystem can fail `stat()` calls on encrypted files
    /// while `open()` + `read()` works correctly.
    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        File::open(path.as_ref()).is_ok()
    }

    /// Delete a file.
    pub fn delete(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }
        fs::remove_file(path.as_ref())?;
        Ok(())
    }

    /// Delete a directory and all its contents.
    /// TODO: Use for wallet hard delete (after retention period)
    #[allow(dead_code)]
    pub fn delete_dir(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }
        fs::remove_dir_all(path.as_ref())?;
        Ok(())
    }

    /// List all files in a directory matching a pattern.
    pub fn list_files(&self, dir: impl AsRef<Path>, extension: &str) -> StorageResult<Vec<String>> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
                        if let Some(stem) = path.file_stem() {
                            if let Some(id) = stem.to_str() {
                                ids.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }
        Ok(ids)
    }

    /// List all subdirectories in a directory.
    pub fn list_dirs(&self, dir: impl AsRef<Path>) -> StorageResult<Vec<String>> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        Ok(names)
    }

    // ========== Raw File Operations (for PEM keys, etc.) ==========

    /// Write raw bytes to a file (for private keys, etc.).
    pub fn write_raw(&self, path: impl AsRef<Path>, data: &[u8]) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }

    /// Read raw bytes from a file.
    pub fn read_raw(&self, path: impl AsRef<Path>) -> StorageResult<Vec<u8>> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }

        let mut file = File::open(path.as_ref())?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(data)
    }

    /// Create a directory (including parents).
    pub fn create_dir(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        if !self.initialized {
            return Err(StorageError::NotInitialized);
        }
        fs::create_dir_all(path.as_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::env;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-storage-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize test storage");
        storage
    }

    fn cleanup_storage(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: String,
        value: i32,
    }

    #[test]
    fn initialize_creates_directories() {
        let storage = test_storage();

        assert!(storage.paths().wallets_dir().exists());
        assert!(storage.paths().bookmarks_dir().exists());
        assert!(storage.paths().invites_dir().exists());
        assert!(storage.paths().recurring_dir().exists());
        assert!(storage.paths().audit_dir().exists());

        cleanup_storage(&storage);
    }

    #[test]
    fn write_and_read_json() {
        let storage = test_storage();
        let data = TestData {
            id: "test-1".to_string(),
            value: 42,
        };

        let path = storage.paths().bookmarks_dir().join("test.json");
        storage.write_json(&path, &data).unwrap();

        let read: TestData = storage.read_json(&path).unwrap();
        assert_eq!(read, data);

        cleanup_storage(&storage);
    }

    #[test]
    fn write_and_read_raw() {
        let storage = test_storage();
        let data = b"raw test data with\nnewlines\nand bytes: \x00\x01\x02";

        let path = storage.paths().wallets_dir().join("test-wallet").join("key.pem");
        storage.write_raw(&path, data).unwrap();

        let read = storage.read_raw(&path).unwrap();
        assert_eq!(read, data);

        cleanup_storage(&storage);
    }

    #[test]
    fn health_check_works() {
        let storage = test_storage();
        storage.health_check().expect("Health check should pass");
        cleanup_storage(&storage);
    }

    #[test]
    fn list_files_returns_ids() {
        let storage = test_storage();

        // Create some test files
        for i in 1..=3 {
            let path = storage.paths().bookmarks_dir().join(format!("bm-{i}.json"));
            storage
                .write_json(&path, &TestData {
                    id: format!("bm-{i}"),
                    value: i,
                })
                .unwrap();
        }

        let ids = storage.list_files(storage.paths().bookmarks_dir(), "json").unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"bm-1".to_string()));
        assert!(ids.contains(&"bm-2".to_string()));
        assert!(ids.contains(&"bm-3".to_string()));

        cleanup_storage(&storage);
    }

    #[test]
    fn list_dirs_returns_names() {
        let storage = test_storage();

        // Create some test directories
        for i in 1..=3 {
            storage
                .create_dir(storage.paths().wallets_dir().join(format!("wallet-{i}")))
                .unwrap();
        }

        let names = storage.list_dirs(storage.paths().wallets_dir()).unwrap();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"wallet-1".to_string()));
        assert!(names.contains(&"wallet-2".to_string()));
        assert!(names.contains(&"wallet-3".to_string()));

        cleanup_storage(&storage);
    }

    #[test]
    fn delete_file_removes_it() {
        let storage = test_storage();

        let path = storage.paths().bookmarks_dir().join("to-delete.json");
        storage
            .write_json(&path, &TestData {
                id: "del".to_string(),
                value: 0,
            })
            .unwrap();

        assert!(storage.exists(&path));
        storage.delete(&path).unwrap();
        assert!(!storage.exists(&path));

        cleanup_storage(&storage);
    }

    #[test]
    fn delete_dir_removes_recursively() {
        let storage = test_storage();

        let wallet_dir = storage.paths().wallet_dir("to-delete");
        storage.create_dir(&wallet_dir).unwrap();
        storage
            .write_json(storage.paths().wallet_meta("to-delete"), &TestData {
                id: "w".to_string(),
                value: 1,
            })
            .unwrap();

        assert!(wallet_dir.exists());
        storage.delete_dir(&wallet_dir).unwrap();
        assert!(!wallet_dir.exists());

        cleanup_storage(&storage);
    }

    #[test]
    fn uninitialized_storage_returns_error() {
        let paths = StoragePaths::new("/tmp/never-init");
        let storage = EncryptedStorage::new(paths);

        let result = storage.read_json::<TestData>("/tmp/any.json");
        assert!(matches!(result, Err(StorageError::NotInitialized)));
    }
}
