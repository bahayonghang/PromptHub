//! Property-based tests for the Window_Manager (task 15.3).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::window::*`, `error::ErrorCode`), so they need no edits to any
//! `mod.rs` — the same pattern used by `tests/folder_properties.rs` (task 5.2)
//! and `tests/security_properties.rs` (task 7.2). The shortcut registry is pure
//! in-memory state, so each case drives [`ShortcutRegistry`] directly through its
//! public functions exactly as the Command_Layer will.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 40: Shortcut conflict rejection preserves prior set
//!
//! **Validates: Requirements 20.11**

use proptest::prelude::*;
use proptest::sample::Index;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::services::window::{Shortcut, ShortcutMode, ShortcutRegistry};

// ---------------------------------------------------------------------------
// Accelerator model
// ---------------------------------------------------------------------------

/// A keyboard modifier with several textual spellings that all normalize to the
/// same canonical token. Used to build accelerators that are textually distinct
/// but bind the *same* key combination (so they conflict).
#[derive(Debug, Clone, Copy)]
enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Super,
    Cmd,
}

impl Modifier {
    /// Equivalent spellings of this modifier. `aliases()[0]` is the canonical
    /// rendering; the rest are recased/aliased variants that
    /// `normalize_accelerator` collapses to the same token.
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Modifier::Ctrl => &["Ctrl", "ctrl", "control", "Control", "CONTROL"],
            Modifier::Shift => &["Shift", "shift", "SHIFT"],
            Modifier::Alt => &["Alt", "alt", "option", "Option"],
            Modifier::Super => &["Super", "super", "meta", "win", "windows"],
            Modifier::Cmd => &["Cmd", "cmd", "command", "Command"],
        }
    }
}

const ALL_MODIFIERS: [Modifier; 5] = [
    Modifier::Ctrl,
    Modifier::Shift,
    Modifier::Alt,
    Modifier::Super,
    Modifier::Cmd,
];

/// Renders the *canonical* accelerator string for a shortcut: each modifier in
/// its primary spelling, in declaration order, followed by a per-shortcut unique
/// key token `key{index}`.
///
/// The unique key token guarantees that two shortcuts with different `key_index`
/// never collide after normalization, so a generated registry has no internal
/// conflicts regardless of which modifiers each shortcut carries.
fn canonical_accel(modifiers: &[Modifier], key_index: usize) -> String {
    let mut parts: Vec<String> = modifiers
        .iter()
        .map(|m| m.aliases()[0].to_string())
        .collect();
    parts.push(format!("key{key_index}"));
    parts.join("+")
}

/// Renders a *textually different but normalization-equivalent* accelerator for
/// the same `(modifiers, key_index)`: each modifier uses an aliased/recased
/// spelling, the key token is upper-cased, and the tokens are rotated into a
/// different order. Because `normalize_accelerator` lowercases, canonicalizes
/// modifier aliases, then sorts and de-duplicates the tokens, this always
/// collides with [`canonical_accel`] for the same inputs — exercising the
/// conflict rule rather than a trivial exact-string duplicate.
fn equivalent_accel(modifiers: &[Modifier], key_index: usize, style: usize) -> String {
    let mut parts: Vec<String> = modifiers
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let aliases = m.aliases();
            aliases[(style + i) % aliases.len()].to_string()
        })
        .collect();
    parts.push(format!("KEY{key_index}"));
    let rot = style % parts.len();
    parts.rotate_left(rot);
    parts.join("+")
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// A subset (possibly empty) of the five modifiers, with no duplicates.
fn modifier_set() -> impl Strategy<Value = Vec<Modifier>> {
    proptest::collection::vec(0usize..ALL_MODIFIERS.len(), 0..=3).prop_map(|idxs| {
        let mut seen = [false; ALL_MODIFIERS.len()];
        let mut set = Vec::new();
        for i in idxs {
            if !seen[i] {
                seen[i] = true;
                set.push(ALL_MODIFIERS[i]);
            }
        }
        set
    })
}

/// An action identifier, e.g. `toggle-window`.
fn action_name() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9-]{0,12}").unwrap()
}

/// A firing mode.
fn mode() -> impl Strategy<Value = ShortcutMode> {
    prop_oneof![Just(ShortcutMode::Global), Just(ShortcutMode::Local)]
}

/// A non-empty set of shortcut specs (1..=15). Each entry's position is its
/// unique key index, so the canonical renderings never conflict with each other.
fn shortcut_specs() -> impl Strategy<Value = Vec<(Vec<Modifier>, String, ShortcutMode)>> {
    proptest::collection::vec((modifier_set(), action_name(), mode()), 1..=15)
}

// ---------------------------------------------------------------------------
// Property 40: Shortcut conflict rejection preserves prior set
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Property 40: Shortcut conflict rejection preserves prior set.**
    ///
    /// For any set of registered shortcuts, attempting to register a shortcut
    /// that conflicts with an already-registered one is rejected with a
    /// `CONFLICT` error and leaves the previously registered set unchanged. The
    /// conflicting candidate is a reordered/aliased/recased spelling of one
    /// registered accelerator paired with a fresh action and mode, so the test
    /// confirms the conflict is detected on the *normalized* key combination and
    /// that the rejected registration neither replaces the prior entry nor alters
    /// the set in any way.
    ///
    /// **Validates: Requirements 20.11**
    #[test]
    fn shortcut_conflict_rejection_preserves_prior_set(
        specs in shortcut_specs(),
        target in any::<Index>(),
        style in 0usize..64,
        conflict_action in action_name(),
        conflict_mode in mode(),
    ) {
        let n = specs.len();

        // Build the registry from the generated (internally conflict-free) set.
        let mut reg = ShortcutRegistry::new();
        for (i, (modifiers, action, mode)) in specs.iter().enumerate() {
            let shortcut = Shortcut {
                action: action.clone(),
                accelerator: canonical_accel(modifiers, i),
                mode: *mode,
            };
            prop_assert!(
                reg.register(shortcut).is_ok(),
                "registering distinct accelerator {i} should succeed"
            );
        }
        prop_assert_eq!(reg.len(), n);

        // Snapshot the prior set before the conflicting attempt.
        let before = reg.shortcuts();

        // Craft a conflicting registration: an equivalent spelling of one
        // already-registered accelerator, but with a different action/mode.
        let t = target.index(n);
        let (modifiers_t, _, _) = &specs[t];
        let conflicting = Shortcut {
            action: conflict_action,
            accelerator: equivalent_accel(modifiers_t, t, style),
            mode: conflict_mode,
        };

        match reg.register(conflicting) {
            Ok(()) => prop_assert!(false, "conflicting registration must be rejected"),
            Err(err) => prop_assert_eq!(err.code, ErrorCode::Conflict),
        }

        // The previously registered set is unchanged (size and contents).
        prop_assert_eq!(reg.len(), n);
        prop_assert_eq!(reg.shortcuts(), before);
    }
}
