//! Minimal in-memory aegis-mesg-sender-core flow.

use aegis_mesg_sender_core::{
    Capabilities, Channel, CircuitBreaker, CircuitConfig, Clock, DeliveryRequest, HealthTracker,
    InMemoryObserver, InMemoryStateStore, Message, MessageId, Provider, ProviderConfig,
    ProviderRegistry, ProviderResponse, Recipient, Router, RouterPolicy, StateStore,
};
use std::sync::Arc;

struct ProviderExample;

impl Provider for ProviderExample {
    fn send(
        &self,
        _request: &DeliveryRequest,
    ) -> Result<ProviderResponse, aegis_mesg_sender_core::ProviderError> {
        Ok(ProviderResponse {
            provider_message_id: "example-1".to_owned(),
        })
    }
}

struct ExampleClock;
impl Clock for ExampleClock {
    fn now_ms(&self) -> u64 {
        0
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(ProviderRegistry::new());
    let config = ProviderConfig::new("example", 1, Capabilities::new(vec![Channel::Email]))?;
    registry.register(
        config,
        Arc::new(ProviderExample),
        CircuitBreaker::new(CircuitConfig::default())?,
        HealthTracker::new(60_000),
    )?;
    let state = Arc::new(InMemoryStateStore::default());
    let observer = Arc::new(InMemoryObserver::default());
    let router = Router::new(
        registry,
        RouterPolicy::default(),
        Arc::new(ExampleClock),
        state.clone(),
        observer.clone(),
    )?;
    let message = Message::new(
        MessageId::new("example-message")?,
        Recipient::new("alice@example.test")?,
        b"hello".to_vec(),
    )?;
    let delivery = router.deliver(DeliveryRequest {
        channel: Channel::Email,
        message,
    })?;
    assert_eq!(state.load(&delivery.message_id), Some(delivery));
    assert_eq!(observer.events().len(), 1);
    Ok(())
}
