//! Deadline and retry tracking for lazily loaded resources.
//!
//! The old entity `ResourceState` had a `Requested` variant that was never
//! retried: when a `ZC_ACK_REQNAMEALL`/`ZC_ACK_REQNAMEALL2` (0x0A30/0x0ADF)
//! response was lost the entity stayed nameless for its whole lifetime and
//! `request_visible_entity_names` silently skipped it forever.
//!
//! [`ResourceState`] gives every resource an explicit lifecycle:
//!
//! `Unavailable` → (`mark_requested`) → `Requested` → (`mark_available`)
//! `Available`, or after a request timeout a new request, bounded by
//! [`ResourcePolicy::maximum_attempts`]. Once the attempts are exhausted the
//! state gives up, logs a warning and never requests again.
//!
//! [`ResourceRetryQueue`] applies the same policy to a set of keys (animation
//! data per entity, item sprites, …) so asset loads have a deadline and a
//! retry budget as well.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Retry policy for one kind of lazily loaded resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourcePolicy {
    /// How long a request may stay unanswered before it counts as lost.
    pub request_timeout: Duration,
    /// Delay between a failed load and its retry.
    pub retry_delay: Duration,
    /// Maximum number of requests (the initial one included).
    pub maximum_attempts: u32,
    /// Human readable resource name for log messages.
    pub label: &'static str,
}

impl ResourcePolicy {
    /// Animation/weapon/falcon data of an entity.
    pub const ANIMATION_DATA: Self = Self {
        request_timeout: Duration::from_secs(5),
        retry_delay: Duration::from_secs(2),
        maximum_attempts: 3,
        label: "animation data",
    };
    /// Entity details/name request (`0x0A30`/`0x0ADF`). The server normally
    /// answers within a frame or two; the request is lost when the map stream
    /// desynchronized or the server is overloaded.
    pub const ENTITY_DETAILS: Self = Self {
        request_timeout: Duration::from_secs(3),
        retry_delay: Duration::from_secs(1),
        maximum_attempts: 4,
        label: "entity details",
    };
    /// Map data load. The session machine uses the same numbers for its
    /// dedicated map load timer (the map retry is part of the session
    /// lifecycle because a failed map load aborts the map entry).
    pub const MAP_DATA: Self = Self {
        request_timeout: Duration::from_secs(8),
        retry_delay: Duration::from_secs(8),
        maximum_attempts: 4,
        label: "map data",
    };
}

/// Internal status of a [`ResourceState`].
#[derive(Debug, Clone)]
enum ResourceStatus<T> {
    /// Never requested yet.
    Unavailable,
    /// A request is in flight since the given time.
    Requested { requested_at: Instant },
    /// The resource is loaded.
    Available(T),
    /// The last load failed; retried after `retry_delay` until `gave_up`.
    Failed { failed_at: Instant, gave_up: bool },
}

/// State of one lazily loaded resource.
#[derive(Debug, Clone)]
pub struct ResourceState<T> {
    status: ResourceStatus<T>,
    attempts: u32,
    policy: ResourcePolicy,
}

impl<T> ResourceState<T> {
    /// A never-requested resource.
    pub fn new(policy: ResourcePolicy) -> Self {
        Self {
            status: ResourceStatus::Unavailable,
            attempts: 0,
            policy,
        }
    }

    /// The loaded value, if any.
    pub fn as_option(&self) -> Option<&T> {
        match &self.status {
            ResourceStatus::Available(value) => Some(value),
            _other => None,
        }
    }

    /// Record that a request was just sent. Counts as one attempt.
    pub fn mark_requested(&mut self, now: Instant) {
        self.attempts += 1;
        crate::logging::log_debug!(
            "resource {}: request {}/{}",
            self.policy.label,
            self.attempts,
            self.policy.maximum_attempts
        );
        self.status = ResourceStatus::Requested { requested_at: now };
    }

    /// The resource was loaded.
    pub fn mark_available(&mut self, value: T) {
        self.attempts = 0;
        self.status = ResourceStatus::Available(value);
    }

    /// The load failed. The resource is retried after
    /// [`ResourcePolicy::retry_delay`] until the attempt budget is exhausted.
    pub fn mark_failed(&mut self, now: Instant, error: &str) {
        let gave_up = self.attempts >= self.policy.maximum_attempts;
        self.status = ResourceStatus::Failed { failed_at: now, gave_up };

        if gave_up {
            crate::logging::log_warn!(
                "resource {}: giving up after {} attempts (last error: {})",
                self.policy.label,
                self.attempts,
                error
            );
        } else {
            crate::logging::log_warn!(
                "resource {}: load failed (attempt {}/{}), retrying: {}",
                self.policy.label,
                self.attempts,
                self.policy.maximum_attempts,
                error
            );
        }
    }

    /// Whether the caller should (re)send a request now.
    ///
    /// A request that stayed unanswered for [`ResourcePolicy::request_timeout`]
    /// counts as lost and is retried. When the attempt budget is exhausted the
    /// state gives up and logs a warning.
    pub fn needs_request(&mut self, now: Instant) -> bool {
        match self.status {
            ResourceStatus::Unavailable => true,
            ResourceStatus::Available(_) => false,
            ResourceStatus::Requested { requested_at } => {
                if now.duration_since(requested_at) < self.policy.request_timeout {
                    return false;
                }

                if self.attempts >= self.policy.maximum_attempts {
                    self.give_up(now);
                    return false;
                }

                crate::logging::log_debug!(
                    "resource {}: request timed out after {:?}, retrying",
                    self.policy.label,
                    self.policy.request_timeout
                );
                true
            }
            ResourceStatus::Failed { failed_at, gave_up } => !gave_up && now.duration_since(failed_at) >= self.policy.retry_delay,
        }
    }

    fn give_up(&mut self, now: Instant) {
        crate::logging::log_warn!(
            "resource {}: giving up after {} unanswered requests",
            self.policy.label,
            self.attempts
        );
        self.status = ResourceStatus::Failed {
            failed_at: now,
            gave_up: true,
        };
    }

    /// Number of requests that were sent so far.
    #[cfg(test)]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Whether the resource gave up loading.
    #[cfg(test)]
    pub fn gave_up(&self) -> bool {
        matches!(self.status, ResourceStatus::Failed { gave_up: true, .. })
    }
}

/// Retry tracking for a set of lazily loaded resources (animation data, item
/// sprites, …) keyed by their loader/entity id.
#[derive(Debug, Clone)]
pub struct ResourceRetryQueue<K> {
    entries: HashMap<K, ResourceState<()>>,
    policy: ResourcePolicy,
}

impl<K> ResourceRetryQueue<K>
where
    K: Eq + Hash + Clone,
{
    pub fn new(policy: ResourcePolicy) -> Self {
        Self {
            entries: HashMap::new(),
            policy,
        }
    }

    /// Start tracking a resource that is currently loading.
    pub fn note_request(&mut self, key: K, now: Instant) {
        self.entries
            .entry(key)
            .or_insert_with(|| ResourceState::new(self.policy))
            .mark_requested(now);
    }

    /// The resource loaded; stop tracking it.
    pub fn note_available(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// The load failed; keep tracking it so it is retried after the policy
    /// delay (until the attempt budget is exhausted).
    pub fn note_failed(&mut self, key: &K, now: Instant, error: &str) {
        if let Some(state) = self.entries.get_mut(key) {
            state.mark_failed(now, error);
        }
    }

    /// Stop tracking a resource (entity despawned, map changed, …).
    pub fn forget(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// Whether the resource is currently tracked.
    pub fn is_tracked(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Whether any resource is currently tracked.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop all tracked resources (e.g. on a map change).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Keys whose request timed out or whose retry delay elapsed.
    pub fn keys_due(&mut self, now: Instant) -> Vec<K> {
        let mut due = Vec::new();

        for (key, state) in self.entries.iter_mut() {
            if state.needs_request(now) {
                due.push(key.clone());
            }
        }

        due
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn details() -> ResourceState<String> {
        ResourceState::new(ResourcePolicy::ENTITY_DETAILS)
    }

    #[test]
    fn unavailable_details_are_requested_immediately() {
        let now = Instant::now();
        let mut details = details();
        assert!(details.needs_request(now));
        assert!(details.as_option().is_none());
    }

    #[test]
    fn lost_detail_response_is_retried_and_recovers() {
        let now = Instant::now();
        let mut details = details();

        // Initial request.
        assert!(details.needs_request(now));
        details.mark_requested(now);
        assert_eq!(details.attempts(), 1);
        assert!(!details.needs_request(now), "a request that is in flight must not be repeated");
        assert!(!details.needs_request(now + ResourcePolicy::ENTITY_DETAILS.request_timeout / 2));

        // The response is lost: after the timeout the request is repeated.
        assert!(details.needs_request(now + ResourcePolicy::ENTITY_DETAILS.request_timeout));
        details.mark_requested(now + ResourcePolicy::ENTITY_DETAILS.request_timeout);
        assert_eq!(details.attempts(), 2);

        // This time the server answers.
        details.mark_available("Poring".to_owned());
        assert_eq!(details.as_option().map(String::as_str), Some("Poring"));
        assert_eq!(details.attempts(), 0);
        assert!(!details.needs_request(now + Duration::from_secs(3600)));
    }

    #[test]
    fn lost_detail_responses_give_up_after_the_attempt_budget() {
        let mut now = Instant::now();
        let mut details = details();

        for attempt in 1..=ResourcePolicy::ENTITY_DETAILS.maximum_attempts {
            assert!(details.needs_request(now), "attempt {attempt} must be requested");
            details.mark_requested(now);
            now += ResourcePolicy::ENTITY_DETAILS.request_timeout;
        }

        // The last request timed out as well: the resource gives up.
        assert!(!details.needs_request(now));
        assert!(details.gave_up());
        assert!(details.as_option().is_none());
        assert!(!details.needs_request(now + Duration::from_secs(3600)));
    }

    #[test]
    fn available_resource_is_never_retried() {
        let now = Instant::now();
        let mut details = details();
        details.mark_available("Poring".to_owned());
        assert!(!details.needs_request(now));
    }

    #[test]
    fn failed_animation_load_is_retried_then_gives_up() {
        let now = Instant::now();
        let policy = ResourcePolicy::ANIMATION_DATA;
        let mut queue = ResourceRetryQueue::new(policy);
        let key = 42_u32;

        queue.note_request(key, now);
        assert!(queue.is_tracked(&key));
        assert!(queue.keys_due(now).is_empty());

        // Fail attempt 1: retry only after the retry delay.
        queue.note_failed(&key, now, "missing sprite");
        assert!(queue.keys_due(now).is_empty());
        assert_eq!(queue.keys_due(now + policy.retry_delay), vec![key]);

        // Retry until the attempt budget is exhausted.
        let mut current = now;
        for _ in 1..policy.maximum_attempts {
            current += policy.retry_delay;
            queue.note_request(key, current);
            queue.note_failed(&key, current, "missing sprite");
        }

        assert!(
            queue.keys_due(current + policy.retry_delay * 10).is_empty(),
            "a gave-up resource must never be retried"
        );
        // The entry is kept (so `is_tracked` prevents a new request); it is
        // removed when the entity despawns or the map changes.
        assert!(queue.is_tracked(&key));
    }

    #[test]
    fn retry_queue_stops_tracking_loaded_resources() {
        let now = Instant::now();
        let mut queue = ResourceRetryQueue::new(ResourcePolicy::ANIMATION_DATA);
        queue.note_request(7_u32, now);
        queue.note_available(&7_u32);
        assert!(!queue.is_tracked(&7_u32));
        assert!(queue.keys_due(now + Duration::from_secs(60)).is_empty());
    }

    #[test]
    fn retry_queue_is_cleared_on_map_change() {
        let now = Instant::now();
        let mut queue = ResourceRetryQueue::new(ResourcePolicy::ANIMATION_DATA);
        queue.note_request(1_u32, now);
        queue.note_request(2_u32, now);
        assert!(!queue.is_empty());
        queue.clear();
        assert!(queue.is_empty());
        assert!(queue.keys_due(now + Duration::from_secs(60)).is_empty());
    }

    #[test]
    fn timeout_policy_of_the_map_load_matches_the_retry_delay() {
        // The session machine derives its map retry timer from this policy.
        assert_eq!(ResourcePolicy::MAP_DATA.request_timeout, ResourcePolicy::MAP_DATA.retry_delay);
    }
}
