//! Credential-gated live provider tests.

#![cfg(feature = "live-tests")]

use aegis_email_sender::{
    to_delivery_request, EmailMessage, HttpProviderConfig, HttpRequest, HttpResponse,
    HttpTransport, Mailgun, Mailpit, Resend, TransportError,
};
use aegis_mesg_sender_core::{MessageId, Provider};
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new().build(),
        }
    }
}

impl HttpTransport for UreqTransport {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let mut builder = self.agent.request(&request.method, &request.url);
        for (name, value) in request.headers {
            builder = builder.set(&name, &value);
        }
        match builder.send_bytes(&request.body) {
            Ok(response) => {
                let status = response.status();
                let mut body = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut body)
                    .map_err(|_| TransportError {
                        message: "provider response could not be read".into(),
                        retryable: false,
                    })?;
                Ok(HttpResponse {
                    status,
                    headers: BTreeMap::new(),
                    body,
                })
            }
            Err(ureq::Error::Status(status, _)) => Err(TransportError {
                message: format!("provider returned HTTP {status}"),
                retryable: status >= 500 || status == 429,
            }),
            Err(_) => Err(TransportError {
                message: "provider request failed".into(),
                retryable: true,
            }),
        }
    }
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} before running live tests"))
}

fn message(from: String, to: String) -> EmailMessage {
    EmailMessage::builder(from)
        .to(to)
        .subject("Aegis live integration test")
        .text("This message was sent by the opt-in Aegis live provider test.")
        .html("<p>Aegis live integration test.</p>")
        .build()
        .expect("live test email is valid")
}

#[test]
#[ignore = "sends a real email; run explicitly with --features live-tests -- --ignored"]
fn resend_live() {
    let transport = Arc::new(UreqTransport::new());
    let provider = Resend::new(
        HttpProviderConfig::new("https://api.resend.com", required("AEGIS_RESEND_API_KEY"))
            .unwrap(),
        transport,
    );
    let email = message(
        required("AEGIS_RESEND_FROM"),
        required("AEGIS_LIVE_EMAIL_TO"),
    );
    let request = to_delivery_request(&email, &MessageId::new("live-resend").unwrap()).unwrap();
    provider.send(&request).expect("Resend accepted live email");
}

#[test]
#[ignore = "sends a real email; run explicitly with --features live-tests -- --ignored"]
fn mailgun_live() {
    let transport = Arc::new(UreqTransport::new());
    let endpoint = std::env::var("AEGIS_MAILGUN_ENDPOINT")
        .unwrap_or_else(|_| "https://api.mailgun.net/v3".into());
    let provider = Mailgun::new(
        HttpProviderConfig::new(endpoint, required("AEGIS_MAILGUN_API_KEY")).unwrap(),
        required("AEGIS_MAILGUN_DOMAIN"),
        transport,
    )
    .unwrap();
    let email = message(
        required("AEGIS_MAILGUN_FROM"),
        required("AEGIS_LIVE_EMAIL_TO"),
    );
    let request = to_delivery_request(&email, &MessageId::new("live-mailgun").unwrap()).unwrap();
    provider
        .send(&request)
        .expect("Mailgun accepted live email");
}

#[test]
#[ignore = "requires a running local Mailpit; run explicitly with --features live-tests -- --ignored"]
fn mailpit_live() {
    let endpoint =
        std::env::var("AEGIS_MAILPIT_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:8025".into());
    let provider = Mailpit::new(endpoint, Arc::new(UreqTransport::new())).unwrap();
    let from = std::env::var("AEGIS_MAILPIT_FROM").unwrap_or_else(|_| "aegis@example.test".into());
    let email = message(from, required("AEGIS_LIVE_EMAIL_TO"));
    let request = to_delivery_request(&email, &MessageId::new("live-mailpit").unwrap()).unwrap();
    provider
        .send(&request)
        .expect("Mailpit accepted live email");
}
