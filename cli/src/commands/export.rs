use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use std::fs::File;
use std::io::Write;

use crate::client::MycellixClient;
use crate::types::MailMessage;

/// Export messages to file in various formats
pub async fn handle_export(
    client: &MycellixClient,
    format: &str,
    output: String,
    since: Option<String>,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("                      MESSAGE EXPORT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📦 Output File:  {}", output);
    println!("📄 Format:       {}", format_name(format));
    if let Some(ref date) = since {
        println!("📅 Filter:       Messages since {}", date);
    }
    println!();

    // 1. Fetch all messages
    println!("📬 Fetching inbox messages...");
    let inbox_messages = client
        .get_inbox()
        .await
        .context("Failed to fetch inbox messages")?;

    println!("📭 Fetching sent messages...");
    let sent_messages = client
        .get_sent()
        .await
        .context("Failed to fetch sent messages")?;

    // 2. Combine messages
    let mut all_messages = inbox_messages;
    all_messages.extend(sent_messages);

    println!();
    println!("📊 Found {} total message(s)", all_messages.len());

    // 3. Apply date filter if specified
    if let Some(since_date) = since {
        let filter_timestamp = parse_date(&since_date)?;
        let before_count = all_messages.len();
        all_messages.retain(|msg| msg.timestamp >= filter_timestamp);
        let after_count = all_messages.len();
        println!("🔍 Filtered to {} message(s) since {}", after_count, since_date);
        if before_count > after_count {
            println!("   (Excluded {} older message(s))", before_count - after_count);
        }
    }

    // Handle no messages to export
    if all_messages.is_empty() {
        println!();
        println!("⚠️  No messages to export!");
        println!();
        println!("💡 Tips:");
        println!("   • Check if you have any messages in your inbox or sent folder");
        println!("   • Try removing the --since filter to export all messages");
        return Ok(());
    }

    println!();
    println!("💾 Writing {} message(s) to {}...", all_messages.len(), output);

    // 4. Export to file
    match format {
        "json" => export_json(&all_messages, &output)?,
        "mbox" => export_mbox(&all_messages, &output)?,
        "csv" => export_csv(&all_messages, &output)?,
        _ => bail!("Unsupported export format: {}", format),
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("                     EXPORT COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("✅ Successfully exported {} message(s)", all_messages.len());
    println!("📁 File: {}", output);
    println!("📊 Format: {}", format_name(format));
    println!();
    println!("💡 You can now open this file with your preferred application");

    Ok(())
}

/// Export messages to JSON format
fn export_json(messages: &[MailMessage], output: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(messages)
        .context("Failed to serialize messages to JSON")?;

    let mut file = File::create(output)
        .context(format!("Failed to create output file: {}", output))?;

    file.write_all(json.as_bytes())
        .context("Failed to write JSON data")?;

    Ok(())
}

/// Export messages to MBOX format (standard Unix mailbox format)
fn export_mbox(messages: &[MailMessage], output: &str) -> Result<()> {
    let mut file = File::create(output)
        .context(format!("Failed to create output file: {}", output))?;

    for msg in messages {
        // MBOX format starts each message with "From " line
        let from_line = format!("From {} {}\n",
            msg.from_did,
            format_timestamp_mbox(msg.timestamp)
        );
        file.write_all(from_line.as_bytes())
            .context("Failed to write MBOX from line")?;

        // Headers
        let headers = format!(
            "From: {}\nTo: {}\nDate: {}\nSubject: {}\n",
            msg.from_did,
            msg.to_did,
            format_timestamp_rfc2822(msg.timestamp),
            decrypt_subject(&msg.subject_encrypted)
        );
        file.write_all(headers.as_bytes())
            .context("Failed to write MBOX headers")?;

        // Body (placeholder - would fetch from CID in real implementation)
        let body = format!("\n[Body CID: {}]\n\n", msg.body_cid);
        file.write_all(body.as_bytes())
            .context("Failed to write MBOX body")?;
    }

    Ok(())
}

/// Export messages to CSV format
fn export_csv(messages: &[MailMessage], output: &str) -> Result<()> {
    let mut file = File::create(output)
        .context(format!("Failed to create output file: {}", output))?;

    // CSV header
    let header = "Timestamp,Date,From,To,Subject,BodyCID,Tier,ThreadID\n";
    file.write_all(header.as_bytes())
        .context("Failed to write CSV header")?;

    // CSV rows
    for msg in messages {
        let subject = decrypt_subject(&msg.subject_encrypted);
        let thread_id = msg.thread_id.as_deref().unwrap_or("");
        let tier = format!("{:?}", msg.epistemic_tier);

        let row = format!(
            "{},{},{},{},{},{},{},{}\n",
            msg.timestamp,
            format_timestamp_iso8601(msg.timestamp),
            escape_csv(&msg.from_did),
            escape_csv(&msg.to_did),
            escape_csv(&subject),
            escape_csv(&msg.body_cid),
            tier,
            escape_csv(thread_id)
        );
        file.write_all(row.as_bytes())
            .context("Failed to write CSV row")?;
    }

    Ok(())
}

/// Parse date string to Unix timestamp
fn parse_date(date_str: &str) -> Result<i64> {
    // TODO: Implement proper date parsing
    // For now, just parse as Unix timestamp
    date_str.parse::<i64>()
        .context(format!("Failed to parse date: {}. Please provide Unix timestamp for now.", date_str))
}

/// Format timestamp for MBOX "From " line
fn format_timestamp_mbox(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());
    dt.format("%a %b %d %H:%M:%S %Y").to_string()
}

/// Format timestamp as RFC 2822 (for email headers)
fn format_timestamp_rfc2822(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());
    dt.to_rfc2822()
}

/// Format timestamp as ISO 8601 (for CSV)
fn format_timestamp_iso8601(ts: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now());
    dt.to_rfc3339()
}

/// Escape CSV field (add quotes if contains comma, newline, or quote)
fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('\n') || field.contains('"') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Format name for display
fn format_name(format: &str) -> &str {
    match format {
        "json" => "JSON (Machine-readable)",
        "mbox" => "MBOX (Standard Unix mailbox)",
        "csv" => "CSV (Spreadsheet-friendly)",
        _ => format,
    }
}

/// Decrypt subject (placeholder implementation)
fn decrypt_subject(encrypted: &[u8]) -> String {
    if let Ok(s) = String::from_utf8(encrypted.to_vec()) {
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
    fn test_escape_csv() {
        assert_eq!(escape_csv("simple"), "simple");
        assert_eq!(escape_csv("has,comma"), "\"has,comma\"");
        assert_eq!(escape_csv("has\"quote"), "\"has\"\"quote\"");
        assert_eq!(escape_csv("has\nnewline"), "\"has\nnewline\"");
    }

    #[test]
    fn test_format_name() {
        assert_eq!(format_name("json"), "JSON (Machine-readable)");
        assert_eq!(format_name("mbox"), "MBOX (Standard Unix mailbox)");
        assert_eq!(format_name("csv"), "CSV (Spreadsheet-friendly)");
    }

    #[test]
    fn test_format_timestamp_iso8601() {
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC
        let formatted = format_timestamp_iso8601(ts);
        assert!(formatted.contains("2021"));
        assert!(formatted.contains("T"));
    }

    #[test]
    fn test_format_timestamp_rfc2822() {
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC
        let formatted = format_timestamp_rfc2822(ts);
        assert!(formatted.contains("2021"));
        assert!(formatted.contains("GMT") || formatted.contains("+0000"));
    }

    #[test]
    fn test_decrypt_subject() {
        let encrypted = b"ENC:Test Subject";
        let decrypted = decrypt_subject(encrypted);
        assert_eq!(decrypted, "Test Subject");

        let plain = b"Plain Subject";
        let result = decrypt_subject(plain);
        assert_eq!(result, "Plain Subject");
    }

    #[test]
    fn test_parse_date() {
        // Test with Unix timestamp
        let result = parse_date("1609459200");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1609459200);

        // Test with invalid format
        let result = parse_date("invalid");
        assert!(result.is_err());
    }
}
