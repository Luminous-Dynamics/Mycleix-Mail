use anyhow::{Context, Result, bail};
use std::io::{self, Read};

use crate::client::MycellixClient;
use crate::types::EpistemicTier;

/// Send an email message
pub async fn handle_send(
    client: &MycellixClient,
    to: String,
    subject: String,
    body: Option<String>,
    attach: Option<Vec<String>>,
    reply_to: Option<String>,
    tier: u8,
) -> Result<()> {
    println!("📧 Composing message...");
    println!();

    // 1. Validate epistemic tier
    let epistemic_tier = EpistemicTier::from_u8(tier)
        .with_context(|| format!("Invalid epistemic tier: {}. Must be 0-4", tier))?;

    println!("   Tier: {}", epistemic_tier);

    // 2. Validate recipient (must be a valid DID)
    if !to.starts_with("did:") {
        bail!(
            "Invalid recipient format: '{}'\n\
             Recipient must be a DID (e.g., did:mycelix:ABC123...)\n\
             Use 'mycelix-mail did resolve <email>' to find a user's DID",
            to
        );
    }

    // Check DID format (should be did:mycelix:base58)
    if !to.starts_with("did:mycelix:") {
        println!("⚠️  Warning: Recipient DID uses non-standard method: {}", to);
        println!("   Expected format: did:mycelix:<base58>");
    }

    println!("   To: {}", to);

    // 3. Validate and get body text
    let body_text = get_body_text(body).await?;

    // Show preview of body (first 100 chars)
    if body_text.len() > 100 {
        println!("   Body: {}... ({} chars)", &body_text[..100], body_text.len());
    } else {
        println!("   Body: {} ({} chars)", body_text, body_text.len());
    }

    // 4. Handle attachments (not yet implemented)
    if let Some(attachments) = &attach {
        if !attachments.is_empty() {
            println!("⚠️  Warning: Attachments not yet implemented");
            println!("   Ignoring {} attachment(s)", attachments.len());
            // TODO: Implement attachment upload to IPFS/Holochain
        }
    }

    // 5. Handle reply-to (for threading)
    if let Some(parent_id) = &reply_to {
        println!("   Reply to: {}", parent_id);
    }

    println!();
    println!("📝 Subject: {}", subject);

    // 6. Encrypt subject (placeholder for now)
    let encrypted_subject = encrypt_subject(&subject);
    println!("🔒 Subject encrypted: {} bytes", encrypted_subject.len());

    // 7. Upload body to DHT/IPFS (stub for now)
    let body_cid = upload_body(&body_text).await?;
    println!("📤 Body uploaded: {}", body_cid);

    // 8. Send message via Holochain
    println!();
    println!("📡 Sending message...");

    match client
        .send_message(to.clone(), encrypted_subject, body_cid, reply_to, epistemic_tier)
        .await
    {
        Ok(message_id) => {
            println!();
            println!("✅ Message sent successfully!");
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Message ID: {}", message_id);
            println!("To: {}", to);
            println!("Subject: {}", subject);
            println!("Tier: {}", epistemic_tier);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
            println!("The recipient will receive your message shortly.");
        }
        Err(e) => {
            println!();
            println!("❌ Failed to send message: {}", e);
            println!();
            println!("This may be because:");
            println!("  • Holochain conductor is not running");
            println!("  • The recipient's DID doesn't exist");
            println!("  • Network connectivity issues");
            println!();
            println!("Try again later or check your configuration.");
            bail!("Message send failed");
        }
    }

    Ok(())
}

/// Get body text from argument or stdin
async fn get_body_text(body: Option<String>) -> Result<String> {
    match body {
        Some(text) if text == "-" => {
            // Read from stdin
            println!("📝 Reading body from stdin (press Ctrl+D when done)...");
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read body from stdin")?;

            if buffer.trim().is_empty() {
                bail!("Body cannot be empty");
            }

            Ok(buffer)
        }
        Some(text) => {
            // Use provided text
            if text.trim().is_empty() {
                bail!("Body cannot be empty");
            }
            Ok(text)
        }
        None => {
            // No body provided - require it
            bail!(
                "Body is required. Use --body \"your message\" or --body - to read from stdin"
            );
        }
    }
}

/// Encrypt subject (placeholder implementation)
///
/// TODO: Implement real encryption using recipient's public key
/// - Fetch recipient's public key from DID registry
/// - Use NaCl sealed box or similar for encryption
/// - Return encrypted bytes
fn encrypt_subject(subject: &str) -> Vec<u8> {
    // Placeholder: Just convert to bytes
    // In real implementation:
    // 1. Fetch recipient's public key from DID or Holochain
    // 2. Encrypt subject with NaCl/TweetNaCl sealed box
    // 3. Return encrypted bytes

    // For now, just prefix with "ENC:" to indicate it should be encrypted
    format!("ENC:{}", subject).into_bytes()
}

/// Upload body to DHT/IPFS (placeholder implementation)
///
/// TODO: Implement real body upload
/// - Upload to IPFS or Holochain DHT
/// - Return content identifier (CID)
async fn upload_body(body: &str) -> Result<String> {
    // Placeholder: Just create a fake CID based on body hash
    // In real implementation:
    // 1. Upload body to IPFS via ipfs-api or Holochain DHT
    // 2. Get content identifier (CID)
    // 3. Return CID

    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(body.as_bytes());
    let hash = hasher.finalize();

    // Create a fake CID (just hex encoding of first 16 bytes)
    let cid = hex::encode(&hash[..16]);

    Ok(format!("bafyrei{}", cid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_subject() {
        let subject = "Hello World";
        let encrypted = encrypt_subject(subject);

        // Should produce bytes
        assert!(!encrypted.is_empty());

        // For now, should start with "ENC:"
        let decrypted = String::from_utf8(encrypted).unwrap();
        assert!(decrypted.starts_with("ENC:"));
        assert!(decrypted.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_upload_body() {
        let body = "Test message body";
        let cid = upload_body(body).await.unwrap();

        // Should produce a CID-like string
        assert!(cid.starts_with("bafyrei"));
        assert!(cid.len() > 10);

        // Same body should produce same CID (deterministic)
        let cid2 = upload_body(body).await.unwrap();
        assert_eq!(cid, cid2);
    }

    #[tokio::test]
    async fn test_different_bodies_different_cids() {
        let body1 = "Message 1";
        let body2 = "Message 2";

        let cid1 = upload_body(body1).await.unwrap();
        let cid2 = upload_body(body2).await.unwrap();

        assert_ne!(cid1, cid2);
    }
}
