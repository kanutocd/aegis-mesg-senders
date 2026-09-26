//! Optional integration contracts for external Redis and telemetry adapters.

use crate::{Delivery, DeliveryEvent, MessageId, Observer, StateStore};
use std::marker::PhantomData;
use std::sync::Arc;

/// A minimal byte-oriented key/value backend implemented by a Redis client or
/// an application-specific Redis wrapper.
#[cfg(feature = "redis")]
pub trait RedisBackend: Send + Sync {
    /// Reads a value by key. Backend failures are represented as `None`.
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    /// Writes a value with a caller-provided expiration in milliseconds.
    fn set(&self, key: &str, value: Vec<u8>, ttl_ms: u64);
}

/// Encodes and decodes normalized deliveries for a Redis-backed store.
#[cfg(feature = "redis")]
pub trait DeliveryCodec: Send + Sync {
    /// Encodes a delivery without exposing credentials or raw message content.
    fn encode(&self, delivery: &Delivery) -> Vec<u8>;
    /// Decodes a previously encoded delivery.
    fn decode(&self, bytes: &[u8]) -> Option<Delivery>;
}

/// A Redis-compatible state store using injected backend and serialization.
#[cfg(feature = "redis")]
pub struct RedisStateStore<B, C> {
    backend: Arc<B>,
    codec: Arc<C>,
    key_prefix: String,
    ttl_ms: u64,
    _marker: PhantomData<fn() -> (B, C)>,
}

#[cfg(feature = "redis")]
impl<B, C> RedisStateStore<B, C>
where
    B: RedisBackend + 'static,
    C: DeliveryCodec + 'static,
{
    /// Creates a Redis state store with an explicit key prefix and TTL.
    pub fn new(backend: Arc<B>, codec: Arc<C>, key_prefix: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            backend,
            codec,
            key_prefix: key_prefix.into(),
            ttl_ms,
            _marker: PhantomData,
        }
    }

    fn key(&self, id: &MessageId) -> String {
        format!("{}:{}", self.key_prefix, id)
    }
}

#[cfg(feature = "redis")]
impl<B, C> StateStore for RedisStateStore<B, C>
where
    B: RedisBackend + 'static,
    C: DeliveryCodec + 'static,
{
    fn load(&self, id: &MessageId) -> Option<Delivery> {
        self.backend
            .get(&self.key(id))
            .and_then(|bytes| self.codec.decode(&bytes))
    }

    fn save(&self, delivery: Delivery) {
        self.backend.set(
            &self.key(&delivery.message_id),
            self.codec.encode(&delivery),
            self.ttl_ms,
        );
    }
}

/// A telemetry sink implemented by an application logger, metrics exporter,
/// or tracing integration.
#[cfg(feature = "telemetry")]
pub trait TelemetrySink: Send + Sync {
    /// Records one already-redacted delivery event.
    fn record(&self, event: &DeliveryEvent);
}

/// Observer adapter that forwards safe delivery events to a telemetry sink.
#[cfg(feature = "telemetry")]
pub struct TelemetryObserver<S> {
    sink: Arc<S>,
}

#[cfg(feature = "telemetry")]
impl<S> TelemetryObserver<S>
where
    S: TelemetrySink + 'static,
{
    /// Creates an observer around a telemetry sink.
    pub fn new(sink: Arc<S>) -> Self {
        Self { sink }
    }
}

#[cfg(feature = "telemetry")]
impl<S> Observer for TelemetryObserver<S>
where
    S: TelemetrySink + 'static,
{
    fn observe(&self, event: DeliveryEvent) {
        self.sink.record(&event);
    }
}

#[cfg(all(test, feature = "redis", feature = "telemetry"))]
mod tests {
    use super::*;
    use crate::{Attempt, DeliveryEventKind, DeliveryStatus};
    use std::sync::Mutex;

    struct Backend {
        value: Mutex<Option<Vec<u8>>>,
    }

    impl RedisBackend for Backend {
        fn get(&self, _key: &str) -> Option<Vec<u8>> {
            self.value.lock().unwrap().clone()
        }

        fn set(&self, _key: &str, value: Vec<u8>, _ttl_ms: u64) {
            *self.value.lock().unwrap() = Some(value);
        }
    }

    struct Codec;

    impl DeliveryCodec for Codec {
        fn encode(&self, delivery: &Delivery) -> Vec<u8> {
            delivery.provider.as_bytes().to_vec()
        }

        fn decode(&self, bytes: &[u8]) -> Option<Delivery> {
            Some(Delivery {
                message_id: MessageId::new("decoded").unwrap(),
                provider: String::from_utf8(bytes.to_vec()).ok()?,
                status: DeliveryStatus::Accepted,
                provider_message_id: Some(String::from("provider-id")),
                attempts: vec![Attempt {
                    provider: String::from("provider"),
                    accepted: true,
                    error: None,
                }],
            })
        }
    }

    struct Sink {
        events: Mutex<Vec<DeliveryEvent>>,
    }

    impl TelemetrySink for Sink {
        fn record(&self, event: &DeliveryEvent) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    fn delivery() -> Delivery {
        Delivery {
            message_id: MessageId::new("message").unwrap(),
            provider: String::from("provider"),
            status: DeliveryStatus::Accepted,
            provider_message_id: Some(String::from("provider-id")),
            attempts: Vec::new(),
        }
    }

    #[test]
    fn redis_state_store_round_trips_through_injected_backend() {
        let backend = Arc::new(Backend {
            value: Mutex::new(None),
        });
        let store = RedisStateStore::new(backend.clone(), Arc::new(Codec), "delivery", 60_000);
        let original = delivery();
        store.save(original.clone());
        let loaded = store.load(&original.message_id).unwrap();
        assert_eq!(loaded.provider, original.provider);
        assert_eq!(loaded.message_id.as_str(), "decoded");
    }

    #[test]
    fn telemetry_observer_forwards_redacted_events() {
        let sink = Arc::new(Sink {
            events: Mutex::new(Vec::new()),
        });
        let observer = TelemetryObserver::new(sink.clone());
        let event = DeliveryEvent {
            message_id: MessageId::new("message").unwrap(),
            recipient: String::from("a***t"),
            provider: String::from("provider"),
            kind: DeliveryEventKind::Accepted,
            provider_message_id: Some(String::from("provider-id")),
        };
        observer.observe(event.clone());
        assert_eq!(sink.events.lock().unwrap().as_slice(), &[event]);
    }
}
