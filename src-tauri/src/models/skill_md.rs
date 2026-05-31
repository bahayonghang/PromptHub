//! Parsed SKILL.md model (Requirement 10).
//!
//! A SKILL.md document is YAML frontmatter followed by a Markdown body. The
//! parsed form preserves every frontmatter field as a name-value pair and the
//! body unchanged, which is the basis for the round-trip contract (10.3).
//!
//! Frontmatter values are kept as a `BTreeMap` so ordering is deterministic and
//! key order is normalized for the round-trip comparison. The value type is
//! `serde_json::Value` here; the SKILL.md parser (a later task) reads YAML into
//! this generic value shape.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A parsed SKILL.md document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSkillMd {
    /// Frontmatter fields preserved as name-value pairs (key order normalized).
    pub frontmatter: BTreeMap<String, serde_json::Value>,
    /// The Markdown body content, preserved unchanged.
    pub body: String,
}
