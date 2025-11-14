use hdk::prelude::*;
use holochain_serialized_bytes::prelude::*;

/// Core mail message entry type
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MailMessage {
    pub from_did: String,
    pub to_did: String,
    pub subject_encrypted: Vec<u8>,
    pub body_cid: String, // IPFS content ID
    pub timestamp: Timestamp,
    pub thread_id: Option<String>,
    pub epistemic_tier: EpistemicTier,
}

/// Trust score for spam filtering
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrustScore {
    pub did: String,
    pub score: f64, // 0.0 - 1.0
    pub last_updated: Timestamp,
    pub matl_source: String,
}

/// Epistemic tiers from Mycelix Epistemic Charter v2.0
#[derive(Clone, PartialEq, Serialize, Deserialize, SerializedBytes, Debug)]
pub enum EpistemicTier {
    Tier0Null,
    Tier1Testimonial,
    Tier2PrivatelyVerifiable,
    Tier3CryptographicallyProven,
    Tier4PubliclyReproducible,
}

/// Contact entry for address book
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Contact {
    pub name: String,
    pub did: String,
    pub email_alias: Option<String>,
    pub notes: Option<String>,
    pub added_at: Timestamp,
}

/// Mapping between DID and the agent pubkey that owns it
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DidBinding {
    pub did: String,
    pub agent_pub_key: AgentPubKey,
}

/// Persisted spam report
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SpamReport {
    pub reporter: AgentPubKey,
    pub spammer_did: String,
    pub message_hash: ActionHash,
    pub reason: String,
    pub reported_at: Timestamp,
}

/// Entry types for the DNA
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type]
    MailMessage(MailMessage),
    #[entry_type]
    TrustScore(TrustScore),
    #[entry_type]
    Contact(Contact),
    #[entry_type]
    DidBinding(DidBinding),
    #[entry_type]
    SpamReport(SpamReport),
}

/// Link types for connecting entries
#[hdk_link_types]
pub enum LinkTypes {
    ToInbox,
    FromOutbox,
    ThreadReply,
    TrustByDid,
    TrustIndex,
    ContactLink,
    SpamReports,
    DidBindingLink,
}

/// Basic validation to guard against malformed data
#[hdk_extern]
pub fn validate(_op: Op) -> ExternResult<ValidateCallbackResult> {
    match _op {
        Op::StoreEntry(store_entry) => {
            match store_entry.entry {
                EntryTypes::TrustScore(score) => {
                    if !(0.0..=1.0).contains(&score.score) {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Trust score must be between 0.0 and 1.0".into(),
                        ));
                    }
                    if score.did.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Trust score DID cannot be empty".into(),
                        ));
                    }
                }
                EntryTypes::MailMessage(message) => {
                    if message.from_did.trim().is_empty() || message.to_did.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Message DIDs cannot be empty".into(),
                        ));
                    }
                    if message.subject_encrypted.is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Subject cannot be empty".into(),
                        ));
                    }
                    if message.body_cid.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Body CID cannot be empty".into(),
                        ));
                    }
                }
                EntryTypes::DidBinding(binding) => {
                    if binding.did.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "DID cannot be empty".into(),
                        ));
                    }
                }
                EntryTypes::SpamReport(report) => {
                    if report.spammer_did.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Spam report must include spammer DID".into(),
                        ));
                    }
                    if report.reason.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Spam report reason cannot be empty".into(),
                        ));
                    }
                }
                EntryTypes::Contact(contact) => {
                    if contact.did.trim().is_empty() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Contact DID cannot be empty".into(),
                        ));
                    }
                }
            }
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
