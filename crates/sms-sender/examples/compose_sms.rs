//! Compose and inspect a validated SMS.

use aegis_sms_sender::{PhoneNumber, Sender, SmsMessage};

fn main() {
    let message = SmsMessage::new(
        PhoneNumber::new("+639171234567").expect("valid E.164 number"),
        Sender::Alphanumeric("AEGIS".into()),
        "Your verification code is 123456",
    )
    .expect("valid SMS");
    println!(
        "encoding={:?}, segments={}",
        message.encoding(),
        message.segments()
    );
}
