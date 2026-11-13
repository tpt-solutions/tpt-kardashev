// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle stores: identity, duplicates, capacity, expiry, and the file-backed store.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::bundle_flags;
use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::store::FileStore;
use tpt_kardashev_dtn::{Bundle, BundleStore, Eid, Error, MemoryStore};

fn bundle(source: u64, time: u64, seq: u64, lifetime: u64) -> Bundle {
    Bundle {
        primary: PrimaryBlock {
            flags: 0,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: 9,
                service: 1,
            },
            source: Eid::Ipn {
                node: source,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time,
                sequence: seq,
            },
            lifetime,
            fragment: None,
        },
        blocks: vec![CanonicalBlock {
            block_type: 1,
            number: 1,
            flags: 0,
            crc: CrcType::Crc32C,
            data: vec![seq as u8; 4],
        }],
    }
}

#[test]
fn insert_get_remove_and_order() {
    let mut s = MemoryStore::new();
    let b1 = bundle(1, 100, 0, 1000);
    let b2 = bundle(1, 50, 0, 1000);
    let id1 = s.insert(&b1).expect("insert");
    s.insert(&b2).expect("insert");
    assert_eq!(s.len().expect("len"), 2);
    assert_eq!(s.get(&id1).expect("get"), Some(b1.clone()));
    // Identities come back in ascending order: creation time 50 sorts before 100.
    let ids = s.ids().expect("ids");
    assert_eq!(ids, vec![b2.id(), b1.id()]);
    assert!(s.remove(&id1).expect("remove"));
    assert!(!s.remove(&id1).expect("remove again"));
    assert_eq!(s.get(&id1).expect("get"), None);
}

#[test]
fn duplicate_identity_is_rejected() {
    let mut s = MemoryStore::new();
    s.insert(&bundle(1, 5, 3, 1000)).expect("insert");
    assert_eq!(
        s.insert(&bundle(1, 5, 3, 9999)),
        Err(Error::DuplicateBundle)
    );
    // The same time and sequence from another source is a different bundle.
    assert!(s.insert(&bundle(2, 5, 3, 1000)).is_ok());
}

#[test]
fn capacity_is_enforced() {
    let mut s = MemoryStore::with_capacity(2);
    s.insert(&bundle(1, 1, 0, 100)).expect("insert");
    s.insert(&bundle(1, 2, 0, 100)).expect("insert");
    assert_eq!(s.insert(&bundle(1, 3, 0, 100)), Err(Error::StoreFull));
}

#[test]
fn expiry_removes_only_bundles_whose_lifetime_has_passed() {
    // Expiry times are created + lifetime: 100 + 50 = 150, 100 + 500 = 600, 0 + 10 = 10.
    let mut s = MemoryStore::new();
    s.insert(&bundle(1, 100, 0, 50)).expect("insert");
    s.insert(&bundle(1, 100, 1, 500)).expect("insert");
    s.insert(&bundle(1, 0, 2, 10)).expect("insert");
    // At exactly 10, nothing has expired: the age must exceed the lifetime.
    assert!(s.expire(10).expect("expire").is_empty());
    // At 151, the bundles expiring at 10 and at 150 have gone; the one expiring at 600 remains.
    let removed = s.expire(151).expect("expire");
    assert_eq!(
        removed,
        vec![bundle(1, 0, 2, 10).id(), bundle(1, 100, 0, 50).id()]
    );
    assert_eq!(s.len().expect("len"), 1);
    assert!(s.get(&bundle(1, 100, 1, 500).id()).expect("get").is_some());
}

#[test]
fn invalid_bundles_are_not_stored() {
    let mut s = MemoryStore::new();
    let mut b = bundle(1, 1, 0, 100);
    // An administrative record may not request status reports (RFC 9171 §4.2.3).
    b.primary.flags |= bundle_flags::ADMIN_RECORD | bundle_flags::REPORT_DELIVERY;
    assert!(matches!(s.insert(&b), Err(Error::InvalidBundle { .. })));
    assert_eq!(s.len().expect("len"), 0);
}

#[test]
fn file_store_persists_across_reopen() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("dtn-file-store-test");
    let _ = std::fs::remove_dir_all(&dir);
    let b = bundle(4, 777, 2, 1000);
    let id;
    {
        let mut s = FileStore::open(&dir).expect("open");
        id = s.insert(&b).expect("insert");
        assert_eq!(s.insert(&b), Err(Error::DuplicateBundle));
    }
    let s = FileStore::open(&dir).expect("reopen");
    assert_eq!(s.len().expect("len"), 1);
    assert_eq!(s.get(&id).expect("get"), Some(b));
    let _ = std::fs::remove_dir_all(&dir);
}
