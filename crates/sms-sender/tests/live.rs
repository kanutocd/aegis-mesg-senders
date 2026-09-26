//! Credential-gated live provider tests.

#![cfg(feature = "live-tests")]

use aegis_mesg_sender_core::{MessageId, Provider};
use aegis_sms_sender::{
    to_delivery_request, HttpRequest, HttpResponse, HttpTransport, Infobip, PhoneNumber, Sender,
    SmsMessage, TextBee, TransportError,
};
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
                    .map_err(|_| TransportError("response read failed".into()))?;
                Ok(HttpResponse { status, body })
            }
            Err(ureq::Error::Status(status, response)) => {
                let mut body = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut body)
                    .map_err(|_| TransportError("error response read failed".into()))?;
                Ok(HttpResponse { status, body })
            }
            Err(_) => Err(TransportError("provider request failed".into())),
        }
    }
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} before running live tests"))
}

fn sms() -> SmsMessage {
    let sender = std::env::var("AEGIS_SMS_FROM").unwrap_or_else(|_| "AEGIS".into());
    let sender = if sender.starts_with('+') {
        Sender::Phone(PhoneNumber::new(sender).unwrap())
    } else {
        Sender::Alphanumeric(sender)
    };
    SmsMessage::new(
        PhoneNumber::new(required("AEGIS_SMS_TO")).unwrap(),
        sender,
        "Aegis live SMS integration test",
    )
    .unwrap()
}

#[test]
#[ignore = "sends a real SMS; run explicitly with --features live-tests -- --ignored"]
fn textbee_live() {
    let endpoint = std::env::var("AEGIS_TEXTBEE_ENDPOINT")
        .unwrap_or_else(|_| "https://api.textbee.dev/api/v1/gateway/send-sms".into());
    let provider = TextBee {
        transport: Arc::new(UreqTransport::new()),
        endpoint,
        api_key: required("AEGIS_TEXTBEE_API_KEY"),
        device_id: std::env::var("AEGIS_TEXTBEE_DEVICE_ID").unwrap_or_default(),
    };
    let request = to_delivery_request(&sms(), &MessageId::new("live-textbee").unwrap()).unwrap();
    provider.send(&request).unwrap_or_else(|error| {
        panic!(
            "TextBee rejected live SMS: kind={:?}, message={}",
            error.kind, error.message
        )
    });
}

#[test]
#[ignore = "sends a real SMS; run explicitly with --features live-tests -- --ignored"]
fn infobip_live() {
    let endpoint = std::env::var("AEGIS_INFOBIP_ENDPOINT").unwrap_or_else(|_| {
        format!(
            "{}/sms/3/messages",
            std::env::var("AEGIS_INFOBIP_BASE_URL")
                .unwrap_or_else(|_| "https://api.infobip.com".into())
                .trim_end_matches('/')
        )
    });
    let provider = Infobip {
        transport: Arc::new(UreqTransport::new()),
        endpoint,
        api_key: required("AEGIS_INFOBIP_API_KEY"),
    };
    let request = to_delivery_request(&sms(), &MessageId::new("live-infobip").unwrap()).unwrap();
    provider.send(&request).expect("Infobip accepted live SMS");
}
