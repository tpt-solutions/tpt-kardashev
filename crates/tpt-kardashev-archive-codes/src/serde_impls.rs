// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hand-written `serde` impls for the data types, behind the `serde` feature.
//!
//! The derive macro is not used: `serde_derive` pulls in `unicode-ident`, which is licensed
//! under `Unicode-3.0`, a licence the dependency policy denies.
//!
//! `CodeError` is serialise-only: its variants hold `&'static str` names, which cannot be
//! deserialised. `Frame` is not implemented: it borrows its payload from the input.

use crate::error::{CodeError, DecodeReport};
use crate::frame::{CodeKind, FrameParams};
use core::fmt;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::{SerializeStruct, SerializeStructVariant};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for CodeError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Exhaustive within this crate: a new variant must be added here.
        match self {
            CodeError::InvalidParameter { name } => {
                let mut s =
                    serializer.serialize_struct_variant("CodeError", 0, "InvalidParameter", 1)?;
                s.serialize_field("name", name)?;
                s.end()
            }
            CodeError::LengthMismatch { expected, actual } => {
                let mut s =
                    serializer.serialize_struct_variant("CodeError", 1, "LengthMismatch", 2)?;
                s.serialize_field("expected", expected)?;
                s.serialize_field("actual", actual)?;
                s.end()
            }
            CodeError::InvalidSymbol => {
                serializer.serialize_unit_variant("CodeError", 2, "InvalidSymbol")
            }
            CodeError::Uncorrectable => {
                serializer.serialize_unit_variant("CodeError", 3, "Uncorrectable")
            }
        }
    }
}

impl Serialize for DecodeReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("DecodeReport", 1)?;
        s.serialize_field("corrected", &self.corrected)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for DecodeReport {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ReportVisitor;

        impl<'de> Visitor<'de> for ReportVisitor {
            type Value = DecodeReport;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a DecodeReport")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<DecodeReport, A::Error> {
                let mut corrected = None;
                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Corrected => {
                            corrected = Some(map.next_value::<usize>()?);
                        }
                        _ => {
                            map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }
                let corrected = corrected.ok_or_else(|| de::Error::missing_field("corrected"))?;
                Ok(DecodeReport { corrected })
            }
        }

        deserializer.deserialize_struct("DecodeReport", &["corrected"], ReportVisitor)
    }
}

impl Serialize for CodeKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Exhaustive within this crate: a new variant must be added here and to the names.
        let (index, name) = match self {
            CodeKind::Uncoded => (0, "Uncoded"),
            CodeKind::ReedSolomon => (1, "ReedSolomon"),
            CodeKind::ConvolutionalK7 => (2, "ConvolutionalK7"),
        };
        serializer.serialize_unit_variant("CodeKind", index, name)
    }
}

const CODE_KIND_NAMES: &[&str] = &["Uncoded", "ReedSolomon", "ConvolutionalK7"];

impl<'de> Deserialize<'de> for CodeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KindVisitor;

        impl<'de> Visitor<'de> for KindVisitor {
            type Value = CodeKind;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a CodeKind name or index")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<CodeKind, E> {
                match v {
                    0 => Ok(CodeKind::Uncoded),
                    1 => Ok(CodeKind::ReedSolomon),
                    2 => Ok(CodeKind::ConvolutionalK7),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(v), &self)),
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<CodeKind, E> {
                match v {
                    "Uncoded" => Ok(CodeKind::Uncoded),
                    "ReedSolomon" => Ok(CodeKind::ReedSolomon),
                    "ConvolutionalK7" => Ok(CodeKind::ConvolutionalK7),
                    _ => Err(E::unknown_variant(v, CODE_KIND_NAMES)),
                }
            }
        }

        deserializer.deserialize_identifier(KindVisitor)
    }
}

impl Serialize for FrameParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("FrameParams", 5)?;
        s.serialize_field("kind", &self.kind)?;
        s.serialize_field("nroots", &self.nroots)?;
        s.serialize_field("depth", &self.depth)?;
        s.serialize_field("fcr", &self.fcr)?;
        s.serialize_field("prim", &self.prim)?;
        s.end()
    }
}

const FRAME_PARAMS_FIELDS: &[&str] = &["kind", "nroots", "depth", "fcr", "prim"];

/// Struct field names of `DecodeReport` and `FrameParams`. Any other name is skipped, as serde
/// does by default.
enum Field {
    Corrected,
    Kind,
    Nroots,
    Depth,
    Fcr,
    Prim,
    Unknown,
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a field name")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                Ok(match v {
                    "corrected" => Field::Corrected,
                    "kind" => Field::Kind,
                    "nroots" => Field::Nroots,
                    "depth" => Field::Depth,
                    "fcr" => Field::Fcr,
                    "prim" => Field::Prim,
                    _ => Field::Unknown,
                })
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

impl<'de> Deserialize<'de> for FrameParams {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ParamsVisitor;

        impl<'de> Visitor<'de> for ParamsVisitor {
            type Value = FrameParams;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("FrameParams")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<FrameParams, A::Error> {
                let mut kind = None;
                let mut nroots = None;
                let mut depth = None;
                let mut fcr = None;
                let mut prim = None;
                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Kind => kind = Some(map.next_value()?),
                        Field::Nroots => nroots = Some(map.next_value()?),
                        Field::Depth => depth = Some(map.next_value::<u8>()?),
                        Field::Fcr => fcr = Some(map.next_value()?),
                        Field::Prim => prim = Some(map.next_value()?),
                        _ => {
                            map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }
                let depth = depth.ok_or_else(|| de::Error::missing_field("depth"))?;
                if depth == 0 {
                    // Same rule as `frame::decode`: an interleave depth of zero is invalid.
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Unsigned(0),
                        &"an interleave depth of at least 1",
                    ));
                }
                Ok(FrameParams {
                    kind: kind.ok_or_else(|| de::Error::missing_field("kind"))?,
                    nroots: nroots.ok_or_else(|| de::Error::missing_field("nroots"))?,
                    depth,
                    fcr: fcr.ok_or_else(|| de::Error::missing_field("fcr"))?,
                    prim: prim.ok_or_else(|| de::Error::missing_field("prim"))?,
                })
            }
        }

        deserializer.deserialize_struct("FrameParams", FRAME_PARAMS_FIELDS, ParamsVisitor)
    }
}
