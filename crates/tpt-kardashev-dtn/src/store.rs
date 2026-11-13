// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle storage (RFC 9171 §5 describes the requirement; the interface here is this crate's).
//!
//! A store holds bundles keyed by identity ([`BundleId`]). Two bundles with the same identity are
//! the same bundle, so a store rejects the second insertion. Iteration order is the identities'
//! sorted order, which keeps results deterministic.
//!
//! Expiry uses DTN time, supplied by the caller. Nothing reads a wall clock.

use crate::bundle::{Bundle, BundleId};
use crate::error::Error;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Storage for bundles, keyed by identity.
pub trait BundleStore {
    /// Stores `bundle` and returns its identity. Fails with [`Error::DuplicateBundle`] if a bundle
    /// with the same identity is already stored, and with [`Error::StoreFull`] at capacity.
    fn insert(&mut self, bundle: &Bundle) -> Result<BundleId, Error>;

    /// The stored bundle with identity `id`, if any.
    fn get(&self, id: &BundleId) -> Result<Option<Bundle>, Error>;

    /// Removes the bundle with identity `id`. Returns whether one was removed.
    fn remove(&mut self, id: &BundleId) -> Result<bool, Error>;

    /// Identities of all stored bundles, in ascending order.
    fn ids(&self) -> Result<Vec<BundleId>, Error>;

    /// Number of stored bundles.
    fn len(&self) -> Result<usize, Error>;

    /// True if no bundle is stored.
    fn is_empty(&self) -> Result<bool, Error> {
        Ok(self.len()? == 0)
    }

    /// Removes every bundle whose lifetime has ended at DTN time `now`, that is, whose
    /// `expires_at()` is before `now`. Returns the identities removed, in ascending order.
    fn expire(&mut self, now: u64) -> Result<Vec<BundleId>, Error> {
        let mut removed = Vec::new();
        for id in self.ids()? {
            let expired = match self.get(&id)? {
                Some(b) => now > b.expires_at(),
                None => false,
            };
            if expired && self.remove(&id)? {
                removed.push(id);
            }
        }
        Ok(removed)
    }
}

/// A bundle store in memory, optionally bounded.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    bundles: BTreeMap<BundleId, Bundle>,
    capacity: Option<usize>,
}

impl MemoryStore {
    /// An unbounded store.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that holds at most `capacity` bundles.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bundles: BTreeMap::new(),
            capacity: Some(capacity),
        }
    }
}

impl BundleStore for MemoryStore {
    fn insert(&mut self, bundle: &Bundle) -> Result<BundleId, Error> {
        bundle.validate()?;
        let id = bundle.id();
        if self.bundles.contains_key(&id) {
            return Err(Error::DuplicateBundle);
        }
        if self.capacity.is_some_and(|c| self.bundles.len() >= c) {
            return Err(Error::StoreFull);
        }
        self.bundles.insert(id.clone(), bundle.clone());
        Ok(id)
    }

    fn get(&self, id: &BundleId) -> Result<Option<Bundle>, Error> {
        Ok(self.bundles.get(id).cloned())
    }

    fn remove(&mut self, id: &BundleId) -> Result<bool, Error> {
        Ok(self.bundles.remove(id).is_some())
    }

    fn ids(&self) -> Result<Vec<BundleId>, Error> {
        Ok(self.bundles.keys().cloned().collect())
    }

    fn len(&self) -> Result<usize, Error> {
        Ok(self.bundles.len())
    }
}

#[cfg(feature = "std")]
pub use file::FileStore;

#[cfg(feature = "std")]
mod file {
    use super::BundleStore;
    use crate::bundle::{Bundle, BundleId};
    use crate::error::Error;
    use alloc::collections::BTreeMap;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// File extension for stored bundles.
    const EXTENSION: &str = "bpv7";

    /// A bundle store that keeps each bundle as one file in a directory.
    ///
    /// The file name encodes the identity, so the directory can be reopened. Each file holds the
    /// bundle's wire encoding; reads decode it again, so a corrupted file is an error, not a
    /// silent value.
    #[derive(Debug)]
    pub struct FileStore {
        dir: PathBuf,
        files: BTreeMap<BundleId, PathBuf>,
    }

    impl FileStore {
        /// Opens (creating if needed) the store in `dir`, and loads the identities of the bundles
        /// already in it.
        pub fn open(dir: impl AsRef<Path>) -> Result<Self, Error> {
            let dir = dir.as_ref().to_path_buf();
            fs::create_dir_all(&dir).map_err(|_| Error::Io)?;
            let mut files = BTreeMap::new();
            for entry in fs::read_dir(&dir).map_err(|_| Error::Io)? {
                let path = entry.map_err(|_| Error::Io)?.path();
                if path.extension().and_then(|e| e.to_str()) != Some(EXTENSION) {
                    continue;
                }
                let bytes = fs::read(&path).map_err(|_| Error::Io)?;
                let bundle = Bundle::decode(&bytes)?;
                files.insert(bundle.id(), path);
            }
            Ok(Self { dir, files })
        }

        fn path_for(&self, id: &BundleId) -> PathBuf {
            let source_hex: String = id
                .source
                .to_string()
                .bytes()
                .map(|b| alloc::format!("{b:02x}"))
                .collect();
            self.dir.join(alloc::format!(
                "{:016x}-{:016x}-{source_hex}.{EXTENSION}",
                id.created.time,
                id.created.sequence
            ))
        }
    }

    impl BundleStore for FileStore {
        fn insert(&mut self, bundle: &Bundle) -> Result<BundleId, Error> {
            let bytes = bundle.encode()?;
            let id = bundle.id();
            if self.files.contains_key(&id) {
                return Err(Error::DuplicateBundle);
            }
            let path = self.path_for(&id);
            fs::write(&path, &bytes).map_err(|_| Error::Io)?;
            self.files.insert(id.clone(), path);
            Ok(id)
        }

        fn get(&self, id: &BundleId) -> Result<Option<Bundle>, Error> {
            match self.files.get(id) {
                None => Ok(None),
                Some(path) => {
                    let bytes = fs::read(path).map_err(|_| Error::Io)?;
                    Ok(Some(Bundle::decode(&bytes)?))
                }
            }
        }

        fn remove(&mut self, id: &BundleId) -> Result<bool, Error> {
            match self.files.remove(id) {
                None => Ok(false),
                Some(path) => {
                    fs::remove_file(path).map_err(|_| Error::Io)?;
                    Ok(true)
                }
            }
        }

        fn ids(&self) -> Result<Vec<BundleId>, Error> {
            Ok(self.files.keys().cloned().collect())
        }

        fn len(&self) -> Result<usize, Error> {
            Ok(self.files.len())
        }
    }
}
