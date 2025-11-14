use hdk::prelude::*;
use mycelix_mail_integrity::*;

/// Input for spam reporting
#[derive(Serialize, Deserialize, Debug)]
pub struct SpamReportInput {
    pub message_hash: ActionHash,
    pub reason: String,
}

/// Input for querying stored spam reports
#[derive(Serialize, Deserialize, Debug)]
pub struct GetSpamReportsInput {
    pub since: Timestamp,
}

/// Check the trust score for a specific DID
/// This queries the local MATL trust scores stored on the DHT
#[hdk_extern]
pub fn check_sender_trust(did: String) -> ExternResult<f64> {
    debug!("Checking trust for DID: {}", did);

    // Use a Path for DID-based lookup
    let path = Path::from(format!("trust.{}", did));
    let path_hash = path.path_entry_hash()?;

    // Query trust scores linked to this DID path
    let links =
        get_links(GetLinksInputBuilder::try_new(path_hash, LinkTypes::TrustByDid)?.build())?;

    // If we have a trust score, return it
    if let Some(link) = links.first() {
        // Convert the link target to the appropriate hash type
        let hash_any_dht: AnyDhtHash =
            ActionHash::from_raw_39(link.target.get_raw_39().to_vec()).into();
        let record = get(hash_any_dht, GetOptions::default())?;

        if let Some(record) = record {
            let trust_score: TrustScore = record
                .entry()
                .to_app_option()
                .map_err(|e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "Deserialization error: {:?}",
                        e
                    )))
                })?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Invalid trust score".into()
                )))?;

            debug!("Found trust score for {}: {}", did, trust_score.score);
            return Ok(trust_score.score);
        }
    }

    // Default: neutral score for new/unknown users (0.5)
    // This allows new users to send mail, but they start with low reputation
    debug!("No trust score found for {}, returning default 0.5", did);
    Ok(0.5)
}

/// Update or create a trust score for a DID
/// This is called by the MATL bridge to sync trust scores from the MATL system
#[hdk_extern]
pub fn update_trust_score(trust_score: TrustScore) -> ExternResult<ActionHash> {
    debug!(
        "Updating trust score for {}: {}",
        trust_score.did, trust_score.score
    );

    // Create the trust score entry
    let score_hash = create_entry(EntryTypes::TrustScore(trust_score.clone()))?;

    // Use a Path for DID-based lookup
    let path = Path::from(format!("trust.{}", trust_score.did));
    path.ensure()?;
    let path_hash = path.path_entry_hash()?;

    // Link from DID path to trust score
    create_link(path_hash, score_hash.clone(), LinkTypes::TrustByDid, ())?;

    // Maintain an index path for enumeration
    let index_path = Path::from("trust_index");
    index_path.ensure()?;
    let index_hash = index_path.path_entry_hash()?;
    create_link(index_hash, score_hash.clone(), LinkTypes::TrustIndex, ())?;

    Ok(score_hash)
}

/// Get filtered inbox messages based on minimum trust threshold
/// This is the KEY SPAM FILTER - uses MATL scores to filter messages
#[hdk_extern]
pub fn filter_inbox(min_trust: f64) -> ExternResult<Vec<MailMessage>> {
    debug!("Filtering inbox with min_trust: {}", min_trust);

    // Call the mail_messages zome to get all inbox messages
    let response: ZomeCallResponse = call(
        CallTargetCell::Local,
        "mail_messages",
        "get_inbox".into(),
        None,
        (),
    )?;

    // Decode the response
    let all_messages: Vec<MailMessage> = match response {
        ZomeCallResponse::Ok(result) => decode(&result.into_vec()).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to decode response: {:?}",
                e
            )))
        })?,
        _ => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Zome call failed".into()
            )))
        }
    };

    debug!("Got {} total messages", all_messages.len());

    // Filter by trust score
    let mut trusted_messages = Vec::new();

    for message in all_messages {
        // Check sender's trust score
        let trust_score = check_sender_trust(message.from_did.clone())?;

        if trust_score >= min_trust {
            trusted_messages.push(message);
        } else {
            debug!(
                "Filtered out message from {} (trust: {} < {})",
                message.from_did, trust_score, min_trust
            );
        }
    }

    debug!("Returning {} trusted messages", trusted_messages.len());
    Ok(trusted_messages)
}

/// Get all trust scores (for admin/debugging)
#[hdk_extern]
pub fn get_all_trust_scores(_: ()) -> ExternResult<Vec<TrustScore>> {
    let index_path = Path::from("trust_index");
    let index_hash = match index_path.path_entry_hash() {
        Ok(hash) => hash,
        Err(_) => return Ok(Vec::new()),
    };

    let links =
        get_links(GetLinksInputBuilder::try_new(index_hash, LinkTypes::TrustIndex)?.build())?;

    let mut scores = Vec::new();
    for link in links {
        let hash_any_dht: AnyDhtHash =
            ActionHash::from_raw_39(link.target.get_raw_39().to_vec()).into();
        if let Some(record) = get(hash_any_dht, GetOptions::default())? {
            if let Some(score) = record.entry().to_app_option::<TrustScore>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Deserialization error: {:?}",
                    e
                )))
            })? {
                scores.push(score);
            }
        }
    }

    Ok(scores)
}

/// Report spam/malicious message
/// This creates a negative report that feeds back into MATL
#[hdk_extern]
pub fn report_spam(input: SpamReportInput) -> ExternResult<()> {
    debug!(
        "Spam reported for message {:?}: {}",
        input.message_hash, input.reason
    );

    // Get the message to identify the sender
    let record = get(input.message_hash, GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Message not found".into())
    ))?;

    let message: MailMessage = record
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Deserialization error: {:?}",
                e
            )))
        })?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid message entry".into()
        )))?;

    let reporter = agent_info()?.agent_initial_pubkey;
    let report = SpamReport {
        reporter,
        spammer_did: message.from_did.clone(),
        message_hash: input.message_hash.clone(),
        reason: input.reason,
        reported_at: sys_time()?,
    };

    let report_hash = create_entry(EntryTypes::SpamReport(report))?;

    let path = Path::from("spam_reports");
    path.ensure()?;
    let path_hash = path.path_entry_hash()?;
    create_link(path_hash, report_hash, LinkTypes::SpamReports, ())?;

    debug!("Spam report registered for sender: {}", message.from_did);
    Ok(())
}

/// Retrieve spam reports recorded since a given timestamp
#[hdk_extern]
pub fn get_spam_reports(input: GetSpamReportsInput) -> ExternResult<Vec<SpamReport>> {
    let path = Path::from("spam_reports");
    let path_hash = match path.path_entry_hash() {
        Ok(hash) => hash,
        Err(_) => return Ok(Vec::new()),
    };

    let links =
        get_links(GetLinksInputBuilder::try_new(path_hash, LinkTypes::SpamReports)?.build())?;

    let mut reports = Vec::new();
    for link in links {
        let hash_any_dht: AnyDhtHash =
            ActionHash::from_raw_39(link.target.get_raw_39().to_vec()).into();
        if let Some(record) = get(hash_any_dht, GetOptions::default())? {
            if let Some(report) = record.entry().to_app_option::<SpamReport>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Deserialization error: {:?}",
                    e
                )))
            })? {
                if report.reported_at > input.since {
                    reports.push(report);
                }
            }
        }
    }

    reports.sort_by(|a, b| a.reported_at.cmp(&b.reported_at));
    Ok(reports)
}

// === Helper Functions ===

/// Calculate a simple local trust score based on message history
/// This is a fallback when MATL scores are not available
fn calculate_local_trust(did: &str) -> ExternResult<f64> {
    // In production, this would:
    // 1. Count total messages sent by this DID
    // 2. Count spam reports
    // 3. Calculate score = 1.0 - (spam_reports / total_messages)

    // For MVP, return neutral
    debug!("Calculating local trust for {} (MVP: returning 0.5)", did);
    Ok(0.5)
}
