//! Deterministic seed derivation and independent RNG streams (M0-04).
//!
//! Contract (spec 20.5, to-do M0-04):
//!
//! - Every stream derives from the tuple
//!   `(root_seed, namespace, outer_seed, lifetime_index, stream_name)`.
//! - Derivation is stable: SHA-256 over a canonical string, then ChaCha8
//!   seeded from the 32-byte digest. No runtime-randomized hash, no shared
//!   mutable global RNG.
//! - Namespaces `development`, `training`, `validation`, `final_test` are
//!   reserved and disjoint; final-test seeds must never enter tuning, search,
//!   or validation.
//! - Environment streams (cue selection, mapping init/change, reward noise,
//!   timing) are independent of agent streams (actor noise, tie breaks):
//!   each producer owns a separately seeded RNG instance, so extra draws in
//!   one stream cannot alter another stream's schedule.
//!
//! Recorded RNG policy: `rand_chacha::ChaCha8Rng` (see `Cargo.lock` for the
//! pinned `rand`/`rand_chacha` versions) fed by `sha2::Sha256` over the
//! canonical string documented in [`canonical_string`]. The reference
//! platform for bitwise replay is recorded in run manifests; cross-platform
//! comparisons require declared tolerances (see AGENTS.md).

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

/// Domain separator for the derivation. Bumped only with a documented,
/// versioned change to the derivation algorithm.
pub const DERIVATION_DOMAIN: &str = "cra-v1";

/// Reserved, disjoint seed namespaces.
pub const SUPPORTED_NAMESPACES: &[&str] = &["development", "training", "validation", "final_test"];

/// Reserved stream names. Environment, initialization, actor-noise,
/// tie-break, and evolution producers must use distinct entries here so that
/// draws in one stream never perturb another stream's schedule.
pub const SUPPORTED_STREAMS: &[&str] = &[
    "cue_order",
    "mapping_init",
    "mapping_change",
    "reward_noise",
    "timing",
    "actor_noise",
    "tie_break",
    "init",
    "evolution",
];

/// A single deterministic stream identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedTuple {
    pub root_seed: u64,
    pub namespace: String,
    pub outer_seed: u64,
    pub lifetime_index: u64,
    pub stream: String,
}

impl SeedTuple {
    pub fn new(
        root_seed: u64,
        namespace: &str,
        outer_seed: u64,
        lifetime_index: u64,
        stream: &str,
    ) -> Self {
        Self {
            root_seed,
            namespace: namespace.to_owned(),
            outer_seed,
            lifetime_index,
            stream: stream.to_owned(),
        }
    }
}

/// Errors for invalid stream identities. These are configuration errors, not
/// simulation failures.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SeedError {
    #[error(
        "unknown seed namespace '{0}'; expected one of development|training|validation|final_test"
    )]
    UnknownNamespace(String),
    #[error(
        "invalid stream name '{0}'; use lowercase [a-z0-9_] (max 64 chars), e.g. one of the reserved streams"
    )]
    InvalidStream(String),
}

/// True when `ns` is one of the four reserved namespaces.
pub fn is_supported_namespace(ns: &str) -> bool {
    SUPPORTED_NAMESPACES.contains(&ns)
}

/// True when `s` is one of the reserved stream names.
pub fn is_reserved_stream(s: &str) -> bool {
    SUPPORTED_STREAMS.contains(&s)
}

fn valid_stream_charset(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Validate a tuple's namespace and stream name without drawing randomness.
pub fn validate_tuple(t: &SeedTuple) -> Result<(), SeedError> {
    if !is_supported_namespace(&t.namespace) {
        return Err(SeedError::UnknownNamespace(t.namespace.clone()));
    }
    if !valid_stream_charset(&t.stream) {
        return Err(SeedError::InvalidStream(t.stream.clone()));
    }
    Ok(())
}

/// Canonical derivation string. This exact format is part of the
/// reproducibility contract; do not change it without bumping
/// [`DERIVATION_DOMAIN`] and recording the migration.
pub fn canonical_string(t: &SeedTuple) -> String {
    format!(
        "{}|root={}|ns={}|outer={}|lifetime={}|stream={}",
        DERIVATION_DOMAIN, t.root_seed, t.namespace, t.outer_seed, t.lifetime_index, t.stream
    )
}

/// Derive the stable 32-byte seed for a tuple (validates first).
pub fn derive_seed_bytes(t: &SeedTuple) -> Result<[u8; 32], SeedError> {
    validate_tuple(t)?;
    let digest = Sha256::digest(canonical_string(t).as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

/// Lowercase hex encoding of [`derive_seed_bytes`], for golden fixtures and
/// audit logs.
pub fn derive_seed_hex(t: &SeedTuple) -> Result<String, SeedError> {
    Ok(to_hex(&derive_seed_bytes(t)?))
}

/// Build the independent RNG for exactly one stream. Each producer must hold
/// its own instance; never share one RNG across streams.
pub fn rng_for(t: &SeedTuple) -> Result<ChaCha8Rng, SeedError> {
    Ok(ChaCha8Rng::from_seed(derive_seed_bytes(t)?))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_format_is_stable() {
        let t = SeedTuple::new(1, "development", 7, 3, "cue_order");
        assert_eq!(
            canonical_string(&t),
            "cra-v1|root=1|ns=development|outer=7|lifetime=3|stream=cue_order"
        );
    }

    #[test]
    fn reserved_namespaces_and_streams_validate() {
        for ns in SUPPORTED_NAMESPACES {
            assert!(is_supported_namespace(ns), "{ns}");
        }
        for s in SUPPORTED_STREAMS {
            let t = SeedTuple::new(0, "development", 0, 0, s);
            assert!(validate_tuple(&t).is_ok(), "{s}");
        }
    }
}
