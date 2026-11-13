// SPDX-License-Identifier: MIT OR Apache-2.0
//! Contacts and contact plans: scheduled, one-directional windows during which a node can send
//! to another node.
//!
//! Nodes are identified by number (the `ipn` node number). Times are DTN milliseconds, rates are
//! bytes per second, and the one-way light time is in milliseconds. Nothing here reads a clock.

use crate::error::Error;
use alloc::vec::Vec;

/// A scheduled window in which `from` can transmit to `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Contact {
    /// Transmitting node.
    pub from: u64,
    /// Receiving node.
    pub to: u64,
    /// Start of the window, DTN milliseconds.
    pub start: u64,
    /// End of the window, DTN milliseconds. Exclusive: a transmission must finish by then.
    pub end: u64,
    /// Data rate in bytes per second. Positive.
    pub rate: u64,
    /// One-way light time from `from` to `to`, in milliseconds.
    pub owlt: u64,
}

impl Contact {
    /// Builds a contact, checking that it is well formed: distinct nodes, `start < end`, and a
    /// positive rate.
    pub fn new(
        from: u64,
        to: u64,
        start: u64,
        end: u64,
        rate: u64,
        owlt: u64,
    ) -> Result<Self, Error> {
        if from == to {
            return Err(Error::InvalidContact {
                rule: "a contact joins two different nodes",
            });
        }
        if start >= end {
            return Err(Error::InvalidContact {
                rule: "start must be before end",
            });
        }
        if rate == 0 {
            return Err(Error::InvalidContact {
                rule: "rate must be positive",
            });
        }
        Ok(Self {
            from,
            to,
            start,
            end,
            rate,
            owlt,
        })
    }

    /// Milliseconds needed to transmit `size` bytes at this contact's rate, rounded up.
    pub fn transmit_ms(&self, size: u64) -> u64 {
        let ms = u128::from(size) * 1000;
        let t = ms.div_ceil(u128::from(self.rate));
        u64::try_from(t).unwrap_or(u64::MAX)
    }
}

/// A set of contacts, sorted by start time and then by the remaining fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContactPlan {
    contacts: Vec<Contact>,
}

impl ContactPlan {
    /// Builds a plan from contacts, sorting them. Every contact is already valid by construction.
    pub fn new(mut contacts: Vec<Contact>) -> Self {
        contacts.sort();
        Self { contacts }
    }

    /// The contacts, in sorted order.
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// Contacts that transmit from `node`.
    pub fn outgoing(&self, node: u64) -> impl Iterator<Item = &Contact> {
        self.contacts.iter().filter(move |c| c.from == node)
    }
}
