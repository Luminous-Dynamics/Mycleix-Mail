use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::client::MycellixClient;

/// Read and display a specific message
pub async fn handle_read(
    client: &MycellixClient,
    message_id: String,
    mark_read: bool,
) -> Result<()> {
    println!("📖 Reading message...");
    println!();

    // 1. Get message from client
    let message = client
        .get_message(&message_id)
        .await
        .context("Failed to fetch message")?;

    // 2. Decrypt subject
    let subject = decrypt_subject(&message.subject_encrypted);

    // 3. Fetch body from IPFS/DHT
    let body = fetch_body(&message.body_cid).await?;

    // 4. Display formatted message
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("                         MESSAGE DETAILS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📬 From:    {}", message.from_did);
    println!("📭 To:      {}", message.to_did);
    println!("📅 Date:    {}", format_timestamp(message.timestamp));
    println!("🏷️  Tier:    {}", format_tier(&message.epistemic_tier));

    if let Some(ref thread) = message.thread_id {
        println!("🧵 Thread:  {}", thread);
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Subject: {}", subject);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("{}", body);
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 5. Mark as read if requested
    if mark_read {
        println!();
        println!("✅ Marking message as read...");

        client
            .mark_read(&message_id)
            .await
            .context("Failed to mark message as read")?;

        println!("   Message marked as read");
    } else {
        println!();
        println!("💡 Use --mark-read to mark this message as read");
    }

    Ok(())
}

/// Decrypt subject (placeholder implementation)
///
/// TODO: Implement real decryption
/// - Fetch sender's public key
/// - Decrypt using our private key
fn decrypt_subject(encrypted: &[u8]) -> String {
    // Placeholder: Just try to convert from bytes
    // In real implementation, this would use NaCl decryption

    if let Ok(s) = String::from_utf8(encrypted.to_vec()) {
        // If it's our fake "ENC:" prefix, strip it
        if s.starts_with("ENC:") {
            s[4..].to_string()
        } else {
            s
        }
    } else {
        "<encrypted>".to_string()
    }
}

/// Fetch body from IPFS/DHT (placeholder implementation)
///
/// TODO: Implement real body fetching
/// - Fetch from IPFS or Holochain DHT using CID
/// - Decrypt if encrypted
/// - Return body text
async fn fetch_body(cid: &str) -> Result<String> {
    // Placeholder: Just return a message indicating we would fetch from IPFS
    // In real implementation:
    // 1. Connect to IPFS node or Holochain DHT
    // 2. Fetch content by CID
    // 3. Decrypt if encrypted
    // 4. Return text content

    if cid.starts_with("bafyrei") {
        // This is our fake CID format from send command
        Ok(format!(
            "(Message body would be fetched from IPFS/DHT using CID: {})\n\
             \n\
             In production, this would display the actual message content\n\
             fetched from the distributed hash table.",
            cid
        ))
    } else {
        bail!("Invalid CID format: {}", cid)
    }
}

/// Format timestamp as human-readable date/time
fn format_timestamp(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());

    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format epistemic tier as full string
fn format_tier(tier: &crate::types::EpistemicTier) -> String {
    use crate::types::EpistemicTier;
    match tier {
        EpistemicTier::Tier0Null => "Tier 0 (Null - Unverifiable belief)".to_string(),
        EpistemicTier::Tier1Testimonial => "Tier 1 (Testimonial - Personal attestation)".to_string(),
        EpistemicTier::Tier2PrivatelyVerifiable => "Tier 2 (Privately Verifiable - Audit guild)".to_string(),
        EpistemicTier::Tier3CryptographicallyProven => "Tier 3 (Cryptographically Proven - ZKP)".to_string(),
        EpistemicTier::Tier4PubliclyReproducible => "Tier 4 (Publicly Reproducible - Open data/code)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_subject_placeholder() {
        // Test with our fake "ENC:" prefix
        let encrypted = b"ENC:Test Subject";
        let decrypted = decrypt_subject(encrypted);
        assert_eq!(decrypted, "Test Subject");

        // Test with non-encrypted text
        let plain = b"Plain Subject";
        let result = decrypt_subject(plain);
        assert_eq!(result, "Plain Subject");
    }

    #[tokio::test]
    async fn test_fetch_body_valid_cid() {
        let cid = "bafyrei1234567890abcdef";
        let body = fetch_body(cid).await.unwrap();

        // Should contain CID in message
        assert!(body.contains(cid));
        assert!(body.contains("IPFS/DHT"));
    }

    #[tokio::test]
    async fn test_fetch_body_invalid_cid() {
        let cid = "invalid-cid-format";
        let result = fetch_body(cid).await;

        // Should fail for invalid CID
        assert!(result.is_err());
    }

    #[test]
    fn test_format_timestamp() {
        // Test with known timestamp
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("2021"));
        assert!(formatted.contains("UTC"));
    }

    #[test]
    fn test_format_tier() {
        use crate::types::EpistemicTier;

        let tier0 = format_tier(&EpistemicTier::Tier0Null);
        assert!(tier0.contains("Tier 0"));
        assert!(tier0.contains("Null"));

        let tier2 = format_tier(&EpistemicTier::Tier2PrivatelyVerifiable);
        assert!(tier2.contains("Tier 2"));
        assert!(tier2.contains("Privately Verifiable"));

        let tier4 = format_tier(&EpistemicTier::Tier4PubliclyReproducible);
        assert!(tier4.contains("Tier 4"));
        assert!(tier4.contains("Publicly Reproducible"));
    }
}
