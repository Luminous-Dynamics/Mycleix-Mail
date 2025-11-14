use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::client::MycellixClient;
use crate::types::MailMessage;

/// List inbox messages with filtering and formatting
pub async fn handle_inbox(
    client: &MycellixClient,
    from: Option<String>,
    trust_min: Option<f64>,
    unread: bool,
    limit: usize,
    format: &str,
) -> Result<()> {
    println!("📬 Fetching inbox...");
    println!();

    // 1. Get inbox messages from client
    let mut messages = client
        .get_inbox()
        .await
        .context("Failed to fetch inbox messages")?;

    // Show filters being applied
    let mut filter_count = 0;
    if from.is_some() {
        filter_count += 1;
    }
    if trust_min.is_some() {
        filter_count += 1;
    }
    if unread {
        filter_count += 1;
    }

    if filter_count > 0 {
        println!("🔍 Applying {} filter(s):", filter_count);
        if let Some(ref sender) = from {
            println!("   • From: {}", sender);
        }
        if let Some(min_trust) = trust_min {
            println!("   • Minimum trust: {:.2}", min_trust);
        }
        if unread {
            println!("   • Unread only");
        }
        println!();
    }

    // 2. Apply filters
    messages = apply_filters(messages, from, trust_min, unread);

    // 3. Sort by timestamp (newest first)
    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // 4. Apply limit
    let total_count = messages.len();
    messages.truncate(limit);

    // Handle empty inbox
    if messages.is_empty() {
        if filter_count > 0 {
            println!("No messages match your filters.");
            println!("Try removing some filters or check 'mycelix-mail inbox' to see all messages.");
        } else {
            println!("Your inbox is empty.");
            println!();
            println!("To receive messages:");
            println!("  1. Share your DID with contacts: mycelix-mail did");
            println!("  2. Wait for others to send you messages");
        }
        return Ok(());
    }

    // 5. Format and display
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Showing {} of {} message(s)", messages.len(), total_count);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    match format {
        "json" => display_json(&messages)?,
        "raw" => display_raw(&messages),
        _ => display_table(&messages),
    }

    println!();

    // Show helpful tips
    if messages.len() < total_count {
        println!(
            "💡 Showing first {} messages. Use --limit {} to see more.",
            limit,
            total_count
        );
    }

    Ok(())
}

/// Apply filters to message list
fn apply_filters(
    messages: Vec<MailMessage>,
    from: Option<String>,
    trust_min: Option<f64>,
    unread: bool,
) -> Vec<MailMessage> {
    messages
        .into_iter()
        .filter(|msg| {
            // Filter by sender
            if let Some(ref sender_did) = from {
                if !msg.from_did.contains(sender_did) {
                    return false;
                }
            }

            // Filter by trust score
            // TODO: Implement trust score lookup
            // For now, we skip trust filtering since we don't have trust data
            if trust_min.is_some() {
                // Would check: client.get_trust_score(&msg.from_did).score >= trust_min
            }

            // Filter by unread status
            if unread {
                // TODO: Implement read status tracking
                // For now, all messages are considered unread
            }

            true
        })
        .collect()
}

/// Display messages in table format
fn display_table(messages: &[MailMessage]) {
    println!("{:<6} {:<40} {:<20} {:<20} {:<6}",
        "ID", "From", "Subject", "Time", "Tier"
    );
    println!("{}", "─".repeat(95));

    for (i, msg) in messages.iter().enumerate() {
        let msg_id = format!("#{}", i + 1);
        let from_short = truncate_did(&msg.from_did, 38);
        let subject = decrypt_subject(&msg.subject_encrypted);
        let subject_short = truncate_string(&subject, 18);
        let time_str = format_timestamp(msg.timestamp);
        let tier_short = format_tier_short(&msg.epistemic_tier);

        println!("{:<6} {:<40} {:<20} {:<20} {:<6}",
            msg_id, from_short, subject_short, time_str, tier_short
        );
    }

    println!();
    println!("💡 Use 'mycelix-mail read <id>' to view full message");
}

/// Display messages in JSON format
fn display_json(messages: &[MailMessage]) -> Result<()> {
    let json = serde_json::to_string_pretty(messages)
        .context("Failed to serialize messages to JSON")?;
    println!("{}", json);
    Ok(())
}

/// Display messages in raw format
fn display_raw(messages: &[MailMessage]) {
    for (i, msg) in messages.iter().enumerate() {
        println!("Message #{}", i + 1);
        println!("  From: {}", msg.from_did);
        println!("  To: {}", msg.to_did);
        println!("  Subject (encrypted): {} bytes", msg.subject_encrypted.len());
        println!("  Body CID: {}", msg.body_cid);
        println!("  Timestamp: {} ({})", msg.timestamp, format_timestamp(msg.timestamp));
        println!("  Tier: {:?}", msg.epistemic_tier);
        if let Some(ref thread) = msg.thread_id {
            println!("  Thread: {}", thread);
        }
        println!();
    }
}

/// Truncate a DID for display
fn truncate_did(did: &str, max_len: usize) -> String {
    if did.len() <= max_len {
        did.to_string()
    } else {
        // Show prefix and suffix
        let prefix_len = max_len.saturating_sub(10);
        let suffix_len = 7;
        format!("{}...{}", &did[..prefix_len], &did[did.len()-suffix_len..])
    }
}

/// Truncate a string for display
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format timestamp as relative time
fn format_timestamp(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());

    let now = Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

/// Format epistemic tier as short string
fn format_tier_short(tier: &crate::types::EpistemicTier) -> String {
    use crate::types::EpistemicTier;
    match tier {
        EpistemicTier::Tier0Null => "T0".to_string(),
        EpistemicTier::Tier1Testimonial => "T1".to_string(),
        EpistemicTier::Tier2PrivatelyVerifiable => "T2".to_string(),
        EpistemicTier::Tier3CryptographicallyProven => "T3".to_string(),
        EpistemicTier::Tier4PubliclyReproducible => "T4".to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EpistemicTier;

    #[test]
    fn test_truncate_did() {
        let did = "did:mycelix:ATHMuhr4Mk9fx2VMUx5kzVPVkL5zyvQGZ1gofWQmJtG6";
        let truncated = truncate_did(did, 30);
        assert!(truncated.len() <= 30);
        assert!(truncated.contains("..."));
    }

    #[test]
    fn test_truncate_string() {
        let long = "This is a very long string that should be truncated";
        let short = truncate_string(long, 20);
        assert_eq!(short.len(), 20);
        assert!(short.ends_with("..."));
    }

    #[test]
    fn test_format_tier_short() {
        assert_eq!(format_tier_short(&EpistemicTier::Tier0Null), "T0");
        assert_eq!(format_tier_short(&EpistemicTier::Tier2PrivatelyVerifiable), "T2");
        assert_eq!(format_tier_short(&EpistemicTier::Tier4PubliclyReproducible), "T4");
    }

    #[test]
    fn test_decrypt_subject_placeholder() {
        let encrypted = b"ENC:Test Subject";
        let decrypted = decrypt_subject(encrypted);
        assert_eq!(decrypted, "Test Subject");
    }

    #[test]
    fn test_apply_filters_empty() {
        let messages = vec![];
        let filtered = apply_filters(messages, None, None, false);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_apply_filters_by_sender() {
        let msg = MailMessage {
            from_did: "did:mycelix:ABC123".to_string(),
            to_did: "did:mycelix:XYZ789".to_string(),
            subject_encrypted: b"Test".to_vec(),
            body_cid: "bafyrei123".to_string(),
            timestamp: 1234567890,
            thread_id: None,
            epistemic_tier: EpistemicTier::Tier2PrivatelyVerifiable,
        };

        let messages = vec![msg.clone()];

        // Should match
        let filtered = apply_filters(messages.clone(), Some("ABC".to_string()), None, false);
        assert_eq!(filtered.len(), 1);

        // Should not match
        let filtered = apply_filters(messages.clone(), Some("ZZZ".to_string()), None, false);
        assert_eq!(filtered.len(), 0);
    }
}
