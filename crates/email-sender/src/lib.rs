//! Typed email composition and provider adapters backed by `aegis-mesg-sender-core`.
//!
//! Provider integrations use [`HttpTransport`] rather than a concrete HTTP
//! client, making protocol behavior deterministic in tests and leaving the
//! choice of runtime to the application.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use aegis_mesg_sender_core::{
    Channel, DeliveryRequest, ErrorKind, Message, MessageId, Provider, ProviderError,
    ProviderResponse, Recipient, ValidationError,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// A validated email address whose debug representation is redacted.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// Parses an address using the deliberately conservative syntax supported
    /// by the facade.
    pub fn new(value: impl Into<String>) -> Result<Self, EmailError> {
        let value = value.into();
        let valid = !value.is_empty()
            && !value.chars().any(|character| character.is_whitespace())
            && value.matches('@').count() == 1
            && value.split('@').next().is_some_and(|part| !part.is_empty())
            && value.split('@').nth(1).is_some_and(|part| {
                !part.is_empty()
                    && part.contains('.')
                    && !part.starts_with('.')
                    && !part.ends_with('.')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(EmailError::InvalidAddress)
        }
    }

    /// Returns the address for protocol serialization.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a stable redacted representation for diagnostics.
    pub fn redacted(&self) -> String {
        let domain = self.0.split('@').nth(1).unwrap_or("redacted");
        format!("***@{domain}")
    }
}

impl fmt::Debug for EmailAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("EmailAddress")
            .field(&self.redacted())
            .finish()
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// An email attachment.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    /// File name presented to the provider.
    pub filename: String,
    /// MIME content type.
    pub content_type: String,
    /// File bytes.
    pub data: Vec<u8>,
}

impl fmt::Debug for Attachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Attachment")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("bytes", &self.data.len())
            .finish()
    }
}

/// A validated email message.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EmailMessage {
    /// Sender address.
    pub from: String,
    /// Primary recipients.
    pub to: Vec<String>,
    /// Carbon-copy recipients.
    pub cc: Vec<String>,
    /// Blind-carbon-copy recipients.
    pub bcc: Vec<String>,
    /// Reply-to address, when configured.
    pub reply_to: Option<String>,
    /// Subject line.
    pub subject: String,
    /// Plain-text body.
    pub text: Option<String>,
    /// HTML body.
    pub html: Option<String>,
    /// Additional provider-neutral headers.
    pub headers: BTreeMap<String, String>,
    /// Attachments.
    pub attachments: Vec<Attachment>,
    /// Non-sensitive application metadata.
    pub metadata: BTreeMap<String, String>,
}

impl fmt::Debug for EmailMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmailMessage")
            .field("from", &"[redacted]")
            .field("recipients", &self.to.len())
            .field("cc", &self.cc.len())
            .field("bcc", &self.bcc.len())
            .field("subject", &self.subject)
            .field("has_text", &self.text.is_some())
            .field("has_html", &self.html.is_some())
            .field("attachments", &self.attachments.len())
            .finish()
    }
}

/// Builder for [`EmailMessage`].
pub struct EmailBuilder {
    message: EmailMessage,
}

impl EmailBuilder {
    /// Starts a message from a sender address.
    pub fn new(from: impl Into<String>) -> Self {
        Self {
            message: EmailMessage {
                from: from.into(),
                to: Vec::new(),
                cc: Vec::new(),
                bcc: Vec::new(),
                reply_to: None,
                subject: String::new(),
                text: None,
                html: None,
                headers: BTreeMap::new(),
                attachments: Vec::new(),
                metadata: BTreeMap::new(),
            },
        }
    }
    /// Adds a primary recipient.
    pub fn to(mut self, address: impl Into<String>) -> Self {
        self.message.to.push(address.into());
        self
    }
    /// Adds a carbon-copy recipient.
    pub fn cc(mut self, address: impl Into<String>) -> Self {
        self.message.cc.push(address.into());
        self
    }
    /// Adds a blind-carbon-copy recipient.
    pub fn bcc(mut self, address: impl Into<String>) -> Self {
        self.message.bcc.push(address.into());
        self
    }
    /// Sets the reply-to address.
    pub fn reply_to(mut self, address: impl Into<String>) -> Self {
        self.message.reply_to = Some(address.into());
        self
    }
    /// Sets the subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.message.subject = subject.into();
        self
    }
    /// Sets the plain-text body.
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.message.text = Some(body.into());
        self
    }
    /// Sets the HTML body.
    pub fn html(mut self, body: impl Into<String>) -> Self {
        self.message.html = Some(body.into());
        self
    }
    /// Adds a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.message.headers.insert(name.into(), value.into());
        self
    }
    /// Adds an attachment.
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.message.attachments.push(attachment);
        self
    }
    /// Adds metadata.
    pub fn metadata(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.message.metadata.insert(name.into(), value.into());
        self
    }
    /// Sets the provider idempotency key used by adapters that support it.
    pub fn idempotency_key(self, key: impl Into<String>) -> Self {
        self.metadata("idempotency-key", key)
    }
    /// Validates and finishes the message.
    pub fn build(self) -> Result<EmailMessage, EmailError> {
        self.message.validate()?;
        Ok(self.message)
    }
}

impl EmailMessage {
    /// Starts building a message.
    pub fn builder(from: impl Into<String>) -> EmailBuilder {
        EmailBuilder::new(from)
    }

    /// Validates addresses and required message content.
    pub fn validate(&self) -> Result<(), EmailError> {
        EmailAddress::new(&self.from)?;
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(EmailError::NoRecipients);
        }
        for value in self.to.iter().chain(self.cc.iter()).chain(self.bcc.iter()) {
            EmailAddress::new(value)?;
        }
        if let Some(reply_to) = &self.reply_to {
            EmailAddress::new(reply_to)?;
        }
        if self.subject.trim().is_empty() {
            return Err(EmailError::Empty("subject"));
        }
        if self.text.as_deref().unwrap_or_default().is_empty()
            && self.html.as_deref().unwrap_or_default().is_empty()
        {
            return Err(EmailError::NoBody);
        }
        Ok(())
    }

    fn encode(&self, id: &MessageId) -> Result<DeliveryRequest, EmailError> {
        self.validate()?;
        let body = serde_json::to_vec(self).map_err(|_| EmailError::Serialization)?;
        let recipient = self
            .to
            .first()
            .or(self.cc.first())
            .or(self.bcc.first())
            .ok_or(EmailError::NoRecipients)?;
        let message = Message::new(
            id.clone(),
            Recipient::new(recipient).map_err(|_| EmailError::InvalidAddress)?,
            body,
        )
        .map_err(|_| EmailError::Serialization)?;
        Ok(DeliveryRequest {
            channel: Channel::Email,
            message,
        })
    }
}

/// Errors raised while composing or validating email.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmailError {
    /// An address was malformed.
    InvalidAddress,
    /// No recipient was supplied.
    NoRecipients,
    /// A required string was empty.
    Empty(&'static str),
    /// No text or HTML body was supplied.
    NoBody,
    /// Message serialization failed.
    Serialization,
}

impl fmt::Display for EmailError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for EmailError {}

/// A request sent through an injected HTTP implementation.
#[derive(Clone, Eq, PartialEq)]
pub struct HttpRequest {
    /// HTTP method.
    pub method: String,
    /// Absolute endpoint URL.
    pub url: String,
    /// Request headers.
    pub headers: BTreeMap<String, String>,
    /// Request body.
    pub body: Vec<u8>,
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

/// A response returned by an injected HTTP implementation.
#[derive(Clone, Eq, PartialEq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: BTreeMap<String, String>,
    /// Response body.
    pub body: Vec<u8>,
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

/// A transport failure safe to expose in provider diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportError {
    /// Safe failure detail.
    pub message: String,
    /// Whether a retry may succeed.
    pub retryable: bool,
}

/// Injectable synchronous HTTP transport.
pub trait HttpTransport: Send + Sync {
    /// Executes one request.
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Provider configuration shared by HTTP adapters.
#[derive(Clone, Eq, PartialEq)]
pub struct HttpProviderConfig {
    /// API endpoint.
    pub endpoint: String,
    /// Secret API key.
    pub api_key: String,
}

impl fmt::Debug for HttpProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpProviderConfig")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"[redacted]")
            .finish()
    }
}

impl HttpProviderConfig {
    /// Creates and validates HTTP provider configuration.
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, EmailError> {
        let endpoint = endpoint.into();
        let api_key = api_key.into();
        if endpoint.trim().is_empty() {
            return Err(EmailError::Empty("endpoint"));
        }
        if api_key.trim().is_empty() {
            return Err(EmailError::Empty("api key"));
        }
        Ok(Self { endpoint, api_key })
    }
}

fn provider_error(status: u16, _body: &[u8]) -> ProviderError {
    let kind = match status {
        401 | 403 => ErrorKind::Authentication,
        408 | 429 => {
            if status == 429 {
                ErrorKind::RateLimited
            } else {
                ErrorKind::Timeout
            }
        }
        400..=499 => ErrorKind::InvalidRequest,
        500..=599 => ErrorKind::Unavailable,
        _ => ErrorKind::Unknown,
    };
    let retryable = matches!(
        kind,
        ErrorKind::RateLimited | ErrorKind::Timeout | ErrorKind::Unavailable
    );
    ProviderError::new(
        kind,
        "provider rejected request",
        retryable,
        !matches!(kind, ErrorKind::Authentication | ErrorKind::InvalidRequest),
    )
}

fn form_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{:02X}", byte));
        }
    }
    encoded
}

fn base64_encode(value: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(value.len().saturating_add(2) / 3 * 4);
    for chunk in value.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[((first & 0b11) << 4 | second >> 4) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[((second & 0b1111) << 2 | third >> 6) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(third & 0b111111) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn multipart_escape(value: &str) -> Result<String, ProviderError> {
    if value.contains(['\r', '\n', '"']) {
        return Err(ProviderError::invalid_request(
            "multipart metadata contains an invalid header character",
        ));
    }
    Ok(value.to_owned())
}

fn multipart_body(
    fields: &BTreeMap<String, String>,
    attachments: &[Attachment],
    boundary: &str,
) -> Result<Vec<u8>, ProviderError> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                multipart_escape(name)?,
                value
            )
            .as_bytes(),
        );
    }
    for attachment in attachments {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"attachment\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                multipart_escape(&attachment.filename)?,
                multipart_escape(&attachment.content_type)?
            )
            .as_bytes(),
        );
        body.extend_from_slice(&attachment.data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok(body)
}

fn submit(
    transport: &dyn HttpTransport,
    request: HttpRequest,
) -> Result<ProviderResponse, ProviderError> {
    let response = transport.send(request).map_err(|error| {
        ProviderError::new(
            if error.retryable {
                ErrorKind::Unavailable
            } else {
                ErrorKind::Internal
            },
            error.message,
            error.retryable,
            error.retryable,
        )
    })?;
    if !(200..300).contains(&response.status) {
        return Err(provider_error(response.status, &response.body));
    }
    let value: serde_json::Value = serde_json::from_slice(&response.body).map_err(|_| {
        ProviderError::new(
            ErrorKind::Internal,
            "provider returned invalid response",
            false,
            false,
        )
    })?;
    let id = value
        .get("id")
        .or_else(|| value.get("message_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ProviderError::new(
                ErrorKind::Internal,
                "provider response omitted message id",
                false,
                false,
            )
        })?;
    Ok(ProviderResponse {
        provider_message_id: id.to_owned(),
    })
}

fn message(request: &DeliveryRequest) -> Result<EmailMessage, ProviderError> {
    serde_json::from_slice(&request.message.body)
        .map_err(|_| ProviderError::invalid_request("invalid email message"))
}

/// A provider adapter that can be registered directly with `aegis-mesg-sender-core`.
pub trait EmailProvider: Provider {
    /// Returns the provider name used in diagnostics.
    fn provider_name(&self) -> &'static str;
}

#[cfg(feature = "resend")]
/// Resend HTTP adapter.
pub struct Resend {
    config: HttpProviderConfig,
    transport: Arc<dyn HttpTransport>,
}
#[cfg(feature = "resend")]
impl Resend {
    /// Creates a Resend adapter.
    pub fn new(config: HttpProviderConfig, transport: Arc<dyn HttpTransport>) -> Self {
        Self { config, transport }
    }
}
#[cfg(feature = "resend")]
impl Provider for Resend {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let email = message(request)?;
        let attachments = email
            .attachments
            .iter()
            .map(|attachment| {
                serde_json::json!({
                    "filename": attachment.filename,
                    "content": base64_encode(&attachment.data),
                    "content_type": attachment.content_type
                })
            })
            .collect::<Vec<_>>();
        let body = serde_json::json!({ "from": email.from, "to": email.to, "cc": email.cc, "bcc": email.bcc, "reply_to": email.reply_to, "subject": email.subject, "text": email.text, "html": email.html, "headers": email.headers, "attachments": attachments });
        let mut headers = BTreeMap::from([
            (
                String::from("authorization"),
                format!("Bearer {}", self.config.api_key),
            ),
            (
                String::from("content-type"),
                String::from("application/json"),
            ),
        ]);
        if let Some(key) = email.metadata.get("idempotency-key") {
            headers.insert(String::from("idempotency-key"), key.clone());
        }
        submit(
            self.transport.as_ref(),
            HttpRequest {
                method: "POST".into(),
                url: format!("{}/emails", self.config.endpoint.trim_end_matches('/')),
                headers,
                body: serde_json::to_vec(&body).map_err(|_| {
                    ProviderError::new(
                        ErrorKind::Internal,
                        "request serialization failed",
                        false,
                        false,
                    )
                })?,
            },
        )
    }
}
#[cfg(feature = "resend")]
impl EmailProvider for Resend {
    fn provider_name(&self) -> &'static str {
        "resend"
    }
}

#[cfg(feature = "mailgun")]
/// Mailgun HTTP adapter.
pub struct Mailgun {
    config: HttpProviderConfig,
    domain: String,
    sandbox: bool,
    transport: Arc<dyn HttpTransport>,
}
#[cfg(feature = "mailgun")]
impl Mailgun {
    /// Creates a Mailgun adapter for a sending domain.
    pub fn new(
        config: HttpProviderConfig,
        domain: impl Into<String>,
        transport: Arc<dyn HttpTransport>,
    ) -> Result<Self, EmailError> {
        let domain = domain.into();
        if domain.trim().is_empty() {
            return Err(EmailError::Empty("domain"));
        }
        Ok(Self {
            config,
            domain,
            sandbox: false,
            transport,
        })
    }

    /// Enables or disables Mailgun test mode for sandbox domains.
    pub fn with_sandbox(mut self, sandbox: bool) -> Self {
        self.sandbox = sandbox;
        self
    }
}
#[cfg(feature = "mailgun")]
impl Provider for Mailgun {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let email = message(request)?;
        let mut form = BTreeMap::new();
        form.insert(String::from("from"), email.from);
        form.insert(String::from("to"), email.to.join(","));
        if !email.cc.is_empty() {
            form.insert(String::from("cc"), email.cc.join(","));
        }
        if !email.bcc.is_empty() {
            form.insert(String::from("bcc"), email.bcc.join(","));
        }
        form.insert(String::from("subject"), email.subject);
        if let Some(text) = email.text {
            form.insert(String::from("text"), text);
        }
        if let Some(html) = email.html {
            form.insert(String::from("html"), html);
        }
        if let Some(reply_to) = email.reply_to {
            form.insert(String::from("h:Reply-To"), reply_to);
        }
        for (name, value) in email.headers {
            form.insert(format!("h:{name}"), value);
        }
        if self.sandbox {
            form.insert(String::from("o:testmode"), String::from("yes"));
        }
        let (body, content_type) = if email.attachments.is_empty() {
            (
                form.into_iter()
                    .map(|(key, value)| format!("{}={}", form_encode(&key), form_encode(&value)))
                    .collect::<Vec<_>>()
                    .join("&")
                    .into_bytes(),
                String::from("application/x-www-form-urlencoded"),
            )
        } else {
            let boundary = format!("aegis-{}", request.message.id.as_str());
            (
                multipart_body(&form, &email.attachments, &boundary)?,
                format!("multipart/form-data; boundary={boundary}"),
            )
        };
        submit(
            self.transport.as_ref(),
            HttpRequest {
                method: "POST".into(),
                url: format!(
                    "{}/{}/messages",
                    self.config.endpoint.trim_end_matches('/'),
                    self.domain
                ),
                headers: BTreeMap::from([
                    (
                        String::from("authorization"),
                        format!(
                            "Basic {}",
                            base64_encode(format!("api:{}", self.config.api_key).as_bytes())
                        ),
                    ),
                    (String::from("content-type"), content_type),
                ]),
                body,
            },
        )
    }
}
#[cfg(feature = "mailgun")]
impl EmailProvider for Mailgun {
    fn provider_name(&self) -> &'static str {
        "mailgun"
    }
}

#[cfg(feature = "mailpit")]
/// Mailpit local-development HTTP adapter.
pub struct Mailpit {
    endpoint: String,
    transport: Arc<dyn HttpTransport>,
}
#[cfg(feature = "mailpit")]
impl Mailpit {
    /// Creates a Mailpit adapter.
    pub fn new(
        endpoint: impl Into<String>,
        transport: Arc<dyn HttpTransport>,
    ) -> Result<Self, EmailError> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err(EmailError::Empty("endpoint"));
        }
        Ok(Self {
            endpoint,
            transport,
        })
    }
}
#[cfg(feature = "mailpit")]
impl Provider for Mailpit {
    fn send(&self, request: &DeliveryRequest) -> Result<ProviderResponse, ProviderError> {
        let email = message(request)?;
        let body = serde_json::json!({ "From": { "Email": email.from }, "To": email.to.into_iter().map(|value| serde_json::json!({ "Email": value })).collect::<Vec<_>>(), "Subject": email.subject, "Text": email.text, "HTML": email.html });
        submit(
            self.transport.as_ref(),
            HttpRequest {
                method: "POST".into(),
                url: format!("{}/api/v1/send", self.endpoint.trim_end_matches('/')),
                headers: BTreeMap::from([(
                    String::from("content-type"),
                    String::from("application/json"),
                )]),
                body: serde_json::to_vec(&body).map_err(|_| {
                    ProviderError::new(
                        ErrorKind::Internal,
                        "request serialization failed",
                        false,
                        false,
                    )
                })?,
            },
        )
    }
}
#[cfg(feature = "mailpit")]
impl EmailProvider for Mailpit {
    fn provider_name(&self) -> &'static str {
        "mailpit"
    }
}

/// A provider webhook receipt normalized independently from submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryReceipt {
    /// Message identifier.
    pub message_id: MessageId,
    /// Provider message identifier.
    pub provider_message_id: String,
    /// Normalized receipt state.
    pub status: ReceiptStatus,
}

/// Normalized provider receipt state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptStatus {
    /// Provider accepted the message.
    Accepted,
    /// Mailbox delivery succeeded.
    Delivered,
    /// Mailbox delivery failed.
    Failed,
}

/// Converts a provider event into a normalized receipt.
pub fn normalize_receipt(
    message_id: MessageId,
    provider_message_id: impl Into<String>,
    status: ReceiptStatus,
) -> Result<DeliveryReceipt, EmailError> {
    let provider_message_id = provider_message_id.into();
    if provider_message_id.trim().is_empty() {
        return Err(EmailError::Empty("provider message id"));
    }
    Ok(DeliveryReceipt {
        message_id,
        provider_message_id,
        status,
    })
}

/// Webhook parsing and signature failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WebhookError {
    /// The payload could not be decoded.
    InvalidPayload,
    /// A required webhook field was absent.
    MissingField(&'static str),
    /// The signature did not verify.
    InvalidSignature,
    /// The signature timestamp is outside the replay-protection window.
    StaleSignature,
    /// The provider status is not terminal or recognized.
    UnknownStatus,
}

impl fmt::Display for WebhookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for WebhookError {}

/// Mailgun event webhook envelope fields used for delivery normalization.
#[derive(Clone, Debug, Deserialize)]
pub struct MailgunWebhook {
    /// Mailgun signature envelope.
    pub signature: MailgunSignature,
    /// Event data.
    pub event_data: MailgunEventData,
}

/// Mailgun signature fields.
#[derive(Clone, Debug, Deserialize)]
pub struct MailgunSignature {
    /// Unix timestamp represented as a string.
    pub timestamp: String,
    /// Webhook token.
    pub token: String,
    /// Hex HMAC-SHA256 signature.
    pub signature: String,
}

/// Mailgun event data fields.
#[derive(Clone, Debug, Deserialize)]
pub struct MailgunEventData {
    /// Provider message id.
    pub id: String,
    /// Mailgun event name.
    pub event: String,
    /// Application metadata attached to the provider event.
    #[serde(rename = "user-variables", default)]
    pub user_variables: BTreeMap<String, String>,
}

/// Parses and verifies a Mailgun webhook using the signing key.
pub fn parse_mailgun_webhook(
    payload: &[u8],
    signing_key: &str,
    message_id: MessageId,
) -> Result<DeliveryReceipt, WebhookError> {
    parse_mailgun_webhook_at(payload, signing_key, message_id, None, 0)
}

/// Parses a Mailgun webhook with timestamp freshness validation.
pub fn parse_mailgun_webhook_at(
    payload: &[u8],
    signing_key: &str,
    message_id: MessageId,
    now_unix_seconds: Option<i64>,
    max_age_seconds: i64,
) -> Result<DeliveryReceipt, WebhookError> {
    let webhook: MailgunWebhook =
        serde_json::from_slice(payload).map_err(|_| WebhookError::InvalidPayload)?;
    if let Some(now) = now_unix_seconds {
        let timestamp = webhook
            .signature
            .timestamp
            .parse::<i64>()
            .map_err(|_| WebhookError::InvalidPayload)?;
        if timestamp > now || now.saturating_sub(timestamp) > max_age_seconds {
            return Err(WebhookError::StaleSignature);
        }
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(signing_key.as_bytes())
        .map_err(|_| WebhookError::InvalidSignature)?;
    mac.update(format!("{}{}", webhook.signature.timestamp, webhook.signature.token).as_bytes());
    let supplied =
        decode_hex(&webhook.signature.signature).ok_or(WebhookError::InvalidSignature)?;
    mac.verify_slice(&supplied)
        .map_err(|_| WebhookError::InvalidSignature)?;
    let status = match webhook.event_data.event.as_str() {
        "delivered" => ReceiptStatus::Delivered,
        "failed" | "rejected" | "undelivered" => ReceiptStatus::Failed,
        _ => return Err(WebhookError::UnknownStatus),
    };
    normalize_receipt(message_id, webhook.event_data.id, status)
        .map_err(|_| WebhookError::InvalidPayload)
}

/// Parses a Mailgun event and extracts `user-variables.aegis_message_id`.
pub fn parse_mailgun_webhook_from_event(
    payload: &[u8],
    signing_key: &str,
    now_unix_seconds: i64,
    max_age_seconds: i64,
) -> Result<DeliveryReceipt, WebhookError> {
    let webhook: MailgunWebhook =
        serde_json::from_slice(payload).map_err(|_| WebhookError::InvalidPayload)?;
    let message_id = webhook
        .event_data
        .user_variables
        .get("aegis_message_id")
        .ok_or(WebhookError::MissingField(
            "user-variables.aegis_message_id",
        ))?;
    parse_mailgun_webhook_at(
        payload,
        signing_key,
        MessageId::new(message_id).map_err(|_| WebhookError::InvalidPayload)?,
        Some(now_unix_seconds),
        max_age_seconds,
    )
}

/// Resend event fields used for delivery normalization.
#[derive(Clone, Debug, Deserialize)]
pub struct ResendWebhook {
    /// Event type, such as `email.delivered`.
    pub r#type: String,
    /// Event payload.
    pub data: ResendWebhookData,
}

/// Resend event data.
#[derive(Clone, Debug, Deserialize)]
pub struct ResendWebhookData {
    /// Provider email id.
    pub email_id: String,
}

/// Parses a Resend event payload. Signature verification is performed by the
/// caller's Svix-compatible ingress because Resend signs the complete HTTP
/// envelope, including timestamp and message id.
pub fn parse_resend_webhook(
    payload: &[u8],
    message_id: MessageId,
) -> Result<DeliveryReceipt, WebhookError> {
    let webhook: ResendWebhook =
        serde_json::from_slice(payload).map_err(|_| WebhookError::InvalidPayload)?;
    let status = match webhook.r#type.as_str() {
        "email.delivered" => ReceiptStatus::Delivered,
        "email.bounced" | "email.failed" => ReceiptStatus::Failed,
        _ => return Err(WebhookError::UnknownStatus),
    };
    normalize_receipt(message_id, webhook.data.email_id, status)
        .map_err(|_| WebhookError::InvalidPayload)
}

/// Verifies a Resend webhook's Svix signature before parsing its event.
pub fn verify_resend_webhook_signature(
    payload: &[u8],
    svix_id: &str,
    svix_timestamp: &str,
    svix_signature: &str,
    signing_secret: &str,
    now_unix_seconds: i64,
    tolerance_seconds: i64,
) -> Result<(), WebhookError> {
    let timestamp = svix_timestamp
        .parse::<i64>()
        .map_err(|_| WebhookError::InvalidSignature)?;
    if timestamp > now_unix_seconds
        || now_unix_seconds.saturating_sub(timestamp) > tolerance_seconds
    {
        return Err(WebhookError::StaleSignature);
    }
    let secret = signing_secret
        .strip_prefix("whsec_")
        .unwrap_or(signing_secret);
    let secret = base64_decode(secret).ok_or(WebhookError::InvalidSignature)?;
    let valid = svix_signature.split_whitespace().any(|signature| {
        let Some(value) = signature.strip_prefix("v1,") else {
            return false;
        };
        let Some(candidate) = base64_decode(value) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&secret) else {
            return false;
        };
        mac.update(format!("{svix_id}.{svix_timestamp}.").as_bytes());
        mac.update(payload);
        mac.verify_slice(&candidate).is_ok()
    });
    if valid {
        Ok(())
    } else {
        Err(WebhookError::InvalidSignature)
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

fn base64_decode(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in value.bytes() {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Some(output)
}

/// Converts an email into a aegis-mesg-sender-core request after validation.
pub fn to_delivery_request(
    email: &EmailMessage,
    message_id: &MessageId,
) -> Result<DeliveryRequest, EmailError> {
    email.encode(message_id)
}

impl From<ValidationError> for EmailError {
    fn from(_: ValidationError) -> Self {
        EmailError::Serialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Transport {
        requests: Mutex<Vec<HttpRequest>>,
        response: HttpResponse,
    }
    impl HttpTransport for Transport {
        fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
            self.requests.lock().unwrap().push(request);
            Ok(self.response.clone())
        }
    }

    fn email() -> EmailMessage {
        EmailMessage::builder("sender@example.test")
            .to("alice@example.test")
            .subject("Hello")
            .text("Body")
            .build()
            .unwrap()
    }

    #[test]
    fn validates_addresses_and_body() {
        assert_eq!(
            EmailMessage::builder("bad")
                .to("a@example.test")
                .subject("x")
                .text("x")
                .build(),
            Err(EmailError::InvalidAddress)
        );
        assert_eq!(
            EmailMessage::builder("a@example.test")
                .subject("x")
                .text("x")
                .build(),
            Err(EmailError::NoRecipients)
        );
    }

    #[test]
    fn resend_maps_request_and_response() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"re_123"}"#.to_vec(),
            },
        });
        let adapter = Resend::new(
            HttpProviderConfig::new("https://api.resend.test", "secret").unwrap(),
            transport.clone(),
        );
        let request = to_delivery_request(
            &EmailMessage::builder("sender@example.test")
                .to("alice@example.test")
                .subject("Hello")
                .text("Body")
                .idempotency_key("idem-1")
                .build()
                .unwrap(),
            &MessageId::new("m-1").unwrap(),
        )
        .unwrap();
        let response = adapter.send(&request).unwrap();
        assert_eq!(response.provider_message_id, "re_123");
        let request = &transport.requests.lock().unwrap()[0];
        assert_eq!(request.url, "https://api.resend.test/emails");
        assert_eq!(request.headers["idempotency-key"], "idem-1");
        assert!(String::from_utf8_lossy(&request.body).contains("alice@example.test"));
    }

    #[test]
    fn provider_success_fixtures_have_stable_ids() {
        for (fixture, expected) in [
            (
                include_str!("../fixtures/resend/success.json"),
                "resend-fixture-0001",
            ),
            (
                include_str!("../fixtures/mailgun/success.json"),
                "<mailgun-fixture-0001@example.test>",
            ),
            (
                include_str!("../fixtures/mailpit/success.json"),
                "mailpit-fixture-0001",
            ),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(
                value.get("id").and_then(serde_json::Value::as_str),
                Some(expected)
            );
        }
    }

    #[test]
    fn resend_payload_contains_supported_fields_and_encoded_attachment() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"re_full"}"#.to_vec(),
            },
        });
        let email = EmailMessage::builder("sender@example.test")
            .to("alice@example.test")
            .cc("copy@example.test")
            .bcc("blind@example.test")
            .reply_to("reply@example.test")
            .subject("Subject")
            .text("Plain")
            .html("<p>HTML</p>")
            .header("X-Trace", "trace-1")
            .attachment(Attachment {
                filename: "hello.txt".into(),
                content_type: "text/plain".into(),
                data: b"hello".to_vec(),
            })
            .build()
            .unwrap();
        Resend::new(
            HttpProviderConfig::new("https://api.resend.test", "secret").unwrap(),
            transport.clone(),
        )
        .send(&to_delivery_request(&email, &MessageId::new("m-full").unwrap()).unwrap())
        .unwrap();
        let requests = transport.requests.lock().unwrap();
        let body = String::from_utf8_lossy(&requests[0].body);
        for expected in [
            "copy@example.test",
            "blind@example.test",
            "reply@example.test",
            "X-Trace",
            "hello.txt",
            "aGVsbG8=",
        ] {
            assert!(body.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn status_errors_are_normalized_without_response_body() {
        let transport = Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 429,
                headers: BTreeMap::new(),
                body: b"secret provider detail".to_vec(),
            },
        };
        let adapter = Resend::new(
            HttpProviderConfig::new("https://api.resend.test", "secret").unwrap(),
            Arc::new(transport),
        );
        let error = adapter
            .send(&to_delivery_request(&email(), &MessageId::new("m-1").unwrap()).unwrap())
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::RateLimited);
        assert_eq!(error.message, "provider rejected request");
    }

    #[cfg(feature = "mailgun")]
    #[test]
    fn mailgun_uses_domain_form_endpoint() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"mg-1"}"#.to_vec(),
            },
        });
        let adapter = Mailgun::new(
            HttpProviderConfig::new("https://api.mailgun.test", "key").unwrap(),
            "example.test",
            transport.clone(),
        )
        .unwrap()
        .with_sandbox(true);
        adapter
            .send(&to_delivery_request(&email(), &MessageId::new("m-1").unwrap()).unwrap())
            .unwrap();
        let request = &transport.requests.lock().unwrap()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.url,
            "https://api.mailgun.test/example.test/messages"
        );
        assert_eq!(
            request.headers["content-type"],
            "application/x-www-form-urlencoded"
        );
        assert!(String::from_utf8_lossy(&request.body).contains("o%3Atestmode=yes"));
    }

    #[cfg(feature = "mailpit")]
    #[test]
    fn mailpit_uses_local_json_endpoint() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"mp-1"}"#.to_vec(),
            },
        });
        let adapter = Mailpit::new("http://localhost:8025", transport.clone()).unwrap();
        adapter
            .send(&to_delivery_request(&email(), &MessageId::new("m-1").unwrap()).unwrap())
            .unwrap();
        let request = &transport.requests.lock().unwrap()[0];
        assert_eq!(request.url, "http://localhost:8025/api/v1/send");
        assert_eq!(request.headers["content-type"], "application/json");
    }

    #[test]
    fn receipt_is_separate_from_acceptance() {
        let receipt = normalize_receipt(
            MessageId::new("m-1").unwrap(),
            "provider-1",
            ReceiptStatus::Delivered,
        )
        .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Delivered);
    }

    #[cfg(feature = "mailgun")]
    #[test]
    fn mailgun_encodes_fields_and_maps_recipients_and_headers() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"mg-2"}"#.to_vec(),
            },
        });
        let email = EmailMessage::builder("sender@example.test")
            .to("alice@example.test")
            .cc("copy@example.test")
            .bcc("blind@example.test")
            .reply_to("reply@example.test")
            .subject("Hello & goodbye")
            .text("Body with spaces")
            .header("X-Trace", "a=b")
            .build()
            .unwrap();
        let adapter = Mailgun::new(
            HttpProviderConfig::new("https://api.mailgun.test", "key").unwrap(),
            "example.test",
            transport.clone(),
        )
        .unwrap();
        adapter
            .send(&to_delivery_request(&email, &MessageId::new("m-2").unwrap()).unwrap())
            .unwrap();
        let request = &transport.requests.lock().unwrap()[0];
        assert_eq!(request.headers["authorization"], "Basic YXBpOmtleQ==");
        let body = String::from_utf8_lossy(&request.body);
        assert!(body.contains("subject=Hello%20%26%20goodbye"));
        assert!(body.contains("cc=copy%40example.test"));
        assert!(body.contains("h%3AX-Trace=a%3Db"));
    }

    #[cfg(feature = "mailgun")]
    #[test]
    fn mailgun_encodes_attachments_as_multipart() {
        let transport = Arc::new(Transport {
            requests: Mutex::new(Vec::new()),
            response: HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"id":"mg-3"}"#.to_vec(),
            },
        });
        let email = EmailMessage::builder("sender@example.test")
            .to("alice@example.test")
            .subject("Hello")
            .text("Body")
            .attachment(Attachment {
                filename: "a.txt".into(),
                content_type: "text/plain".into(),
                data: b"content".to_vec(),
            })
            .build()
            .unwrap();
        let adapter = Mailgun::new(
            HttpProviderConfig::new("https://api.mailgun.test", "key").unwrap(),
            "example.test",
            transport.clone(),
        )
        .unwrap();
        adapter
            .send(&to_delivery_request(&email, &MessageId::new("m-3").unwrap()).unwrap())
            .unwrap();
        let request = &transport.requests.lock().unwrap()[0];
        assert!(request.headers["content-type"].starts_with("multipart/form-data; boundary="));
        let body = String::from_utf8_lossy(&request.body);
        assert!(body.contains("filename=\"a.txt\""));
        assert!(body.contains("content"));
    }

    #[test]
    fn provider_webhooks_verify_and_map_terminal_statuses() {
        let mut mac = Hmac::<Sha256>::new_from_slice(b"signing-key").unwrap();
        mac.update(b"1700000000token");
        let signature = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let payload = serde_json::json!({
            "signature": {"timestamp": "1700000000", "token": "token", "signature": signature},
            "event_data": {"id": "mailgun-id", "event": "delivered"}
        });
        let receipt = parse_mailgun_webhook(
            &serde_json::to_vec(&payload).unwrap(),
            "signing-key",
            MessageId::new("m-mailgun").unwrap(),
        )
        .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Delivered);

        let resend = serde_json::json!({
            "type": "email.bounced",
            "data": {"email_id": "resend-id"}
        });
        let receipt = parse_resend_webhook(
            &serde_json::to_vec(&resend).unwrap(),
            MessageId::new("m-resend").unwrap(),
        )
        .unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Failed);
    }

    #[test]
    fn webhook_replay_protection_and_resend_svix_verification_work() {
        let mut mailgun_mac = Hmac::<Sha256>::new_from_slice(b"signing-key").unwrap();
        mailgun_mac.update(b"1700000000token");
        let signature = mailgun_mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mailgun = serde_json::json!({
            "signature": {"timestamp": "1700000000", "token": "token", "signature": signature},
            "event_data": {"id": "mailgun-id", "event": "delivered", "user-variables": {"aegis_message_id": "m-mailgun"}}
        });
        assert_eq!(
            parse_mailgun_webhook_from_event(
                &serde_json::to_vec(&mailgun).unwrap(),
                "signing-key",
                1_700_000_060,
                300,
            )
            .unwrap()
            .message_id
            .as_str(),
            "m-mailgun"
        );
        assert_eq!(
            parse_mailgun_webhook_from_event(
                &serde_json::to_vec(&mailgun).unwrap(),
                "signing-key",
                1_700_001_000,
                300,
            )
            .unwrap_err(),
            WebhookError::StaleSignature
        );

        let payload = br#"{"type":"email.delivered","data":{"email_id":"re-1"}}"#;
        let secret = b"webhook-secret";
        let secret_encoded = base64_encode(secret);
        let mut resend_mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        resend_mac.update(b"msg-1.1700000000.");
        resend_mac.update(payload);
        let signature = format!("v1,{}", base64_encode(&resend_mac.finalize().into_bytes()));
        verify_resend_webhook_signature(
            payload,
            "msg-1",
            "1700000000",
            &signature,
            &format!("whsec_{secret_encoded}"),
            1_700_000_060,
            300,
        )
        .unwrap();
    }
}
