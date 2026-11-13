// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hand-written `serde` impls for the plain data types, behind the `serde` feature.
//!
//! The derive macro is not used: `serde_derive` pulls in `unicode-ident`, which is licensed
//! under `Unicode-3.0`, a licence the dependency policy denies. `serde` itself has no
//! proc-macro dependencies.
//!
//! Newtypes serialise as newtype structs (JSON: the bare number). `AccountKind` serialises as a
//! unit variant. `Error` is not implemented: it is a diagnostic, and its variants hold
//! `&'static str` names that cannot be deserialised.

use crate::ledger::AccountKind;
use crate::units::{Joules, Kelvin, Watts};
use core::fmt;
use serde::de::{self, EnumAccess, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! newtype_serde {
    ($ty:ident) => {
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_newtype_struct(stringify!($ty), &self.0)
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct NewtypeVisitor;

                impl<'de> Visitor<'de> for NewtypeVisitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("an f64 for ", stringify!($ty)))
                    }

                    fn visit_newtype_struct<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<$ty, D::Error> {
                        f64::deserialize(deserializer).map($ty)
                    }

                    fn visit_f64<E: de::Error>(self, v: f64) -> Result<$ty, E> {
                        Ok($ty(v))
                    }
                }

                deserializer.deserialize_newtype_struct(stringify!($ty), NewtypeVisitor)
            }
        }
    };
}

newtype_serde!(Joules);
newtype_serde!(Watts);
newtype_serde!(Kelvin);

/// Variant names, in declaration order.
const ACCOUNT_KIND_NAMES: &[&str] = &["Source", "Store", "Sink", "WasteHeat"];

impl Serialize for AccountKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Exhaustive within this crate: a new variant must be added here and to the names.
        let (index, name) = match self {
            AccountKind::Source => (0, "Source"),
            AccountKind::Store => (1, "Store"),
            AccountKind::Sink => (2, "Sink"),
            AccountKind::WasteHeat => (3, "WasteHeat"),
        };
        serializer.serialize_unit_variant("AccountKind", index, name)
    }
}

/// Variant tag, read from the input as a name or an index.
enum Tag {
    Source,
    Store,
    Sink,
    WasteHeat,
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TagVisitor;

        impl<'de> Visitor<'de> for TagVisitor {
            type Value = Tag;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an AccountKind variant name or index")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Tag, E> {
                match v {
                    0 => Ok(Tag::Source),
                    1 => Ok(Tag::Store),
                    2 => Ok(Tag::Sink),
                    3 => Ok(Tag::WasteHeat),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(v), &self)),
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Tag, E> {
                match v {
                    "Source" => Ok(Tag::Source),
                    "Store" => Ok(Tag::Store),
                    "Sink" => Ok(Tag::Sink),
                    "WasteHeat" => Ok(Tag::WasteHeat),
                    _ => Err(E::unknown_variant(v, ACCOUNT_KIND_NAMES)),
                }
            }
        }

        deserializer.deserialize_identifier(TagVisitor)
    }
}

impl<'de> Deserialize<'de> for AccountKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KindVisitor;

        impl<'de> Visitor<'de> for KindVisitor {
            type Value = AccountKind;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an AccountKind")
            }

            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<AccountKind, A::Error> {
                let (tag, variant): (Tag, _) = data.variant()?;
                variant.unit_variant()?;
                Ok(match tag {
                    Tag::Source => AccountKind::Source,
                    Tag::Store => AccountKind::Store,
                    Tag::Sink => AccountKind::Sink,
                    Tag::WasteHeat => AccountKind::WasteHeat,
                })
            }
        }

        deserializer.deserialize_enum("AccountKind", ACCOUNT_KIND_NAMES, KindVisitor)
    }
}
