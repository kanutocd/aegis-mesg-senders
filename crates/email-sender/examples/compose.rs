//! Minimal email composition example.

use aegis_email_sender::{to_delivery_request, EmailMessage};
use aegis_mesg_sender_core::MessageId;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let email = EmailMessage::builder("noreply@example.test")
        .to("recipient@example.test")
        .subject("Welcome")
        .text("Thanks for signing up.")
        .build()?;
    let request = to_delivery_request(&email, &MessageId::new("welcome-1")?)?;
    println!("prepared {} byte email request", request.message.body.len());
    Ok(())
}
