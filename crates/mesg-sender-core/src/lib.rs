//! Channel-neutral delivery contracts and runtime.
//!
//! The crate intentionally contains no transport, channel SDK, database, or
//! async-runtime dependency. Applications provide those integrations through
//! [`Provider`], [`StateStore`], [`Observer`], and [`Clock`].

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(feature = "redis", feature = "telemetry"))]
pub mod integrations;

/// A channel identifier used for capability matching.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Channel {
    /// Email delivery.
    Email,
    /// SMS delivery.
    Sms,
    /// An application-defined channel.
    Custom(String),
}

/// An opaque message identifier supplied by the caller.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MessageId(String);

impl MessageId {
    /// Creates an identifier, rejecting empty values.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::Empty("message id"));
        }
        Ok(Self(value))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A recipient that intentionally exposes only a redacted representation in
/// diagnostics and events.
#[derive(Clone, Eq, PartialEq)]
pub struct Recipient(String);

impl Recipient {
    /// Creates a recipient, rejecting empty values.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::Empty("recipient"));
        }
        Ok(Self(value))
    }

    /// Returns a stable, non-sensitive representation for logs and events.
    pub fn redacted(&self) -> String {
        let chars: Vec<char> = self.0.chars().collect();
        if chars.len() <= 2 {
            return "**".to_owned();
        }
        format!("{}***{}", chars[0], chars[chars.len() - 1])
    }
}

impl fmt::Debug for Recipient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Recipient")
            .field(&self.redacted())
            .finish()
    }
}

/// An immutable channel-neutral message.
#[derive(Clone, Eq, PartialEq)]
pub struct Message {
    /// The caller-assigned message identifier.
    pub id: MessageId,
    /// The destination recipient.
    pub recipient: Recipient,
    /// Opaque content interpreted by a channel adapter.
    pub body: Vec<u8>,
    /// Non-sensitive metadata used by adapters and routing policy.
    pub metadata: BTreeMap<String, String>,
}

impl fmt::Debug for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Message")
            .field("id", &self.id)
            .field("recipient", &self.recipient)
            .field("body_bytes", &self.body.len())
            .field("metadata_keys", &self.metadata.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Message {
    /// Validates and constructs a message.
    pub fn new(
        id: MessageId,
        recipient: Recipient,
        body: Vec<u8>,
    ) -> Result<Self, ValidationError> {
        if body.is_empty() {
            return Err(ValidationError::Empty("message body"));
        }
        Ok(Self {
            id,
            recipient,
            body,
            metadata: BTreeMap::new(),
        })
    }

    /// Adds metadata while preserving the message's value semantics.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// A request passed to a provider adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryRequest {
    /// The channel being delivered.
    pub channel: Channel,
    /// The immutable message.
    pub message: Message,
}

/// Provider capabilities used during election.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capabilities {
    /// Channels supported by the provider instance.
    pub channels: Vec<Channel>,
    /// Optional maximum message size in bytes.
    pub max_body_bytes: Option<usize>,
}

impl Capabilities {
    /// Creates capabilities for the supplied channels.
    pub fn new(channels: impl Into<Vec<Channel>>) -> Self {
        Self {
            channels: channels.into(),
            max_body_bytes: None,
        }
    }

    /// Sets the maximum body size.
    #[must_use]
    pub fn with_max_body_bytes(mut self, limit: usize) -> Self {
        self.max_body_bytes = Some(limit);
        self
    }

    fn supports(&self, request: &DeliveryRequest) -> bool {
        self.channels.contains(&request.channel)
            && self
                .max_body_bytes
                .map_or(true, |limit| request.message.body.len() <= limit)
    }
}

/// Configuration for a named provider instance.
#[derive(Clone, Eq, PartialEq)]
pub struct ProviderConfig {
    /// Stable instance name.
    pub name: String,
    /// Higher values are preferred during election.
    pub priority: i32,
    /// Declared provider capabilities.
    pub capabilities: Capabilities,
    /// An opaque credential retained by the adapter boundary.
    pub credential: Option<String>,
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderConfig")
            .field("name", &self.name)
            .field("priority", &self.priority)
            .field("capabilities", &self.capabilities)
            .field(
                "credential",
                &self.credential.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

impl ProviderConfig {
    /// Creates provider configuration.
    pub fn new(
        name: impl Into<String>,
        priority: i32,
        capabilities: Capabilities,
    ) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ValidationError::Empty("provider name"));
        }
        Ok(Self {
            name,
            priority,
            capabilities,
            credential: None,
        })
    }

    /// Attaches an opaque credential to the configuration.
    #[must_use]
    pub fn with_credential(mut self, credential: impl Into<String>) -> Self {
        self.credential = Some(credential.into());
        self
    }
}

/// A provider adapter implemented by a channel-specific integration.
pub trait Provider: Send + Sync {
    /// Attempts one provider submission.
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError>;
}

/// A successful provider submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderResponse {
    /// Provider-side correlation identifier.
    pub provider_message_id: String,
}

/// Error categories normalized by the runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Credentials or authentication were rejected.
    Authentication,
    /// The request was invalid and should not be retried.
    InvalidRequest,
    /// The provider rate limit was reached.
    RateLimited,
    /// The provider was unavailable.
    Unavailable,
    /// The provider request timed out.
    Timeout,
    /// The adapter encountered an internal error.
    Internal,
    /// An unclassified provider error.
    Unknown,
}

impl ErrorKind {
    /// Returns the default retry policy for this normalized category.
    #[must_use]
    pub const fn default_retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable | Self::Timeout)
    }

    /// Returns the default provider-failover policy for this normalized
    /// category. Authentication failures may use another configured provider,
    /// while invalid requests must not be retried elsewhere.
    #[must_use]
    pub const fn default_failover(self) -> bool {
        !matches!(self, Self::InvalidRequest)
    }
}

/// An error returned by a provider adapter.
#[derive(Clone, Eq, PartialEq)]
pub struct ProviderError {
    /// Normalized category.
    pub kind: ErrorKind,
    /// Safe diagnostic detail; must not contain secrets or raw content.
    pub message: String,
    /// Whether another attempt may succeed.
    pub retryable: bool,
    /// Whether another provider may be attempted.
    pub failover: bool,
}

impl fmt::Debug for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderError")
            .field("kind", &self.kind)
            .field("retryable", &self.retryable)
            .field("failover", &self.failover)
            .finish()
    }
}

impl ProviderError {
    /// Creates a normalized provider error.
    pub fn new(
        kind: ErrorKind,
        message: impl Into<String>,
        retryable: bool,
        failover: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
            failover,
        }
    }

    /// Creates an error using the runtime's default policy for its category.
    /// Adapters should use [`Self::new`] when provider-specific behavior needs
    /// a different policy.
    pub fn from_kind(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::new(
            kind,
            message,
            kind.default_retryable(),
            kind.default_failover(),
        )
    }

    /// Returns whether retrying the same provider is permitted.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }

    /// Returns whether another provider may be attempted.
    #[must_use]
    pub const fn should_failover(&self) -> bool {
        self.failover
    }

    /// Returns whether this error permits neither retry nor failover.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        !self.retryable && !self.failover
    }

    /// Creates a terminal invalid-request error.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::from_kind(ErrorKind::InvalidRequest, message)
    }
}

/// Validation failures detected before provider election.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    /// A required value was empty.
    Empty(&'static str),
    /// No registered provider supports the request.
    NoEligibleProvider,
    /// A resilience or routing policy is invalid.
    InvalidPolicy(&'static str),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ValidationError {}

/// Normalized delivery state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryStatus {
    /// The provider accepted the submission.
    Accepted,
    /// A later provider event confirmed delivery.
    Delivered,
    /// A later provider event confirmed failure.
    Failed,
}

/// The durable normalized result of a delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Delivery {
    /// Caller message identifier.
    pub message_id: MessageId,
    /// Selected provider instance.
    pub provider: String,
    /// Current normalized state.
    pub status: DeliveryStatus,
    /// Provider correlation identifier, when available.
    pub provider_message_id: Option<String>,
    /// Attempt history, safe for persistence and diagnostics.
    pub attempts: Vec<Attempt>,
}

/// A single provider attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attempt {
    /// Provider instance name.
    pub provider: String,
    /// Whether the attempt was accepted.
    pub accepted: bool,
    /// Safe normalized error category, if rejected.
    pub error: Option<ErrorKind>,
}

/// Events emitted as delivery state changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryEventKind {
    /// The provider accepted the submission.
    Accepted,
    /// Delivery was confirmed.
    Delivered,
    /// Delivery failed.
    Failed,
}

/// A safe structured delivery event. It contains no credentials, raw body, or
/// full recipient.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryEvent {
    /// Message identifier.
    pub message_id: MessageId,
    /// Redacted recipient representation.
    pub recipient: String,
    /// Provider instance name.
    pub provider: String,
    /// State transition.
    pub kind: DeliveryEventKind,
    /// Provider correlation identifier, when available.
    pub provider_message_id: Option<String>,
}

/// A provider registry containing independent named instances.
pub struct ProviderRegistry {
    providers: RwLock<BTreeMap<String, RegisteredProvider>>,
}

struct RegisteredProvider {
    config: ProviderConfig,
    provider: Arc<dyn Provider>,
    circuit: Mutex<CircuitBreaker>,
    health: Mutex<HealthTracker>,
}

impl ProviderRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(BTreeMap::new()),
        }
    }

    /// Registers or replaces a named provider instance.
    pub fn register(
        &self,
        config: ProviderConfig,
        provider: Arc<dyn Provider>,
        circuit: CircuitBreaker,
        health: HealthTracker,
    ) -> Result<(), ValidationError> {
        if config.name.trim().is_empty() {
            return Err(ValidationError::Empty("provider name"));
        }
        self.providers
            .write()
            .expect("registry lock poisoned")
            .insert(
                config.name.clone(),
                RegisteredProvider {
                    config,
                    provider,
                    circuit: Mutex::new(circuit),
                    health: Mutex::new(health),
                },
            );
        Ok(())
    }

    /// Returns whether a named provider is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.providers
            .read()
            .expect("registry lock poisoned")
            .contains_key(name)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A clock abstraction for deterministic resilience tests.
pub trait Clock: Send + Sync {
    /// Returns monotonic milliseconds.
    fn now_ms(&self) -> u64;
}

/// A system clock suitable for production use.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis() as u64)
    }
}

/// Circuit breaker settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitConfig {
    /// Consecutive failures before opening.
    pub failure_threshold: u32,
    /// Cooldown before half-open probing.
    pub cooldown_ms: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown_ms: 30_000,
        }
    }
}

/// Circuit state exposed in health snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitState {
    /// Normal operation.
    Closed,
    /// Calls are blocked until cooldown expires.
    Open,
    /// One probe is allowed to test recovery.
    HalfOpen,
}

/// A lock-protected circuit breaker state machine.
#[derive(Clone, Debug)]
pub struct CircuitBreaker {
    config: CircuitConfig,
    state: CircuitState,
    failures: u32,
    opened_at: Option<u64>,
    probe_in_flight: bool,
}

impl CircuitBreaker {
    /// Creates a closed circuit breaker.
    pub fn new(config: CircuitConfig) -> Result<Self, ValidationError> {
        if config.failure_threshold == 0 {
            return Err(ValidationError::InvalidPolicy(
                "failure threshold must be positive",
            ));
        }
        Ok(Self {
            config,
            state: CircuitState::Closed,
            failures: 0,
            opened_at: None,
            probe_in_flight: false,
        })
    }

    /// Returns the current state, transitioning an expired open circuit to
    /// half-open.
    pub fn state(&mut self, now_ms: u64) -> CircuitState {
        if self.state == CircuitState::Open
            && now_ms.saturating_sub(self.opened_at.unwrap_or(now_ms)) >= self.config.cooldown_ms
        {
            self.state = CircuitState::HalfOpen;
        }
        self.state
    }

    /// Returns whether an attempt may start.
    pub fn allow(&mut self, now_ms: u64) -> bool {
        match self.state(now_ms) {
            CircuitState::Closed => true,
            CircuitState::HalfOpen if !self.probe_in_flight => {
                self.probe_in_flight = true;
                true
            }
            CircuitState::HalfOpen => false,
            CircuitState::Open => false,
        }
    }

    /// Returns whether the circuit is eligible without claiming a half-open
    /// probe slot.
    pub fn is_available(&mut self, now_ms: u64) -> bool {
        self.state(now_ms) != CircuitState::Open
    }

    /// Records a successful attempt.
    pub fn record_success(&mut self) {
        self.state = CircuitState::Closed;
        self.failures = 0;
        self.opened_at = None;
        self.probe_in_flight = false;
    }

    /// Records a failed attempt and opens after the configured threshold.
    pub fn record_failure(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= self.config.failure_threshold {
            self.state = CircuitState::Open;
            self.opened_at = Some(now_ms);
            self.probe_in_flight = false;
        }
    }
}

/// A rolling provider health tracker.
#[derive(Clone, Debug)]
pub struct HealthTracker {
    window_ms: u64,
    samples: VecDeque<HealthSample>,
}

#[derive(Clone, Copy, Debug)]
struct HealthSample {
    at_ms: u64,
    success: bool,
}

impl HealthTracker {
    /// Creates a rolling tracker with the supplied time window.
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            samples: VecDeque::new(),
        }
    }
    /// Records an outcome.
    pub fn record(&mut self, now_ms: u64, success: bool) {
        self.samples.push_back(HealthSample {
            at_ms: now_ms,
            success,
        });
        self.prune(now_ms);
    }
    /// Returns a score from 0.0 to 1.0, defaulting to neutral health.
    pub fn score(&mut self, now_ms: u64) -> f32 {
        self.prune(now_ms);
        if self.samples.is_empty() {
            return 1.0;
        }
        self.samples.iter().filter(|sample| sample.success).count() as f32
            / self.samples.len() as f32
    }
    fn prune(&mut self, now_ms: u64) {
        while self
            .samples
            .front()
            .is_some_and(|sample| now_ms.saturating_sub(sample.at_ms) > self.window_ms)
        {
            self.samples.pop_front();
        }
    }
}

/// Routing policy controlling election and bounded failover.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouterPolicy {
    /// Optional explicit provider instance.
    pub provider: Option<String>,
    /// Maximum number of provider attempts.
    pub max_attempts: usize,
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self {
            provider: None,
            max_attempts: 3,
        }
    }
}

/// Persistence abstraction for normalized deliveries.
pub trait StateStore: Send + Sync {
    /// Loads a delivery by message identifier.
    fn load(&self, id: &MessageId) -> Option<Delivery>;
    /// Saves a delivery.
    fn save(&self, delivery: Delivery);
}

/// An in-memory state store for tests and single-process applications.
#[derive(Default)]
pub struct InMemoryStateStore {
    deliveries: Mutex<BTreeMap<MessageId, Delivery>>,
}

impl StateStore for InMemoryStateStore {
    fn load(&self, id: &MessageId) -> Option<Delivery> {
        self.deliveries
            .lock()
            .expect("state lock poisoned")
            .get(id)
            .cloned()
    }
    fn save(&self, delivery: Delivery) {
        self.deliveries
            .lock()
            .expect("state lock poisoned")
            .insert(delivery.message_id.clone(), delivery);
    }
}

/// Structured event sink abstraction.
pub trait Observer: Send + Sync {
    /// Receives one safe delivery event.
    fn observe(&self, event: DeliveryEvent);
}

/// An in-memory observer useful for deterministic tests.
#[derive(Default)]
pub struct InMemoryObserver {
    events: Mutex<Vec<DeliveryEvent>>,
}

impl InMemoryObserver {
    /// Returns a snapshot of emitted events.
    pub fn events(&self) -> Vec<DeliveryEvent> {
        self.events.lock().expect("observer lock poisoned").clone()
    }
}

impl Observer for InMemoryObserver {
    fn observe(&self, event: DeliveryEvent) {
        self.events
            .lock()
            .expect("observer lock poisoned")
            .push(event);
    }
}

/// A delivery router performing validation, election, bounded failover, and
/// normalized state persistence.
pub struct Router {
    registry: Arc<ProviderRegistry>,
    policy: RouterPolicy,
    clock: Arc<dyn Clock>,
    state: Arc<dyn StateStore>,
    observer: Arc<dyn Observer>,
}

impl Router {
    /// Constructs a router from its injected boundaries.
    pub fn new(
        registry: Arc<ProviderRegistry>,
        policy: RouterPolicy,
        clock: Arc<dyn Clock>,
        state: Arc<dyn StateStore>,
        observer: Arc<dyn Observer>,
    ) -> Result<Self, ValidationError> {
        if policy.max_attempts == 0 {
            return Err(ValidationError::InvalidPolicy(
                "maximum attempts must be positive",
            ));
        }
        Ok(Self {
            registry,
            policy,
            clock,
            state,
            observer,
        })
    }

    /// Routes one request with bounded failover.
    pub fn deliver(&self, request: DeliveryRequest) -> Result<Delivery, RouteError> {
        if request.message.body.is_empty() {
            return Err(RouteError::Validation(ValidationError::Empty(
                "message body",
            )));
        }
        let now = self.clock.now_ms();
        let candidates = self.elect(&request, now)?;
        let mut attempts = Vec::new();
        for name in candidates.into_iter().take(self.policy.max_attempts) {
            let providers = self
                .registry
                .providers
                .read()
                .expect("registry lock poisoned");
            let registered = providers
                .get(&name)
                .expect("elected provider must remain registered");
            let mut circuit = registered.circuit.lock().expect("circuit lock poisoned");
            if !circuit.allow(now) {
                continue;
            }
            drop(circuit);
            let result = registered.provider.send(&request);
            match result {
                Ok(response) => {
                    registered
                        .circuit
                        .lock()
                        .expect("circuit lock poisoned")
                        .record_success();
                    registered
                        .health
                        .lock()
                        .expect("health lock poisoned")
                        .record(now, true);
                    attempts.push(Attempt {
                        provider: registered.config.name.clone(),
                        accepted: true,
                        error: None,
                    });
                    let delivery = Delivery {
                        message_id: request.message.id.clone(),
                        provider: registered.config.name.clone(),
                        status: DeliveryStatus::Accepted,
                        provider_message_id: Some(response.provider_message_id.clone()),
                        attempts,
                    };
                    self.state.save(delivery.clone());
                    self.observer.observe(DeliveryEvent {
                        message_id: request.message.id,
                        recipient: request.message.recipient.redacted(),
                        provider: delivery.provider.clone(),
                        kind: DeliveryEventKind::Accepted,
                        provider_message_id: delivery.provider_message_id.clone(),
                    });
                    return Ok(delivery);
                }
                Err(error) => {
                    registered
                        .circuit
                        .lock()
                        .expect("circuit lock poisoned")
                        .record_failure(now);
                    registered
                        .health
                        .lock()
                        .expect("health lock poisoned")
                        .record(now, false);
                    let should_failover = error.failover;
                    let kind = error.kind;
                    attempts.push(Attempt {
                        provider: registered.config.name.clone(),
                        accepted: false,
                        error: Some(kind),
                    });
                    if !should_failover {
                        return Err(RouteError::Provider { error, attempts });
                    }
                }
            }
        }
        Err(RouteError::NoProvider { attempts })
    }

    fn elect(&self, request: &DeliveryRequest, now: u64) -> Result<Vec<String>, RouteError> {
        let guard = self
            .registry
            .providers
            .read()
            .expect("registry lock poisoned");
        if let Some(name) = &self.policy.provider {
            let provider = guard
                .get(name)
                .ok_or_else(|| RouteError::UnknownProvider(name.clone()))?;
            if !provider.config.capabilities.supports(request) {
                return Err(RouteError::NoEligibleProvider);
            }
            if !provider
                .circuit
                .lock()
                .expect("circuit lock poisoned")
                .is_available(now)
            {
                return Err(RouteError::NoEligibleProvider);
            }
            return Ok(vec![provider.config.name.clone()]);
        }
        let mut candidates: Vec<&RegisteredProvider> = guard
            .values()
            .filter(|provider| {
                provider.config.capabilities.supports(request)
                    && provider
                        .circuit
                        .lock()
                        .expect("circuit lock poisoned")
                        .is_available(now)
            })
            .collect();
        candidates.sort_by(|left, right| {
            let left_score = left.config.priority as f32
                + left.health.lock().expect("health lock poisoned").score(now);
            let right_score = right.config.priority as f32
                + right
                    .health
                    .lock()
                    .expect("health lock poisoned")
                    .score(now);
            right_score
                .partial_cmp(&left_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.config.name.cmp(&right.config.name))
        });
        if candidates.is_empty() {
            Err(RouteError::NoEligibleProvider)
        } else {
            Ok(candidates
                .into_iter()
                .map(|provider| provider.config.name.clone())
                .collect())
        }
    }
}

/// Routing failures with structured attempt history.
#[derive(Debug)]
pub enum RouteError {
    /// Request validation failed.
    Validation(ValidationError),
    /// An explicit provider was not registered.
    UnknownProvider(String),
    /// No provider could accept the request.
    NoEligibleProvider,
    /// A provider returned a terminal or exhausted error.
    Provider {
        /// The normalized provider error.
        error: ProviderError,
        /// Attempts made before the error.
        attempts: Vec<Attempt>,
    },
    /// All failover attempts were exhausted.
    NoProvider {
        /// Attempts made before exhaustion.
        attempts: Vec<Attempt>,
    },
}

impl fmt::Display for RouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for RouteError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct FakeClock(AtomicU64);
    impl FakeClock {
        fn new(value: u64) -> Self {
            Self(AtomicU64::new(value))
        }
    }
    impl Clock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    struct FakeProvider {
        response: Result<ProviderResponse, ProviderError>,
    }
    impl Provider for FakeProvider {
        fn send(&self, _request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
            self.response.clone()
        }
    }

    fn request() -> DeliveryRequest {
        DeliveryRequest {
            channel: Channel::Email,
            message: Message::new(
                MessageId::new("m-1").unwrap(),
                Recipient::new("alice@example.test").unwrap(),
                b"hello".to_vec(),
            )
            .unwrap(),
        }
    }

    fn provider(
        name: &str,
        priority: i32,
        result: Result<ProviderResponse, ProviderError>,
    ) -> (ProviderConfig, Arc<FakeProvider>) {
        (
            ProviderConfig::new(name, priority, Capabilities::new(vec![Channel::Email])).unwrap(),
            Arc::new(FakeProvider { response: result }),
        )
    }

    #[test]
    fn terminal_error_does_not_fail_over() {
        let registry = Arc::new(ProviderRegistry::new());
        let (config, adapter) =
            provider("a", 10, Err(ProviderError::invalid_request("bad request")));
        registry
            .register(
                config,
                adapter,
                CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                HealthTracker::new(1_000),
            )
            .unwrap();
        let (config, adapter) = provider(
            "b",
            1,
            Ok(ProviderResponse {
                provider_message_id: "b-1".into(),
            }),
        );
        registry
            .register(
                config,
                adapter,
                CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                HealthTracker::new(1_000),
            )
            .unwrap();
        let result = Router::new(
            registry,
            RouterPolicy::default(),
            Arc::new(FakeClock::new(0)),
            Arc::new(InMemoryStateStore::default()),
            Arc::new(InMemoryObserver::default()),
        )
        .unwrap()
        .deliver(request());
        assert!(
            matches!(result, Err(RouteError::Provider { attempts, .. }) if attempts.len() == 1)
        );
    }

    #[test]
    fn normalized_error_policy_is_explicit_and_queryable() {
        let rate_limited = ProviderError::from_kind(ErrorKind::RateLimited, "slow down");
        assert!(rate_limited.is_retryable());
        assert!(rate_limited.should_failover());
        assert!(!rate_limited.is_terminal());

        let invalid = ProviderError::from_kind(ErrorKind::InvalidRequest, "bad request");
        assert!(!invalid.is_retryable());
        assert!(!invalid.should_failover());
        assert!(invalid.is_terminal());
    }

    #[test]
    fn failover_is_bounded_and_records_attempts() {
        let registry = Arc::new(ProviderRegistry::new());
        for name in ["a", "b", "c"] {
            let (config, adapter) = provider(
                name,
                1,
                Err(ProviderError::new(
                    ErrorKind::Unavailable,
                    "down",
                    true,
                    true,
                )),
            );
            registry
                .register(
                    config,
                    adapter,
                    CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                    HealthTracker::new(1_000),
                )
                .unwrap();
        }
        let result = Router::new(
            registry,
            RouterPolicy {
                provider: None,
                max_attempts: 2,
            },
            Arc::new(FakeClock::new(0)),
            Arc::new(InMemoryStateStore::default()),
            Arc::new(InMemoryObserver::default()),
        )
        .unwrap()
        .deliver(request());
        assert!(matches!(result, Err(RouteError::NoProvider { attempts }) if attempts.len() == 2));
    }

    #[test]
    fn election_uses_priority_then_stable_name() {
        let registry = Arc::new(ProviderRegistry::new());
        for name in ["zeta", "alpha"] {
            let (config, adapter) = provider(
                name,
                10,
                Ok(ProviderResponse {
                    provider_message_id: name.into(),
                }),
            );
            registry
                .register(
                    config,
                    adapter,
                    CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                    HealthTracker::new(1_000),
                )
                .unwrap();
        }
        let result = Router::new(
            registry,
            RouterPolicy::default(),
            Arc::new(FakeClock::new(0)),
            Arc::new(InMemoryStateStore::default()),
            Arc::new(InMemoryObserver::default()),
        )
        .unwrap()
        .deliver(request())
        .unwrap();
        assert_eq!(result.provider, "alpha");
    }

    #[test]
    fn circuit_opens_and_then_allows_probe() {
        let mut circuit = CircuitBreaker::new(CircuitConfig {
            failure_threshold: 2,
            cooldown_ms: 10,
        })
        .unwrap();
        circuit.record_failure(0);
        assert!(circuit.allow(0));
        circuit.record_failure(0);
        assert!(!circuit.allow(5));
        assert!(circuit.allow(10));
        assert!(!circuit.allow(10));
        circuit.record_success();
        assert_eq!(circuit.state(10), CircuitState::Closed);
    }

    #[test]
    fn events_are_redacted_and_state_is_saved() {
        let registry = Arc::new(ProviderRegistry::new());
        let (config, adapter) = provider(
            "provider",
            1,
            Ok(ProviderResponse {
                provider_message_id: "corr".into(),
            }),
        );
        registry
            .register(
                config,
                adapter,
                CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                HealthTracker::new(1_000),
            )
            .unwrap();
        let state = Arc::new(InMemoryStateStore::default());
        let observer = Arc::new(InMemoryObserver::default());
        let delivery = Router::new(
            registry,
            RouterPolicy::default(),
            Arc::new(FakeClock::new(0)),
            state.clone(),
            observer.clone(),
        )
        .unwrap()
        .deliver(request())
        .unwrap();
        assert_eq!(state.load(&delivery.message_id), Some(delivery));
        assert_eq!(observer.events()[0].recipient, "a***t");
    }

    #[test]
    fn capability_filtering_rejects_unsupported_requests_before_send() {
        let registry = Arc::new(ProviderRegistry::new());
        let config = ProviderConfig::new(
            "email-only",
            1,
            Capabilities::new(vec![Channel::Email]).with_max_body_bytes(4),
        )
        .unwrap();
        registry
            .register(
                config,
                Arc::new(FakeProvider {
                    response: Ok(ProviderResponse {
                        provider_message_id: "never".into(),
                    }),
                }),
                CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                HealthTracker::new(1_000),
            )
            .unwrap();
        let result = Router::new(
            registry,
            RouterPolicy::default(),
            Arc::new(FakeClock::new(0)),
            Arc::new(InMemoryStateStore::default()),
            Arc::new(InMemoryObserver::default()),
        )
        .unwrap()
        .deliver(request());
        assert!(matches!(result, Err(RouteError::NoEligibleProvider)));
    }

    #[test]
    fn explicit_provider_selection_is_enforced() {
        let registry = Arc::new(ProviderRegistry::new());
        let (config, adapter) = provider(
            "available",
            1,
            Ok(ProviderResponse {
                provider_message_id: "available-1".into(),
            }),
        );
        registry
            .register(
                config,
                adapter,
                CircuitBreaker::new(CircuitConfig::default()).unwrap(),
                HealthTracker::new(1_000),
            )
            .unwrap();
        let result = Router::new(
            registry,
            RouterPolicy {
                provider: Some("missing".into()),
                max_attempts: 1,
            },
            Arc::new(FakeClock::new(0)),
            Arc::new(InMemoryStateStore::default()),
            Arc::new(InMemoryObserver::default()),
        )
        .unwrap()
        .deliver(request());
        assert!(matches!(result, Err(RouteError::UnknownProvider(name)) if name == "missing"));
    }

    #[test]
    fn provider_debug_redacts_credentials_and_events_redact_recipients() {
        let config = ProviderConfig::new("provider", 1, Capabilities::new(vec![Channel::Email]))
            .unwrap()
            .with_credential("super-secret-token");
        let debug = format!("{config:?}");
        assert!(!debug.contains("super-secret-token"));
        assert_eq!(Recipient::new("x").unwrap().redacted(), "**");
    }

    #[test]
    fn debug_output_redacts_message_body_and_provider_error_detail() {
        let message = Message::new(
            MessageId::new("m-1").unwrap(),
            Recipient::new("alice@example.test").unwrap(),
            b"message-secret".to_vec(),
        )
        .unwrap();
        let message_debug = format!("{message:?}");
        assert!(!message_debug.contains("message-secret"));
        assert!(message_debug.contains("body_bytes"));

        let error = ProviderError::new(
            ErrorKind::Internal,
            "authorization secret and raw body",
            false,
            false,
        );
        let error_debug = format!("{error:?}");
        assert!(!error_debug.contains("authorization secret"));
        assert!(!error_debug.contains("raw body"));
    }

    #[test]
    fn in_memory_store_and_observer_are_safe_across_threads() {
        let state = Arc::new(InMemoryStateStore::default());
        let observer = Arc::new(InMemoryObserver::default());
        let mut workers = Vec::new();
        for index in 0..8 {
            let state = Arc::clone(&state);
            let observer = Arc::clone(&observer);
            workers.push(std::thread::spawn(move || {
                let id = MessageId::new(format!("message-{index}")).unwrap();
                let delivery = Delivery {
                    message_id: id.clone(),
                    provider: "provider".into(),
                    status: DeliveryStatus::Accepted,
                    provider_message_id: Some(format!("provider-{index}")),
                    attempts: vec![],
                };
                state.save(delivery);
                observer.observe(DeliveryEvent {
                    message_id: id,
                    recipient: "a***t".into(),
                    provider: "provider".into(),
                    kind: DeliveryEventKind::Accepted,
                    provider_message_id: Some(format!("provider-{index}")),
                });
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(observer.events().len(), 8);
        assert!(state.load(&MessageId::new("message-3").unwrap()).is_some());
    }
}
