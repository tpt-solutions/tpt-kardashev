// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hand-written `serde` impls for the dtn data types, behind the `serde` feature.
//!
//! The derive macro is not used: `serde_derive` pulls in `unicode-ident`, which is licensed under
//! `Unicode-3.0`, a licence the dependency policy denies.
//!
//! Which way each type goes follows from its invariants:
//! - [`Eid`] round-trips as its URI text, and parsing it re-checks the syntax.
//! - [`CrcType`] round-trips as its code (0, 1 or 2).
//! - Plain records of integers (`CreationTimestamp`, `Fragment`, `Claim`, `Checkpoint`) round-trip
//!   as maps of field names to values.
//! - [`Contact`] only serialises. Its constructor checks the window and the rate, and a
//!   deserialiser would skip that check. [`BundleId`], [`Hop`] and [`Route`] are derived outputs,
//!   so they only serialise too.
//! - Bundles, segments and sessions are not implemented: their decoders are the way in.

use crate::bundle::{BundleId, CrcType, CreationTimestamp, Fragment};
use crate::contact::Contact;
use crate::eid::Eid;
use crate::ltp::segment::{Checkpoint, Claim};
use crate::route::{Hop, Route};
use core::fmt;
use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Implements `Serialize` for a struct of `u64` fields, as a map keyed by field name.
macro_rules! serialize_u64_fields {
    ($ty:ident { $($field:ident),+ $(,)? }) => {
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let count = [$(stringify!($field)),+].len();
                let mut st = serializer.serialize_struct(stringify!($ty), count)?;
                $( st.serialize_field(stringify!($field), &self.$field)?; )+
                st.end()
            }
        }
    };
}

/// Implements both directions for a struct of `u64` fields. Missing fields are errors; unknown
/// fields are skipped.
macro_rules! u64_fields_serde {
    ($ty:ident { $($field:ident),+ $(,)? }) => {
        serialize_u64_fields!($ty { $($field),+ });

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct RecordVisitor;

                impl<'de> Visitor<'de> for RecordVisitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " map"))
                    }

                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<$ty, A::Error> {
                        $( let mut $field: Option<u64> = None; )+
                        while let Some(key) = map.next_key::<&'de str>()? {
                            $(
                                if key == stringify!($field) {
                                    $field = Some(map.next_value()?);
                                    continue;
                                }
                            )+
                            map.next_value::<IgnoredAny>()?;
                        }
                        Ok($ty {
                            $(
                                $field: $field.ok_or_else(|| {
                                    de::Error::missing_field(stringify!($field))
                                })?,
                            )+
                        })
                    }
                }

                deserializer.deserialize_struct(
                    stringify!($ty),
                    &[$(stringify!($field)),+],
                    RecordVisitor,
                )
            }
        }
    };
}

u64_fields_serde!(CreationTimestamp { time, sequence });
u64_fields_serde!(Fragment { offset, total_len });
u64_fields_serde!(Claim { offset, length });
u64_fields_serde!(Checkpoint {
    serial,
    report_serial
});
serialize_u64_fields!(Contact {
    from,
    to,
    start,
    end,
    rate,
    owlt
});

impl Serialize for Eid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Eid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EidVisitor;

        impl<'de> Visitor<'de> for EidVisitor {
            type Value = Eid;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an endpoint ID URI such as dtn://node/demux or ipn:1.2")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Eid, E> {
                Eid::parse(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(EidVisitor)
    }
}

impl Serialize for CrcType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.code())
    }
}

impl<'de> Deserialize<'de> for CrcType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CrcVisitor;

        impl<'de> Visitor<'de> for CrcVisitor {
            type Value = CrcType;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a CRC type code: 0, 1 or 2")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<CrcType, E> {
                match v {
                    0 => Ok(CrcType::None),
                    1 => Ok(CrcType::Crc16),
                    2 => Ok(CrcType::Crc32C),
                    other => Err(E::custom(format_args!("unknown CRC type {other}"))),
                }
            }
        }

        deserializer.deserialize_u64(CrcVisitor)
    }
}

impl Serialize for BundleId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("BundleId", 2)?;
        st.serialize_field("source", &self.source)?;
        st.serialize_field("created", &self.created)?;
        st.end()
    }
}

impl Serialize for Hop {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Hop", 3)?;
        st.serialize_field("contact", &self.contact)?;
        st.serialize_field("departure", &self.departure)?;
        st.serialize_field("arrival", &self.arrival)?;
        st.end()
    }
}

impl Serialize for Route {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Route", 2)?;
        st.serialize_field("hops", &self.hops)?;
        st.serialize_field("arrival", &self.arrival)?;
        st.end()
    }
}
