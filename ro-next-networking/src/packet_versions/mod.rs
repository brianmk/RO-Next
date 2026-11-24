pub mod version_20220406;

/// All supported packet versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedPacketVersion {
    _20220406,
}

impl SupportedPacketVersion {
    /// Every packet version this client has handler registrations for.
    ///
    /// Tests iterate this list instead of hard-coding a version, so adding a
    /// new variant automatically extends the duplicate-registration and
    /// "handler creation succeeds" coverage.
    pub const ALL: &'static [Self] = &[Self::_20220406];
}

pub mod noop_policy;
