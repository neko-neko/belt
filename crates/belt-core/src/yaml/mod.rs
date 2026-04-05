//! YAML abstraction layer.
//!
//! All YAML parsing/serialization MUST go through this module. Other modules
//! MUST NOT depend on the concrete backend (`serde-saphyr`, `serde_yml`, etc.)
//! directly. This keeps the backend swappable for security and performance
//! reasons (see spec §YAML Abstraction Layer).
//!
//! Phase 1 uses `serde-saphyr` for parsing and a custom minimal emitter
//! (`emit_yaml`) for serialization. The public API exposes a backend-agnostic
//! dynamic `Value` enum (mirroring the subset of `YAML 1.2` needed for
//! Pipeline/`RuleSet`) plus typed `parse` / `serialize` helpers built on
//! `serde::Serialize` / `serde::Deserialize`.
//!
//! Public entry points:
//! - [`parse`], [`parse_with_options`]: typed parsing via serde.
//! - [`parse_value`]: dynamic parsing into [`Value`].
//! - [`serialize`]: typed serialization (serde → [`Value`] → YAML text).
//! - [`to_value`], [`from_value`]: conversion between `T: Serialize` /
//!   `T: DeserializeOwned` and [`Value`] without a JSON round-trip.
//! - [`emit_yaml`]: direct emission of a [`Value`] tree as YAML text.
//!
//! The `serde_json` crate is intentionally NOT used by this module; all
//! conversions go through the local [`Value`] type.

use std::fmt;

use serde::Deserialize;
use serde::de::{self, DeserializeOwned, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error produced by the YAML abstraction layer.
#[derive(Debug, Error)]
pub enum YamlError {
    /// Parsing failed.
    #[error("yaml parse error: {0}")]
    Parse(String),

    /// Serialization failed.
    #[error("yaml serialize error: {0}")]
    Serialize(String),

    /// A duplicate mapping key was rejected under
    /// [`DuplicateKeyPolicy::Error`].
    #[error("duplicate mapping key: {0}")]
    DuplicateKey(String),

    /// Budget limit (depth, anchors, events) exceeded during parsing.
    #[error("yaml budget exceeded: {0}")]
    Budget(String),
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Duplicate-key handling policy.
///
/// **Phase 1 stub**: Only [`DuplicateKeyPolicy::Error`] is actually honored
/// (via the `serde-saphyr` default). [`DuplicateKeyPolicy::FirstWins`] and
/// [`DuplicateKeyPolicy::LastWins`] are declared for forward compatibility
/// and have no runtime effect in Phase 1. Full wiring into the YAML backend
/// is deferred to Phase 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateKeyPolicy {
    /// Reject duplicates as an error. This is the default and matches the
    /// spec requirement "reports duplicate key as error by default".
    #[default]
    Error,
    /// Keep the first occurrence, silently drop subsequent duplicates.
    FirstWins,
    /// Overwrite earlier occurrences with the last one.
    LastWins,
}

/// Resource budget for parsing.
///
/// **Phase 1 stub**: This struct is exposed for forward compatibility but
/// [`parse_with_options`] currently ignores all fields and always uses an
/// unlimited budget. The full wiring into the YAML backend is deferred to
/// Phase 2 once saphyr's API stabilizes.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    /// Maximum number of anchors.
    pub max_anchors: usize,
    /// Maximum document depth.
    pub max_depth: usize,
    /// Maximum number of parser events.
    pub max_events: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_anchors: 200,
            max_depth: 100,
            max_events: 50_000,
        }
    }
}

/// Options controlling a parse call.
///
/// **Phase 1 stub**: This struct is exposed for forward compatibility but
/// [`parse_with_options`] currently ignores all fields and always uses
/// [`DuplicateKeyPolicy::Error`] (the `serde-saphyr 0.0.23` default) and an
/// unlimited budget. The full wiring into the YAML backend is deferred to
/// Phase 2.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlOptions {
    /// Duplicate-key policy.
    pub duplicate_keys: DuplicateKeyPolicy,
    /// Parser budget.
    pub budget: Budget,
}

// ---------------------------------------------------------------------------
// Dynamic Value type
// ---------------------------------------------------------------------------

/// Dynamic YAML value type (backend-agnostic).
///
/// Phase 1 uses an internal `Value` enum that mirrors the subset of `YAML 1.2`
/// needed for Pipeline/`RuleSet`. This avoids depending on `serde_yml` (banned)
/// or `serde_json::Value` (which lacks `YAML`-specific constructs like non-string
/// mapping keys and distinct integer/float scalars).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `YAML` `null` (`~`, empty, or the string `null`).
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer value. `i64` is sufficient for Phase 1 (pipeline version, depth,
    /// port numbers, hook exit codes, etc.).
    Int(i64),
    /// Floating-point value. Rarely used in Pipeline/`RuleSet` but included
    /// for completeness. NaN/Inf are rejected by the emitter.
    Float(f64),
    /// Unicode string scalar.
    String(String),
    /// Ordered sequence.
    Sequence(Vec<Value>),
    /// Ordered mapping (insertion order is preserved).
    Mapping(Mapping),
}

impl Value {
    /// Returns `true` if the value is a mapping.
    #[must_use]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Value::Mapping(_))
    }

    /// Returns the underlying mapping, if this value is a mapping.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        if let Value::Mapping(m) = self {
            Some(m)
        } else {
            None
        }
    }

    /// Returns `true` if the value is a sequence.
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Value::Sequence(_))
    }

    /// Returns the underlying sequence, if this value is a sequence.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&[Value]> {
        if let Value::Sequence(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Returns the underlying string slice, if this value is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// Returns the underlying integer, if this value is an integer.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        if let Value::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    /// Returns the underlying boolean, if this value is a boolean.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Looks up `key` as a string key in a mapping.
    ///
    /// Convenience for the common case of string-keyed maps. Returns `None`
    /// if the value is not a mapping or the key is not present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_mapping()
            .and_then(|m| m.get(&Value::String(key.to_string())))
    }
}

// ---------------------------------------------------------------------------
// Mapping (insertion-ordered)
// ---------------------------------------------------------------------------

/// Insertion-ordered YAML mapping.
///
/// Backed by a `Vec<(Value, Value)>` to preserve key order. Phase 1 uses this
/// over `BTreeMap`/`HashMap` because YAML mapping order is observable (the
/// emitter must round-trip deterministically and the spec pipelines expect
/// stable field order).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mapping {
    entries: Vec<(Value, Value)>,
}

impl Mapping {
    /// Creates an empty mapping.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts a `(key, value)` pair. If the key already exists, the existing
    /// entry is updated in place (preserving its original position) and the
    /// previous value is returned.
    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        if let Some(existing) = self.entries.iter_mut().find(|(k, _)| k == &key) {
            let prev = std::mem::replace(&mut existing.1, value);
            return Some(prev);
        }
        self.entries.push((key, value));
        None
    }

    /// Returns an iterator over `(key, value)` pairs in insertion order.
    pub fn iter(&self) -> std::slice::Iter<'_, (Value, Value)> {
        self.entries.iter()
    }

    /// Looks up a value by key (by exact `Value` equality).
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the mapping has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<'a> IntoIterator for &'a Mapping {
    type Item = &'a (Value, Value);
    type IntoIter = std::slice::Iter<'a, (Value, Value)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl IntoIterator for Mapping {
    type Item = (Value, Value);
    type IntoIter = std::vec::IntoIter<(Value, Value)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

// ---------------------------------------------------------------------------
// serde::Serialize / Deserialize for Value
// ---------------------------------------------------------------------------

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int(i) => serializer.serialize_i64(*i),
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::String(s) => serializer.serialize_str(s),
            Value::Sequence(seq) => {
                let mut s = serializer.serialize_seq(Some(seq.len()))?;
                for item in seq {
                    s.serialize_element(item)?;
                }
                s.end()
            }
            Value::Mapping(m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in &m.entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any valid YAML value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Value::Int(v))
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Self::Value, E> {
        i64::try_from(v).map_or_else(
            |_| Err(E::custom(format!("integer out of i64 range: {v}"))),
            |i| Ok(Value::Int(i)),
        )
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        i64::try_from(v).map_or_else(
            |_| Err(E::custom(format!("integer out of i64 range: {v}"))),
            |i| Ok(Value::Int(i)),
        )
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
        i64::try_from(v).map_or_else(
            |_| Err(E::custom(format!("integer out of i64 range: {v}"))),
            |i| Ok(Value::Int(i)),
        )
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Value::String(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
        Ok(Value::String(v.to_string()))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        Value::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element::<Value>()? {
            items.push(item);
        }
        Ok(Value::Sequence(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut out = Mapping::new();
        while let Some((k, v)) = map.next_entry::<Value, Value>()? {
            out.insert(k, v);
        }
        Ok(Value::Mapping(out))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parses `text` as YAML into a typed value `T`.
///
/// # Errors
///
/// Returns [`YamlError::Parse`] on backend errors, or
/// [`YamlError::DuplicateKey`] when the default
/// [`DuplicateKeyPolicy::Error`] rejects a duplicate.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, YamlError> {
    parse_with_options(text, YamlOptions::default())
}

/// Parses `text` as YAML into a typed value `T` with custom [`YamlOptions`].
///
/// Phase 1 wires only the default saphyr options; the `_options` parameter
/// is a forward-compatibility stub. Takes [`YamlOptions`] by value because
/// the struct is [`Copy`] and small.
///
/// # Errors
///
/// Returns [`YamlError::Parse`] on backend errors or
/// [`YamlError::DuplicateKey`] on duplicate-key rejection.
pub fn parse_with_options<T: DeserializeOwned>(
    text: &str,
    _options: YamlOptions,
) -> Result<T, YamlError> {
    match serde_saphyr::from_str::<T>(text) {
        Ok(v) => Ok(v),
        Err(e) => Err(classify_parse_error(&e.to_string())),
    }
}

/// Parses `text` as YAML into a dynamic [`Value`].
///
/// # Errors
///
/// Returns [`YamlError::Parse`] or [`YamlError::DuplicateKey`].
pub fn parse_value(text: &str) -> Result<Value, YamlError> {
    match serde_saphyr::from_str::<Value>(text) {
        Ok(v) => Ok(v),
        Err(e) => Err(classify_parse_error(&e.to_string())),
    }
}

/// Maps a backend error message to the correct [`YamlError`] variant.
fn classify_parse_error(msg: &str) -> YamlError {
    let lower = msg.to_lowercase();
    if lower.contains("duplicate") {
        YamlError::DuplicateKey(msg.to_string())
    } else {
        YamlError::Parse(msg.to_string())
    }
}

/// Serializes a typed value to YAML text.
///
/// The value is first converted to a [`Value`] tree via [`to_value`], then
/// emitted by [`emit_yaml`].
///
/// # Errors
///
/// Returns [`YamlError::Serialize`] on any conversion or emission failure.
pub fn serialize<T: Serialize>(value: &T) -> Result<String, YamlError> {
    let v = to_value(value)?;
    emit_yaml(&v)
}

/// Converts a typed value to a dynamic [`Value`].
///
/// # Errors
///
/// Returns [`YamlError::Serialize`] if the serializer fails.
pub fn to_value<T: Serialize>(value: &T) -> Result<Value, YamlError> {
    value_serializer::to_yaml_value(value)
}

/// Converts a dynamic [`Value`] to a typed value.
///
/// # Errors
///
/// Returns [`YamlError::Parse`] if the deserializer fails (e.g. missing
/// field or type mismatch).
pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, YamlError> {
    value_serializer::from_yaml_value(value)
}

// ---------------------------------------------------------------------------
// Minimal YAML emitter
// ---------------------------------------------------------------------------

/// Emits a [`Value`] tree as YAML text.
///
/// The emitter follows the Phase 1 spec rules:
/// - 2-space indentation
/// - Trailing newline
/// - Quote scalars that are ambiguous (reserved words, numeric-looking
///   strings, whitespace-edge strings, strings with YAML special characters)
/// - Block style for non-empty mappings and sequences
/// - Flow style `{}` / `[]` for empty mappings/sequences
/// - Non-string mapping keys and `NaN`/`Inf` floats are rejected
///
/// # Errors
///
/// Returns [`YamlError::Serialize`] on unsupported input (e.g. a non-string
/// mapping key or a non-finite float).
pub fn emit_yaml(value: &Value) -> Result<String, YamlError> {
    let mut out = String::new();
    emit_document_root(value, &mut out)?;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn emit_document_root(value: &Value, out: &mut String) -> Result<(), YamlError> {
    match value {
        Value::Mapping(m) if !m.is_empty() => emit_mapping_block(m, 0, out),
        Value::Sequence(s) if !s.is_empty() => emit_sequence_block(s, 0, out),
        other => {
            emit_inline_scalar_or_flow(other, out)?;
            Ok(())
        }
    }
}

fn write_indent(indent: usize, out: &mut String) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn emit_mapping_block(m: &Mapping, indent: usize, out: &mut String) -> Result<(), YamlError> {
    for (k, v) in &m.entries {
        write_indent(indent, out);
        let key_str = mapping_key_as_string(k)?;
        out.push_str(&format_scalar_key(&key_str));
        out.push(':');
        match v {
            Value::Mapping(child) if !child.is_empty() => {
                out.push('\n');
                emit_mapping_block(child, indent + 1, out)?;
            }
            Value::Sequence(seq) if !seq.is_empty() => {
                out.push('\n');
                emit_sequence_block(seq, indent + 1, out)?;
            }
            Value::Mapping(_) => {
                // empty mapping
                out.push_str(" {}\n");
            }
            Value::Sequence(_) => {
                // empty sequence
                out.push_str(" []\n");
            }
            scalar => {
                out.push(' ');
                emit_inline_scalar_or_flow(scalar, out)?;
                out.push('\n');
            }
        }
    }
    Ok(())
}

fn emit_sequence_block(seq: &[Value], indent: usize, out: &mut String) -> Result<(), YamlError> {
    for item in seq {
        write_indent(indent, out);
        out.push('-');
        match item {
            Value::Mapping(m) if !m.is_empty() => {
                // Inline the first key on the `-` line, then emit remaining
                // keys indented to `indent + 1`.
                out.push(' ');
                emit_mapping_inline_first(m, indent + 1, out)?;
            }
            Value::Sequence(inner) if !inner.is_empty() => {
                out.push('\n');
                emit_sequence_block(inner, indent + 1, out)?;
            }
            Value::Mapping(_) => {
                out.push_str(" {}\n");
            }
            Value::Sequence(_) => {
                out.push_str(" []\n");
            }
            scalar => {
                out.push(' ');
                emit_inline_scalar_or_flow(scalar, out)?;
                out.push('\n');
            }
        }
    }
    Ok(())
}

/// Emits a non-empty mapping inside a sequence element, with the first key
/// appearing on the same line as the `- ` marker and subsequent keys indented
/// to `inner_indent`.
fn emit_mapping_inline_first(
    m: &Mapping,
    inner_indent: usize,
    out: &mut String,
) -> Result<(), YamlError> {
    for (idx, (k, v)) in m.entries.iter().enumerate() {
        if idx > 0 {
            write_indent(inner_indent, out);
        }
        let key_str = mapping_key_as_string(k)?;
        out.push_str(&format_scalar_key(&key_str));
        out.push(':');
        match v {
            Value::Mapping(child) if !child.is_empty() => {
                out.push('\n');
                emit_mapping_block(child, inner_indent + 1, out)?;
            }
            Value::Sequence(seq) if !seq.is_empty() => {
                out.push('\n');
                emit_sequence_block(seq, inner_indent + 1, out)?;
            }
            Value::Mapping(_) => {
                out.push_str(" {}\n");
            }
            Value::Sequence(_) => {
                out.push_str(" []\n");
            }
            scalar => {
                out.push(' ');
                emit_inline_scalar_or_flow(scalar, out)?;
                out.push('\n');
            }
        }
    }
    Ok(())
}

/// Emits a scalar (or empty flow container) without a trailing newline. Used
/// for values placed after `key: ` or `- `.
fn emit_inline_scalar_or_flow(value: &Value, out: &mut String) -> Result<(), YamlError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(i) => {
            out.push_str(&i.to_string());
        }
        Value::Float(f) => {
            if !f.is_finite() {
                return Err(YamlError::Serialize(format!(
                    "non-finite float is not supported by the Phase 1 emitter: {f}"
                )));
            }
            // Ensure the printed form round-trips as a YAML float.
            let printed = format_finite_float(*f);
            out.push_str(&printed);
        }
        Value::String(s) => {
            out.push_str(&format_scalar_value(s));
        }
        Value::Sequence(seq) => {
            // Only empty sequences reach this path (non-empty are handled
            // by block emission).
            if seq.is_empty() {
                out.push_str("[]");
            } else {
                return Err(YamlError::Serialize(
                    "non-empty sequence not allowed in inline position".to_string(),
                ));
            }
        }
        Value::Mapping(m) => {
            if m.is_empty() {
                out.push_str("{}");
            } else {
                return Err(YamlError::Serialize(
                    "non-empty mapping not allowed in inline position".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Returns the string representation of a mapping key, or an error if the
/// key is not a string-compatible scalar. Phase 1 only supports string keys
/// (per spec). Integer/bool keys are rejected to keep the emitter minimal and
/// round-trip safe.
fn mapping_key_as_string(key: &Value) -> Result<String, YamlError> {
    match key {
        Value::String(s) => Ok(s.clone()),
        _ => Err(YamlError::Serialize(
            "non-string mapping key not supported in Phase 1 emitter".to_string(),
        )),
    }
}

/// Formats a mapping key using the same quote rule as regular scalars.
fn format_scalar_key(s: &str) -> String {
    format_scalar_value(s)
}

/// Formats a string scalar value, applying the Phase 1 quote rule.
fn format_scalar_value(s: &str) -> String {
    if needs_quoting(s) {
        quote_double(s)
    } else {
        s.to_string()
    }
}

/// Returns `true` if `s` must be emitted as a double-quoted YAML scalar.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if is_reserved_word(s) {
        return true;
    }
    if parses_as_yaml_number(s) {
        return true;
    }
    // Leading or trailing whitespace.
    if s.starts_with(|c: char| c.is_whitespace()) || s.ends_with(|c: char| c.is_whitespace()) {
        return true;
    }
    // Strings that begin with a YAML block indicator must be quoted even if
    // none of their characters are YAML-special in isolation. Otherwise the
    // emitter would write e.g. `- - foo` (a nested sequence) instead of the
    // scalar `"- foo"`, and shell argv strings like "--help" or regex patterns
    // starting with "?" would be mis-parsed. See `needs_quoting_*` tests.
    if starts_with_indicator(s) {
        return true;
    }
    for c in s.chars() {
        if is_yaml_special_char(c) {
            return true;
        }
        if is_control_char(c) {
            return true;
        }
    }
    false
}

/// Returns `true` if `s` starts with a YAML block indicator that would cause
/// a plain scalar to be mis-parsed at the start of a line.
///
/// Specifically:
/// - The document markers `---` and `...`.
/// - `-`, `?`, or `:` when they are the only character or are followed by a
///   space or tab (these are YAML's block sequence entry, complex mapping key,
///   and block mapping value indicators).
fn starts_with_indicator(s: &str) -> bool {
    if s == "---" || s == "..." {
        return true;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '-' | '?' | ':') {
        return false;
    }
    match chars.next() {
        // Lone indicator character at the start of a line is also a YAML
        // block marker (e.g. `-` on its own line = empty sequence item).
        None => true,
        Some(next) if next == ' ' || next == '\t' => true,
        _ => false,
    }
}

fn is_reserved_word(s: &str) -> bool {
    matches!(
        s,
        "true"
            | "false"
            | "null"
            | "yes"
            | "no"
            | "on"
            | "off"
            | "True"
            | "False"
            | "Null"
            | "Yes"
            | "No"
            | "On"
            | "Off"
            | "TRUE"
            | "FALSE"
            | "NULL"
            | "YES"
            | "NO"
            | "ON"
            | "OFF"
            | "~"
    )
}

fn parses_as_yaml_number(s: &str) -> bool {
    if s.parse::<i64>().is_ok() {
        return true;
    }
    if s.parse::<u64>().is_ok() {
        return true;
    }
    if let Ok(f) = s.parse::<f64>() {
        if f.is_finite() {
            return true;
        }
    }
    // YAML-specific forms (`.inf`, `-.inf`, `.nan`).
    matches!(s, ".inf" | "-.inf" | "+.inf" | ".nan" | ".Inf" | ".NaN")
}

fn is_yaml_special_char(c: char) -> bool {
    matches!(
        c,
        ':' | '#'
            | '['
            | ']'
            | '{'
            | '}'
            | ','
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    )
}

fn is_control_char(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t') || (c.is_control() && c != ' ')
}

fn quote_double(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => {
                // Encode as \xNN / \uNNNN for printable round-trip safety.
                let code = other as u32;
                if code <= 0xff {
                    // `write!` on `String` cannot fail; any error would be a
                    // logic bug in `std::fmt` which we cannot recover from.
                    let _ = write!(out, "\\x{code:02x}");
                } else {
                    let _ = write!(out, "\\u{code:04x}");
                }
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn format_finite_float(f: f64) -> String {
    // Ensure the printed form parses back as a float (not int). Append `.0`
    // when Rust prints an integer-valued float as a bare integer.
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') || s.contains('E') || s.contains("inf") || s == "NaN" {
        s
    } else {
        format!("{s}.0")
    }
}

// ---------------------------------------------------------------------------
// value_serializer — serde Serializer/Deserializer targeting yaml::Value
// ---------------------------------------------------------------------------

mod value_serializer {
    //! A custom `serde::Serializer` and `serde::Deserializer` that target the
    //! local [`Value`] type directly. This avoids a `serde_json` round-trip
    //! and keeps the YAML abstraction fully backend-agnostic, per the spec.

    use std::fmt;

    use serde::de::{
        self, DeserializeOwned, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
        Visitor,
    };
    use serde::ser::{self, Serialize};

    use super::{Mapping, Value, YamlError};

    // -------------------------------------------------------------------
    // Serialize side: T -> Value
    // -------------------------------------------------------------------

    pub(super) fn to_yaml_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, YamlError> {
        value.serialize(YamlValueSerializer)
    }

    #[derive(Debug)]
    pub(super) struct SerError(String);

    impl fmt::Display for SerError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for SerError {}

    impl ser::Error for SerError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            Self(msg.to_string())
        }
    }

    impl From<SerError> for YamlError {
        fn from(e: SerError) -> Self {
            YamlError::Serialize(e.0)
        }
    }

    struct YamlValueSerializer;

    type SerResult = Result<Value, YamlError>;

    impl ser::Serializer for YamlValueSerializer {
        type Ok = Value;
        type Error = YamlError;

        type SerializeSeq = SeqSerializer;
        type SerializeTuple = SeqSerializer;
        type SerializeTupleStruct = SeqSerializer;
        type SerializeTupleVariant = TupleVariantSerializer;
        type SerializeMap = MapSerializer;
        type SerializeStruct = MapSerializer;
        type SerializeStructVariant = StructVariantSerializer;

        fn serialize_bool(self, v: bool) -> SerResult {
            Ok(Value::Bool(v))
        }

        fn serialize_i8(self, v: i8) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_i16(self, v: i16) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_i32(self, v: i32) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_i64(self, v: i64) -> SerResult {
            Ok(Value::Int(v))
        }
        fn serialize_i128(self, v: i128) -> SerResult {
            i64::try_from(v).map_or_else(
                |_| {
                    Err(YamlError::Serialize(format!(
                        "integer out of i64 range: {v}"
                    )))
                },
                |i| Ok(Value::Int(i)),
            )
        }

        fn serialize_u8(self, v: u8) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_u16(self, v: u16) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_u32(self, v: u32) -> SerResult {
            Ok(Value::Int(i64::from(v)))
        }
        fn serialize_u64(self, v: u64) -> SerResult {
            i64::try_from(v).map_or_else(
                |_| {
                    Err(YamlError::Serialize(format!(
                        "unsigned integer out of i64 range: {v}"
                    )))
                },
                |i| Ok(Value::Int(i)),
            )
        }
        fn serialize_u128(self, v: u128) -> SerResult {
            i64::try_from(v).map_or_else(
                |_| {
                    Err(YamlError::Serialize(format!(
                        "unsigned integer out of i64 range: {v}"
                    )))
                },
                |i| Ok(Value::Int(i)),
            )
        }

        fn serialize_f32(self, v: f32) -> SerResult {
            Ok(Value::Float(f64::from(v)))
        }
        fn serialize_f64(self, v: f64) -> SerResult {
            Ok(Value::Float(v))
        }

        fn serialize_char(self, v: char) -> SerResult {
            Ok(Value::String(v.to_string()))
        }
        fn serialize_str(self, v: &str) -> SerResult {
            Ok(Value::String(v.to_string()))
        }
        fn serialize_bytes(self, v: &[u8]) -> SerResult {
            // Represent bytes as a sequence of ints (Phase 1 compromise;
            // pipelines never serialize raw bytes).
            let items: Vec<Value> = v
                .iter()
                .copied()
                .map(|b| Value::Int(i64::from(b)))
                .collect();
            Ok(Value::Sequence(items))
        }

        fn serialize_none(self) -> SerResult {
            Ok(Value::Null)
        }
        fn serialize_some<T>(self, value: &T) -> SerResult
        where
            T: ?Sized + Serialize,
        {
            value.serialize(self)
        }

        fn serialize_unit(self) -> SerResult {
            Ok(Value::Null)
        }
        fn serialize_unit_struct(self, _name: &'static str) -> SerResult {
            Ok(Value::Null)
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> SerResult {
            Ok(Value::String(variant.to_string()))
        }

        fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> SerResult
        where
            T: ?Sized + Serialize,
        {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> SerResult
        where
            T: ?Sized + Serialize,
        {
            let inner = value.serialize(YamlValueSerializer)?;
            let mut m = Mapping::new();
            m.insert(Value::String(variant.to_string()), inner);
            Ok(Value::Mapping(m))
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SeqSerializer {
                items: Vec::with_capacity(len.unwrap_or(0)),
            })
        }
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(SeqSerializer {
                items: Vec::with_capacity(len),
            })
        }
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(SeqSerializer {
                items: Vec::with_capacity(len),
            })
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(TupleVariantSerializer {
                variant,
                items: Vec::with_capacity(len),
            })
        }
        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(MapSerializer {
                mapping: Mapping::new(),
                pending_key: None,
            })
        }
        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(MapSerializer {
                mapping: Mapping::new(),
                pending_key: None,
            })
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(StructVariantSerializer {
                variant,
                mapping: Mapping::new(),
            })
        }
    }

    pub(super) struct SeqSerializer {
        items: Vec<Value>,
    }

    impl ser::SerializeSeq for SeqSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let v = value.serialize(YamlValueSerializer)?;
            self.items.push(v);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Sequence(self.items))
        }
    }

    impl ser::SerializeTuple for SeqSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            <Self as ser::SerializeSeq>::serialize_element(self, value)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            <Self as ser::SerializeSeq>::end(self)
        }
    }

    impl ser::SerializeTupleStruct for SeqSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            <Self as ser::SerializeSeq>::serialize_element(self, value)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            <Self as ser::SerializeSeq>::end(self)
        }
    }

    pub(super) struct TupleVariantSerializer {
        variant: &'static str,
        items: Vec<Value>,
    }

    impl ser::SerializeTupleVariant for TupleVariantSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let v = value.serialize(YamlValueSerializer)?;
            self.items.push(v);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            let mut m = Mapping::new();
            m.insert(
                Value::String(self.variant.to_string()),
                Value::Sequence(self.items),
            );
            Ok(Value::Mapping(m))
        }
    }

    pub(super) struct MapSerializer {
        mapping: Mapping,
        pending_key: Option<Value>,
    }

    impl ser::SerializeMap for MapSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let k = key.serialize(YamlValueSerializer)?;
            self.pending_key = Some(k);
            Ok(())
        }
        fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let v = value.serialize(YamlValueSerializer)?;
            let k = self.pending_key.take().ok_or_else(|| {
                YamlError::Serialize("map value without preceding key".to_string())
            })?;
            self.mapping.insert(k, v);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Mapping(self.mapping))
        }
    }

    impl ser::SerializeStruct for MapSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let v = value.serialize(YamlValueSerializer)?;
            self.mapping.insert(Value::String(key.to_string()), v);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Mapping(self.mapping))
        }
    }

    pub(super) struct StructVariantSerializer {
        variant: &'static str,
        mapping: Mapping,
    }

    impl ser::SerializeStructVariant for StructVariantSerializer {
        type Ok = Value;
        type Error = YamlError;
        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: ?Sized + Serialize,
        {
            let v = value.serialize(YamlValueSerializer)?;
            self.mapping.insert(Value::String(key.to_string()), v);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            let mut outer = Mapping::new();
            outer.insert(
                Value::String(self.variant.to_string()),
                Value::Mapping(self.mapping),
            );
            Ok(Value::Mapping(outer))
        }
    }

    // Required by serde: Serializer::Error must be an `ser::Error` implementation.
    impl ser::Error for YamlError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            YamlError::Serialize(msg.to_string())
        }
    }

    // -------------------------------------------------------------------
    // Deserialize side: Value -> T
    // -------------------------------------------------------------------

    pub(super) fn from_yaml_value<T: DeserializeOwned>(value: Value) -> Result<T, YamlError> {
        T::deserialize(ValueDeserializer { value })
    }

    struct ValueDeserializer {
        value: Value,
    }

    impl<'de> de::Deserializer<'de> for ValueDeserializer {
        type Error = YamlError;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self.value {
                Value::Null => visitor.visit_unit(),
                Value::Bool(b) => visitor.visit_bool(b),
                Value::Int(i) => visitor.visit_i64(i),
                Value::Float(f) => visitor.visit_f64(f),
                Value::String(s) => visitor.visit_string(s),
                Value::Sequence(items) => visitor.visit_seq(SeqAccessImpl::new(items.into_iter())),
                Value::Mapping(m) => visitor.visit_map(MapAccessImpl::new(m.into_iter())),
            }
        }

        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self.value {
                Value::Null => visitor.visit_none(),
                other => visitor.visit_some(ValueDeserializer { value: other }),
            }
        }

        fn deserialize_enum<V>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self.value {
                // Unit variant, e.g. `"FooVariant"`.
                Value::String(s) => visitor.visit_enum(EnumAccessUnit { variant: s }),
                // Variant with payload, encoded as `{ variant_name: payload }`.
                Value::Mapping(m) => {
                    let mut iter = m.into_iter();
                    let (k, v) = iter.next().ok_or_else(|| {
                        YamlError::Parse("expected singleton mapping for enum".to_string())
                    })?;
                    if iter.next().is_some() {
                        return Err(YamlError::Parse(
                            "expected singleton mapping for enum, got multiple keys".to_string(),
                        ));
                    }
                    let Value::String(variant) = k else {
                        return Err(YamlError::Parse(
                            "enum variant key must be a string".to_string(),
                        ));
                    };
                    visitor.visit_enum(EnumAccessWithPayload {
                        variant,
                        payload: v,
                    })
                }
                _ => Err(YamlError::Parse(
                    "invalid YAML value for enum deserialization".to_string(),
                )),
            }
        }

        // Forward everything else to `deserialize_any`. This is the standard
        // idiom for a self-describing format.
        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf unit unit_struct newtype_struct seq tuple
            tuple_struct map struct identifier ignored_any
        }
    }

    impl de::Error for YamlError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            YamlError::Parse(msg.to_string())
        }
    }

    struct SeqAccessImpl {
        iter: std::vec::IntoIter<Value>,
    }

    impl SeqAccessImpl {
        fn new(iter: std::vec::IntoIter<Value>) -> Self {
            Self { iter }
        }
    }

    impl<'de> SeqAccess<'de> for SeqAccessImpl {
        type Error = YamlError;
        fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            match self.iter.next() {
                Some(v) => seed.deserialize(ValueDeserializer { value: v }).map(Some),
                None => Ok(None),
            }
        }
        fn size_hint(&self) -> Option<usize> {
            Some(self.iter.len())
        }
    }

    struct MapAccessImpl {
        iter: std::vec::IntoIter<(Value, Value)>,
        next_value: Option<Value>,
    }

    impl MapAccessImpl {
        fn new(iter: std::vec::IntoIter<(Value, Value)>) -> Self {
            Self {
                iter,
                next_value: None,
            }
        }
    }

    impl<'de> MapAccess<'de> for MapAccessImpl {
        type Error = YamlError;
        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: de::DeserializeSeed<'de>,
        {
            match self.iter.next() {
                Some((k, v)) => {
                    self.next_value = Some(v);
                    seed.deserialize(ValueDeserializer { value: k }).map(Some)
                }
                None => Ok(None),
            }
        }
        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            let v = self.next_value.take().ok_or_else(|| {
                YamlError::Parse("map value requested without preceding key".to_string())
            })?;
            seed.deserialize(ValueDeserializer { value: v })
        }
        fn size_hint(&self) -> Option<usize> {
            Some(self.iter.len())
        }
    }

    struct EnumAccessUnit {
        variant: String,
    }

    impl<'de> EnumAccess<'de> for EnumAccessUnit {
        type Error = YamlError;
        type Variant = UnitVariantAccess;
        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            let deserializer: de::value::StringDeserializer<YamlError> =
                self.variant.into_deserializer();
            let value = seed.deserialize(deserializer)?;
            Ok((value, UnitVariantAccess))
        }
    }

    struct UnitVariantAccess;

    impl<'de> VariantAccess<'de> for UnitVariantAccess {
        type Error = YamlError;
        fn unit_variant(self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            Err(YamlError::Parse(
                "expected unit variant, got newtype".to_string(),
            ))
        }
        fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            Err(YamlError::Parse(
                "expected unit variant, got tuple".to_string(),
            ))
        }
        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            Err(YamlError::Parse(
                "expected unit variant, got struct".to_string(),
            ))
        }
    }

    struct EnumAccessWithPayload {
        variant: String,
        payload: Value,
    }

    impl<'de> EnumAccess<'de> for EnumAccessWithPayload {
        type Error = YamlError;
        type Variant = PayloadVariantAccess;
        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            let deserializer: de::value::StringDeserializer<YamlError> =
                self.variant.into_deserializer();
            let value = seed.deserialize(deserializer)?;
            Ok((
                value,
                PayloadVariantAccess {
                    payload: self.payload,
                },
            ))
        }
    }

    struct PayloadVariantAccess {
        payload: Value,
    }

    impl<'de> VariantAccess<'de> for PayloadVariantAccess {
        type Error = YamlError;
        fn unit_variant(self) -> Result<(), Self::Error> {
            match self.payload {
                Value::Null => Ok(()),
                _ => Err(YamlError::Parse(
                    "expected unit variant payload (null)".to_string(),
                )),
            }
        }
        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            seed.deserialize(ValueDeserializer {
                value: self.payload,
            })
        }
        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            if let Value::Sequence(items) = self.payload {
                visitor.visit_seq(SeqAccessImpl::new(items.into_iter()))
            } else {
                Err(YamlError::Parse(
                    "expected sequence payload for tuple variant".to_string(),
                ))
            }
        }
        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            if let Value::Mapping(m) = self.payload {
                visitor.visit_map(MapAccessImpl::new(m.into_iter()))
            } else {
                Err(YamlError::Parse(
                    "expected mapping payload for struct variant".to_string(),
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_rejects_nan_float() {
        let v = Value::Float(f64::NAN);
        let err = emit_yaml(&v).expect_err("nan must be rejected");
        assert!(matches!(err, YamlError::Serialize(_)));
    }

    #[test]
    fn emit_rejects_inf_float() {
        let v = Value::Float(f64::INFINITY);
        let err = emit_yaml(&v).expect_err("inf must be rejected");
        assert!(matches!(err, YamlError::Serialize(_)));
    }

    #[test]
    fn emit_rejects_non_string_mapping_key() {
        let mut m = Mapping::new();
        m.insert(Value::Int(1), Value::String("x".into()));
        let v = Value::Mapping(m);
        let err = emit_yaml(&v).expect_err("non-string key must be rejected");
        match err {
            YamlError::Serialize(msg) => {
                assert!(msg.contains("non-string"), "msg: {msg}");
            }
            other => panic!("expected Serialize error, got {other:?}"),
        }
    }

    #[test]
    fn emit_empty_mapping_uses_flow_style() {
        let v = Value::Mapping(Mapping::new());
        let out = emit_yaml(&v).expect("emit ok");
        assert_eq!(out, "{}\n");
    }

    #[test]
    fn emit_empty_sequence_uses_flow_style() {
        let v = Value::Sequence(Vec::new());
        let out = emit_yaml(&v).expect("emit ok");
        assert_eq!(out, "[]\n");
    }

    #[test]
    fn emit_string_with_colon_is_quoted() {
        let s = format_scalar_value("a:b");
        assert_eq!(s, "\"a:b\"");
    }

    #[test]
    fn emit_string_with_hash_is_quoted() {
        assert_eq!(
            format_scalar_value("# not a comment"),
            "\"# not a comment\""
        );
    }

    #[test]
    fn emit_string_plain_unquoted() {
        assert_eq!(format_scalar_value("hello"), "hello");
        assert_eq!(format_scalar_value("belt"), "belt");
    }

    #[test]
    fn emit_string_empty_is_quoted() {
        assert_eq!(format_scalar_value(""), "\"\"");
    }

    #[test]
    fn emit_string_leading_space_is_quoted() {
        assert_eq!(format_scalar_value(" leading"), "\" leading\"");
    }

    #[test]
    fn needs_quoting_reserved_words_all_case_variants() {
        for w in [
            "true", "false", "null", "yes", "no", "on", "off", "True", "False", "Null", "Yes",
            "No", "On", "Off", "TRUE", "FALSE", "NULL", "YES", "NO", "ON", "OFF", "~",
        ] {
            assert!(needs_quoting(w), "{w} should be quoted");
        }
    }

    #[test]
    fn needs_quoting_numeric_like() {
        for n in ["0", "1", "-1", "3.14", "1e9", "1.0e-9"] {
            assert!(needs_quoting(n), "{n} should be quoted");
        }
    }

    #[test]
    fn needs_quoting_leading_dash_followed_by_space() {
        assert!(needs_quoting("- leading"));
        assert!(needs_quoting("- "));
        assert!(needs_quoting("-\ttab"));
    }

    #[test]
    fn needs_quoting_lone_dash() {
        // "-" alone is also a block indicator at the start of a line.
        // (YAML treats `-` followed by EOL as an empty sequence item.)
        // Quoting it is the safest option for round-trip.
        assert!(needs_quoting("-"));
    }

    #[test]
    fn needs_quoting_leading_question_followed_by_space() {
        assert!(needs_quoting("? key"));
        assert!(needs_quoting("?"));
    }

    #[test]
    fn needs_quoting_document_start_marker() {
        assert!(needs_quoting("---"));
    }

    #[test]
    fn needs_quoting_document_end_marker() {
        assert!(needs_quoting("..."));
    }

    #[test]
    fn mapping_insert_updates_in_place_preserving_order() {
        let mut m = Mapping::new();
        m.insert(Value::String("a".into()), Value::Int(1));
        m.insert(Value::String("b".into()), Value::Int(2));
        m.insert(Value::String("c".into()), Value::Int(3));
        let prev = m.insert(Value::String("b".into()), Value::Int(22));
        assert_eq!(prev, Some(Value::Int(2)));
        // Order: a, b, c — with b's value updated.
        let entries: Vec<_> = m.iter().collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, Value::String("a".into()));
        assert_eq!(entries[1].0, Value::String("b".into()));
        assert_eq!(entries[1].1, Value::Int(22));
        assert_eq!(entries[2].0, Value::String("c".into()));
    }

    #[test]
    fn round_trip_nested_struct_via_value() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Nested {
            outer: String,
            inner: Inner,
        }
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Inner {
            tag: String,
            values: Vec<i64>,
        }
        let n = Nested {
            outer: "hello".into(),
            inner: Inner {
                tag: "t".into(),
                values: vec![1, 2, 3],
            },
        };
        let yaml = serialize(&n).expect("serialize ok");
        let back: Nested = parse(&yaml).expect("parse ok");
        assert_eq!(back, n);
    }

    #[test]
    fn parse_value_nested_sequence_and_mapping() {
        let text = "name: belt\nitems:\n  - a\n  - b\nmeta:\n  version: 1\n";
        let v = parse_value(text).expect("parse ok");
        assert_eq!(v.get("name").and_then(Value::as_str), Some("belt"));
        let items = v.get("items").expect("items").as_sequence().expect("seq");
        assert_eq!(items.len(), 2);
        let meta = v.get("meta").expect("meta");
        assert_eq!(meta.get("version").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn emit_nested_mapping_uses_two_space_indent() {
        let mut inner = Mapping::new();
        inner.insert(Value::String("tag".into()), Value::String("t".into()));
        let mut outer = Mapping::new();
        outer.insert(Value::String("outer".into()), Value::String("hello".into()));
        outer.insert(Value::String("inner".into()), Value::Mapping(inner));
        let v = Value::Mapping(outer);
        let out = emit_yaml(&v).expect("emit ok");
        assert_eq!(out, "outer: hello\ninner:\n  tag: t\n");
    }

    #[test]
    fn emit_sequence_of_maps_inline_first_key() {
        let mut m1 = Mapping::new();
        m1.insert(Value::String("k".into()), Value::Int(1));
        let mut m2 = Mapping::new();
        m2.insert(Value::String("k".into()), Value::Int(2));
        let v = Value::Sequence(vec![Value::Mapping(m1), Value::Mapping(m2)]);
        let out = emit_yaml(&v).expect("emit ok");
        assert_eq!(out, "- k: 1\n- k: 2\n");
    }

    #[test]
    fn parse_error_reported_as_parse_variant() {
        // Malformed YAML: unbalanced flow mapping delimiter.
        let err = parse_value("{ key: value").expect_err("must error");
        assert!(matches!(err, YamlError::Parse(_)));
    }
}
