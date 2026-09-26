//! Typed SMS composition and provider adapters backed by
//! [`aegis_mesg_sender_core`].

#![deny(unsafe_code)]
#![warn(missing_docs)]

use aegis_mesg_sender_core::{
    Channel, DeliveryRequest, ErrorKind, Message, MessageId, Provider, ProviderError,
    ProviderResponse, Recipient,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// A validated E.164 phone number.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PhoneNumber(String);

impl PhoneNumber {
    /// Parses `+` followed by 8–15 digits.
    pub fn new(value: impl Into<String>) -> Result<Self, SmsError> {
        let value = value.into();
        if value.starts_with('+')
            && (8..=15).contains(&(value.len() - 1))
            && value[1..].chars().all(|c| c.is_ascii_digit())
        {
            Ok(Self(value))
        } else {
            Err(SmsError::InvalidPhoneNumber)
        }
    }

    /// Returns the number for provider serialization.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PhoneNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tail: String = self.0.chars().rev().take(3).collect();
        write!(
            formatter,
            "PhoneNumber(***{})",
            tail.chars().rev().collect::<String>()
        )
    }
}

/// Sender identity used by an SMS provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Sender {
    /// An E.164 originating number.
    Phone(PhoneNumber),
    /// An alphanumeric sender id, limited to eleven ASCII characters.
    Alphanumeric(String),
}

impl Sender {
    /// Validates a sender identity.
    pub fn validate(&self) -> Result<(), SmsError> {
        match self {
            Self::Phone(number) => PhoneNumber::new(number.as_str().to_owned()).map(|_| ()),
            Self::Alphanumeric(value)
                if !value.is_empty()
                    && value.len() <= 11
                    && value.chars().all(|c| c.is_ascii_alphanumeric()) =>
            {
                Ok(())
            }
            Self::Alphanumeric(_) => Err(SmsError::InvalidSender),
        }
    }

    fn value(&self) -> &str {
        match self {
            Self::Phone(number) => number.as_str(),
            Self::Alphanumeric(value) => value,
        }
    }
}

/// Message encoding selected by SMS length rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Encoding {
    /// GSM-7 compatible text.
    Gsm7,
    /// Unicode text.
    Unicode,
}

/// A validated outbound SMS.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SmsMessage {
    /// Destination number.
    pub to: PhoneNumber,
    /// Originating identity.
    pub from: Sender,
    /// Message body.
    pub body: String,
    /// Non-sensitive provider/application metadata.
    pub metadata: BTreeMap<String, String>,
}

impl SmsMessage {
    /// Creates and validates an SMS.
    pub fn new(to: PhoneNumber, from: Sender, body: impl Into<String>) -> Result<Self, SmsError> {
        let message = Self {
            to,
            from,
            body: body.into(),
            metadata: BTreeMap::new(),
        };
        message.validate()?;
        Ok(message)
    }

    /// Adds metadata without changing validation semantics.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Validates sender, body, and the ten-segment limit.
    pub fn validate(&self) -> Result<(), SmsError> {
        self.from.validate()?;
        if self.body.is_empty() {
            return Err(SmsError::EmptyBody);
        }
        if self.body.chars().count() > 10 * self.single_segment_limit() {
            return Err(SmsError::BodyTooLong);
        }
        Ok(())
    }

    /// Returns GSM-7 or Unicode encoding.
    pub fn encoding(&self) -> Encoding {
        if self.body.chars().all(is_gsm7) {
            Encoding::Gsm7
        } else {
            Encoding::Unicode
        }
    }

    /// Returns the number of SMS segments required.
    pub fn segments(&self) -> usize {
        let limit = self.single_segment_limit();
        self.body.chars().count().div_ceil(limit).max(1)
    }

    fn single_segment_limit(&self) -> usize {
        if self.encoding() == Encoding::Gsm7 {
            160
        } else {
            70
        }
    }

    fn encode(&self, id: &MessageId) -> Result<DeliveryRequest, SmsError> {
        self.validate()?;
        let body = serde_json::to_vec(self).map_err(|_| SmsError::Serialization)?;
        let recipient = Recipient::new(self.to.as_str()).map_err(|_| SmsError::Serialization)?;
        let message =
            Message::new(id.clone(), recipient, body).map_err(|_| SmsError::Serialization)?;
        Ok(DeliveryRequest {
            channel: Channel::Sms,
            message,
        })
    }
}

fn is_gsm7(character: char) -> bool {
    character.is_ascii()
        && (character.is_ascii_alphanumeric()
            || " !\"#%&'()*+,-./:;<=>?@£$¥ÄÖÑÜ§¿äöñüà^{}\\[~]|€\n\r".contains(character))
}

/// SMS validation failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmsError {
    /// Invalid E.164 number.
    InvalidPhoneNumber,
    /// Invalid sender identity.
    InvalidSender,
    /// Empty body.
    EmptyBody,
    /// Body exceeds ten segments.
    BodyTooLong,
    /// Payload serialization failed.
    Serialization,
    /// Provider response was malformed.
    InvalidResponse,
}

impl fmt::Display for SmsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for SmsError {}

/// Builds a channel-neutral core request from an SMS.
pub fn to_delivery_request(
    message: &SmsMessage,
    id: &MessageId,
) -> Result<DeliveryRequest, SmsError> {
    message.encode(id)
}

/// HTTP request passed to an SMS adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: String,
    /// Target URL.
    pub url: String,
    /// Headers.
    pub headers: BTreeMap<String, String>,
    /// Request body.
    pub body: Vec<u8>,
}

/// HTTP response returned by an injected transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Response body.
    pub body: Vec<u8>,
}

/// Transport error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportError(pub String);

/// Runtime-independent HTTP transport.
pub trait HttpTransport: Send + Sync {
    /// Sends a request.
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Provider adapter contract.
pub trait SmsProvider: Provider {
    /// Provider instance name.
    fn provider_name(&self) -> &str;
}

fn provider_response(
    result: Result<HttpResponse, TransportError>,
    provider: &str,
) -> Result<ProviderResponse, ProviderError> {
    let response = result.map_err(|error| {
        ProviderError::from_kind(
            ErrorKind::Unavailable,
            format!("{provider} transport: {}", error.0),
        )
    })?;
    if !(200..300).contains(&response.status) {
        return Err(ProviderError::new(
            ErrorKind::Unavailable,
            format!("{provider} returned HTTP {}", response.status),
            response.status == 429 || response.status >= 500,
            true,
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&response.body).map_err(|_| {
        ProviderError::from_kind(
            ErrorKind::Internal,
            format!("{provider} returned invalid JSON"),
        )
    })?;
    let id = value
        .get("message_id")
        .or_else(|| value.get("messageId"))
        .or_else(|| value.get("id"))
        .or_else(|| value.get("sid"))
        .or_else(|| value.pointer("/data/id"))
        .or_else(|| value.pointer("/data/smsBatchId"))
        .or_else(|| value.pointer("/messages/0/messageId"))
        .or_else(|| value.pointer("/messages/0/message_id"))
        .or_else(|| value.pointer("/0/message_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ProviderError::from_kind(
                ErrorKind::Internal,
                format!("{provider} response omitted message id"),
            )
        })?;
    Ok(ProviderResponse {
        provider_message_id: id.to_owned(),
    })
}

fn json_request(
    url: &str,
    body: serde_json::Value,
    headers: BTreeMap<String, String>,
) -> HttpRequest {
    HttpRequest {
        method: "POST".into(),
        url: url.into(),
        headers,
        body: serde_json::to_vec(&body).expect("SMS JSON is serializable"),
    }
}

fn form_request(
    url: &str,
    fields: &[(&str, &str)],
    mut headers: BTreeMap<String, String>,
) -> HttpRequest {
    headers.insert(
        "content-type".into(),
        "application/x-www-form-urlencoded".into(),
    );
    HttpRequest {
        method: "POST".into(),
        url: url.into(),
        headers,
        body: fields
            .iter()
            .map(|(key, value)| format!("{}={}", form_encode(key), form_encode(value)))
            .collect::<Vec<_>>()
            .join("&")
            .into_bytes(),
    }
}

fn form_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

/// Twilio configuration and adapter.
pub struct Twilio<T> {
    /// Injected transport.
    pub transport: Arc<T>,
    /// API endpoint.
    pub endpoint: String,
    /// Account SID.
    pub account_sid: String,
    /// Auth token.
    pub auth_token: String,
}

impl<T: HttpTransport> Provider for Twilio<T> {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let sms: SmsMessage = serde_json::from_slice(&request.message.body)
            .map_err(|_| ProviderError::invalid_request("invalid SMS payload"))?;
        let mut headers = BTreeMap::new();
        headers.insert(
            "authorization".into(),
            format!(
                "Basic {}",
                base64(&format!("{}:{}", self.account_sid, self.auth_token))
            ),
        );
        provider_response(
            self.transport.send(form_request(
                &self.endpoint,
                &[
                    ("To", sms.to.as_str()),
                    ("From", sms.from.value()),
                    ("Body", &sms.body),
                ],
                headers,
            )),
            "Twilio",
        )
    }
}
impl<T: HttpTransport> SmsProvider for Twilio<T> {
    fn provider_name(&self) -> &str {
        "twilio"
    }
}

/// TextBee configuration and adapter.
pub struct TextBee<T> {
    /// Injected transport.
    pub transport: Arc<T>,
    /// API endpoint.
    pub endpoint: String,
    /// API key.
    pub api_key: String,
    /// Device identifier.
    pub device_id: String,
}

impl<T: HttpTransport> Provider for TextBee<T> {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let sms: SmsMessage = serde_json::from_slice(&request.message.body)
            .map_err(|_| ProviderError::invalid_request("invalid SMS payload"))?;
        let mut headers = BTreeMap::new();
        headers.insert("x-api-key".into(), self.api_key.clone());
        let mut body = serde_json::json!({
            "recipients": [sms.to.as_str()],
            "message": sms.body
        });
        if !self.device_id.trim().is_empty() {
            body["deviceId"] = serde_json::Value::String(self.device_id.clone());
        }
        provider_response(
            self.transport
                .send(json_request(&self.endpoint, body, headers)),
            "TextBee",
        )
    }
}
impl<T: HttpTransport> SmsProvider for TextBee<T> {
    fn provider_name(&self) -> &str {
        "textbee"
    }
}

/// Semaphore configuration and adapter.
pub struct Semaphore<T> {
    /// Injected transport.
    pub transport: Arc<T>,
    /// API endpoint.
    pub endpoint: String,
    /// API key.
    pub api_key: String,
}

impl<T: HttpTransport> Provider for Semaphore<T> {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let sms: SmsMessage = serde_json::from_slice(&request.message.body)
            .map_err(|_| ProviderError::invalid_request("invalid SMS payload"))?;
        provider_response(
            self.transport.send(form_request(
                &self.endpoint,
                &[
                    ("apikey", &self.api_key),
                    ("number", sms.to.as_str()),
                    ("message", &sms.body),
                    ("sendername", sms.from.value()),
                ],
                BTreeMap::new(),
            )),
            "Semaphore",
        )
    }
}
impl<T: HttpTransport> SmsProvider for Semaphore<T> {
    fn provider_name(&self) -> &str {
        "semaphore"
    }
}

/// Infobip configuration and adapter.
pub struct Infobip<T> {
    /// Injected transport.
    pub transport: Arc<T>,
    /// API endpoint.
    pub endpoint: String,
    /// API key.
    pub api_key: String,
}

impl<T: HttpTransport> Provider for Infobip<T> {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let sms: SmsMessage = serde_json::from_slice(&request.message.body)
            .map_err(|_| ProviderError::invalid_request("invalid SMS payload"))?;
        let mut headers = BTreeMap::new();
        headers.insert("authorization".into(), format!("App {}", self.api_key));
        headers.insert("content-type".into(), "application/json".into());
        provider_response(
            self.transport.send(json_request(
                &self.endpoint,
                serde_json::json!({"messages":[{"destinations":[{"to": sms.to.as_str()}], "from": sms.from.value(), "text": sms.body}]}),
                headers,
            )),
            "Infobip",
        )
    }
}
impl<T: HttpTransport> SmsProvider for Infobip<T> {
    fn provider_name(&self) -> &str {
        "infobip"
    }
}

fn base64(value: &str) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.as_bytes();
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let number = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(TABLE[((number >> 18) & 63) as usize] as char);
        output.push(TABLE[((number >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((number >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(number & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

/// Normalized SMS receipt state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmsReceiptStatus {
    /// Handset accepted the message.
    Delivered,
    /// Provider permanently failed delivery.
    Failed,
}

/// A normalized SMS provider receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmsDeliveryReceipt {
    /// Provider message id.
    pub provider_message_id: String,
    /// Normalized state.
    pub status: SmsReceiptStatus,
}

/// Converts common provider status names to a normalized receipt.
pub fn normalize_receipt(
    provider_message_id: impl Into<String>,
    status: &str,
) -> Result<SmsDeliveryReceipt, SmsError> {
    let status = match status.to_ascii_lowercase().as_str() {
        "delivered" | "sent" | "success" => SmsReceiptStatus::Delivered,
        "failed" | "undelivered" | "rejected" => SmsReceiptStatus::Failed,
        _ => return Err(SmsError::InvalidResponse),
    };
    Ok(SmsDeliveryReceipt {
        provider_message_id: provider_message_id.into(),
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTransport(Mutex<Vec<HttpRequest>>);
    impl HttpTransport for FakeTransport {
        fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
            self.0.lock().unwrap().push(request);
            Ok(HttpResponse {
                status: 200,
                body: br#"{"message_id":"sms-1"}"#.to_vec(),
            })
        }
    }

    fn message() -> SmsMessage {
        SmsMessage::new(
            PhoneNumber::new("+639171234567").unwrap(),
            Sender::Alphanumeric("AEGIS".into()),
            "hello",
        )
        .unwrap()
    }

    #[test]
    fn validates_e164_and_sms_lengths() {
        assert!(PhoneNumber::new("09171234567").is_err());
        assert!(PhoneNumber::new("+639171234567").is_ok());
        let sms = message();
        assert_eq!(sms.encoding(), Encoding::Gsm7);
        assert_eq!(sms.segments(), 1);
        assert_eq!(
            SmsMessage::new(sms.to, sms.from, "🙂".repeat(701)).unwrap_err(),
            SmsError::BodyTooLong
        );
    }

    #[test]
    fn twilio_request_is_encoded_and_response_normalized() {
        let transport = Arc::new(FakeTransport::default());
        let provider = Twilio {
            transport: transport.clone(),
            endpoint: "https://twilio.test/messages".into(),
            account_sid: "sid".into(),
            auth_token: "secret".into(),
        };
        let request = to_delivery_request(&message(), &MessageId::new("m-1").unwrap()).unwrap();
        assert_eq!(
            provider.send(&request).unwrap().provider_message_id,
            "sms-1"
        );
        let sent = &transport.0.lock().unwrap()[0];
        assert!(String::from_utf8_lossy(&sent.body).contains("Body=hello"));
        assert!(!format!("{sent:?}").contains("secret"));
    }

    #[test]
    fn normalizes_receipts() {
        assert_eq!(
            normalize_receipt("id", "DELIVERED").unwrap().status,
            SmsReceiptStatus::Delivered
        );
        assert_eq!(
            normalize_receipt("id", "undelivered").unwrap().status,
            SmsReceiptStatus::Failed
        );
        assert!(normalize_receipt("id", "queued").is_err());
    }

    #[test]
    fn provider_response_fixtures_cover_provider_id_shapes() {
        for (fixture, expected) in [
            (
                include_str!("../fixtures/twilio/success.json"),
                "SMfixture0001",
            ),
            (
                include_str!("../fixtures/textbee/success.json"),
                "textbee-fixture-0001",
            ),
            (
                include_str!("../fixtures/semaphore/success.json"),
                "semaphore-fixture-0001",
            ),
            (
                include_str!("../fixtures/infobip/success.json"),
                "infobip-fixture-0001",
            ),
        ] {
            let result = provider_response(
                Ok(HttpResponse {
                    status: 200,
                    body: fixture.as_bytes().to_vec(),
                }),
                "fixture",
            )
            .unwrap();
            assert_eq!(result.provider_message_id, expected);
        }
    }

    #[test]
    fn form_values_are_percent_encoded() {
        let request = form_request(
            "https://provider.test",
            &[("message", "hello world&yes")],
            BTreeMap::new(),
        );
        assert_eq!(
            String::from_utf8(request.body).unwrap(),
            "message=hello%20world%26yes"
        );
    }
}
