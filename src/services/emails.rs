use anyhow::Ok;
use aws_config::BehaviorVersion;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

// https://github.com/awslabs/aws-sdk-rust/blob/main/examples/examples/ses/src/bin/send-email.rs
pub async fn send_email_with_aws_ses(
    dest: &str,
    subject: &str,
    message: &str,
) -> anyhow::Result<()> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = aws_sdk_sesv2::Client::new(&config);

    let mut destination: Destination = Destination::builder().build();
    destination.to_addresses = Some(vec![dest.to_string()]);
    let subject_content = Content::builder()
        .data(subject)
        .charset("UTF-8")
        .build()
        .expect("Building subject content");
    let body_content = Content::builder()
        .data(message)
        .charset("UTF-8")
        .build()
        .expect("Building body content");
    let body = Body::builder().text(body_content).build();

    let msg = Message::builder()
        .subject(subject_content)
        .body(body)
        .build();

    let email_content = EmailContent::builder().simple(msg).build();

    client
        .send_email()
        .from_email_address("no-reply@iothings.fr")
        .destination(destination)
        .content(email_content)
        .send()
        .await?;

    Ok(())
}

// https://www.scaleway.com/en/developers/api/transactional-email/
pub async fn send_email_with_scw_api(
    dest: &str,
    subject: &str,
    message: &str,
) -> anyhow::Result<()> {
    Ok(())
}
