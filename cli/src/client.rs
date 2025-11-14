use anyhow::{Context, Result};

use crate::config::Config;
use crate::types::*;

/// Client for interacting with Mycelix Mail system
///
/// Provides high-level methods for all mail operations, DID resolution,
/// and trust score management. Handles connections to:
/// - DID registry (HTTP)
/// - MATL bridge (HTTP)
///
/// NOTE: Phase C (Holochain integration) is BLOCKED pending proper API documentation.
/// Current implementation uses educational stubs that demonstrate the architecture.
pub struct MycellixClient {
    /// HTTP client for external services
    http_client: reqwest::Client,

    /// Configuration
    config: Config,

    /// Holochain conductor URL (for future Phase C)
    conductor_url: String,

    /// DID registry URL
    did_registry_url: String,

    /// MATL bridge URL
    matl_bridge_url: String,
}

impl MycellixClient {
    /// Create a new Mycelix client
    pub async fn new(
        conductor_url: &str,
        did_registry_url: &str,
        matl_bridge_url: &str,
        config: Config,
    ) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.conductor.timeout))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http_client,
            config,
            conductor_url: conductor_url.to_string(),
            did_registry_url: did_registry_url.to_string(),
            matl_bridge_url: matl_bridge_url.to_string(),
        })
    }

    //
    // ===== MAIL OPERATIONS (Stub - Phase C Pending) =====
    //

    /// Send a mail message
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `mail_messages::send_message`
    pub async fn send_message(
        &self,
        to_did: String,
        subject: Vec<u8>,
        body_cid: String,
        _thread_id: Option<String>,
        tier: EpistemicTier,
    ) -> Result<String> {
        println!("📧 [STUB] Would send message to {}", to_did);
        println!("   Subject: {} bytes (encrypted)", subject.len());
        println!("   Body CID: {}", body_cid);
        println!("   Tier: {}", tier);

        // Simulated message ID
        let message_id = format!("msg_stub_{}", chrono::Utc::now().timestamp());
        Ok(message_id)
    }

    /// Get inbox messages
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `mail_messages::get_inbox`
    pub async fn get_inbox(&self) -> Result<Vec<MailMessage>> {
        println!("📬 [STUB] Would fetch inbox from DHT");
        Ok(vec![])
    }

    /// Get sent messages
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `mail_messages::get_outbox`
    pub async fn get_sent(&self) -> Result<Vec<MailMessage>> {
        println!("📤 [STUB] Would fetch sent messages from DHT");
        Ok(vec![])
    }

    /// Get a specific message by ID
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `mail_messages::get_message`
    pub async fn get_message(&self, message_id: &str) -> Result<MailMessage> {
        println!("🔍 [STUB] Would fetch message {} from DHT", message_id);

        Ok(MailMessage {
            from_did: self
                .config
                .identity
                .did
                .clone()
                .unwrap_or_else(|| "did:mycelix:demo-sender".to_string()),
            to_did: self
                .config
                .identity
                .did
                .clone()
                .unwrap_or_else(|| "did:mycelix:demo-recipient".to_string()),
            subject_encrypted: format!("ENC:Demo message {}", message_id).into_bytes(),
            body_cid: format!("bafyrei{}", hex::encode(message_id.as_bytes())),
            timestamp: chrono::Utc::now().timestamp(),
            thread_id: None,
            epistemic_tier: EpistemicTier::Tier2PrivatelyVerifiable,
        })
    }

    /// Mark a message as read
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to update message metadata
    pub async fn mark_read(&self, message_id: &str) -> Result<()> {
        println!("✓ [STUB] Would mark message {} as read", message_id);
        Ok(())
    }

    /// Delete a message
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `mail_messages::delete_message`
    pub async fn delete_message(&self, message_id: &str) -> Result<()> {
        println!("🗑️  [STUB] Would delete message {}", message_id);
        Ok(())
    }

    /// Search messages
    ///
    /// TODO (Phase C): Implement DHT-level search or client-side filtering
    pub async fn search_messages(&self, query: &str) -> Result<Vec<MailMessage>> {
        println!("🔎 [STUB] Would search for: {}", query);
        Ok(vec![])
    }

    //
    // ===== TRUST SCORE OPERATIONS (Partial - MATL HTTP) =====
    //

    /// Get trust score for a DID
    ///
    /// Queries the MATL bridge for trust scores
    pub async fn get_trust_score(&self, did: String) -> Result<Option<TrustScore>> {
        let url = format!("{}/trust/{}", self.matl_bridge_url, did);

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let score: TrustScore = response.json().await
                    .context("Failed to parse trust score response")?;
                Ok(Some(score))
            }
            Ok(response) if response.status() == 404 => {
                Ok(None)  // No trust score exists
            }
            Ok(response) => {
                println!("⚠️  MATL bridge returned status: {}", response.status());
                Ok(None)
            }
            Err(_) => {
                // MATL bridge not available - use stub
                println!("📊 [STUB] MATL bridge unavailable, would check local cache for {}", did);
                Ok(None)
            }
        }
    }

    /// Set/update trust score for a DID
    ///
    /// TODO (Phase C): Sync to Holochain DHT via `trust_filter::update_trust_score`
    pub async fn set_trust_score(&self, did: String, score: f64) -> Result<()> {
        println!("⭐ [STUB] Would set trust score for {} to {:.2}", did, score);

        // TODO: Also update MATL bridge
        // POST to {}/trust with TrustScoreUpdate

        Ok(())
    }

    /// List all trust scores
    ///
    /// TODO (Phase C): Replace with real Holochain zome call to `trust_filter::get_all_trust_scores`
    pub async fn list_trust_scores(&self) -> Result<Vec<TrustScore>> {
        println!("📋 [STUB] Would list all trust scores from DHT");
        self.sync_trust_from_matl().await
    }

    /// Sync trust scores from MATL
    ///
    /// Fetches trust scores from MATL bridge and stores locally
    pub async fn sync_trust_from_matl(&self) -> Result<Vec<TrustScore>> {
        println!("🔄 [STUB] Would sync trust scores from MATL bridge");

        // TODO: Implement HTTP call to MATL bridge
        // For now, return a single neutral entry so downstream logic continues to work.
        let did = self
            .config
            .identity
            .did
            .clone()
            .unwrap_or_else(|| "did:mycelix:demo".to_string());

        let trust_score = TrustScore {
            did,
            score: 0.5,
            last_updated: chrono::Utc::now().timestamp(),
            source: "local-cache".to_string(),
        };

        Ok(vec![trust_score])
    }

    //
    // ===== DID OPERATIONS (HTTP to DID Registry) =====
    //

    /// Register a DID with the DID registry
    ///
    /// Maps DID to Holochain AgentPubKey for message routing
    pub async fn register_did(&self, did: String, agent_pub_key: String) -> Result<()> {
        let url = format!("{}/register", self.did_registry_url);

        let registration = serde_json::json!({
            "did": did,
            "agent_pub_key": agent_pub_key,
        });

        match self.http_client.post(&url)
            .json(&registration)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("✓ DID registered successfully");
                Ok(())
            }
            Ok(response) => {
                println!("⚠️  DID registry returned status: {}", response.status());
                println!("   [STUB] Would register locally");
                Ok(())
            }
            Err(_) => {
                println!("⚠️  DID registry not available");
                println!("   [STUB] Would register locally");
                Ok(())
            }
        }
    }

    /// Resolve a DID to an AgentPubKey
    ///
    /// Queries the DID registry to find the Holochain agent for a DID
    pub async fn resolve_did(&self, did: String) -> Result<Option<DidResolution>> {
        let url = format!("{}/resolve/{}", self.did_registry_url, did);

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let resolution: DidResolution = response.json().await
                    .context("Failed to parse DID resolution response")?;
                Ok(Some(resolution))
            }
            Ok(response) if response.status() == 404 => {
                Ok(None)  // DID not found
            }
            Ok(response) => {
                println!("⚠️  DID registry returned status: {}", response.status());
                Ok(None)
            }
            Err(_) => {
                println!("⚠️  DID registry not available");
                Ok(Some(self.stub_did_resolution(did)))
            }
        }
    }

    /// List all registered DIDs
    ///
    /// Queries the DID registry for all known DIDs
    pub async fn list_dids(&self) -> Result<Vec<DidResolution>> {
        let url = format!("{}/list", self.did_registry_url);

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let dids: Vec<DidResolution> = response.json().await
                    .context("Failed to parse DID list response")?;
                Ok(dids)
            }
            Ok(response) => {
                println!("⚠️  DID registry returned status: {}", response.status());
                Ok(vec![])
            }
            Err(_) => {
                println!("⚠️  DID registry not available");
                let fallback = self.stub_did_resolution(
                    self.config
                        .identity
                        .did
                        .clone()
                        .unwrap_or_else(|| "did:mycelix:demo".to_string()),
                );
                Ok(vec![fallback])
            }
        }
    }

    /// Get the current user's DID
    pub fn whoami(&self) -> Result<String> {
        self.config.identity.did.clone()
            .context("No DID configured. Run 'mycelix-mail init' first.")
    }

    //
    // ===== UTILITY OPERATIONS =====
    //

    /// Get mail statistics
    ///
    /// TODO (Phase C): Calculate from real DHT data
    pub async fn get_stats(&self) -> Result<MailStats> {
        println!("📊 [STUB] Would calculate stats from DHT");

        Ok(MailStats {
            total_messages: 0,
            unread_messages: 0,
            total_contacts: 0,
            total_trust_scores: 0,
            last_sync: Some(chrono::Utc::now().timestamp()),
        })
    }

    /// Health check - verify connections
    ///
    /// Checks connectivity to all external services
    pub async fn health_check(&self) -> Result<bool> {
        println!("🏥 Checking service health...");

        // Check DID registry
        let did_ok = match self.http_client.get(&format!("{}/health", self.did_registry_url))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("  ✓ DID Registry: OK");
                true
            }
            _ => {
                println!("  ✗ DID Registry: UNAVAILABLE");
                false
            }
        };

        // Check MATL bridge
        let matl_ok = match self.http_client.get(&format!("{}/health", self.matl_bridge_url))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("  ✓ MATL Bridge: OK");
                true
            }
            _ => {
                println!("  ✗ MATL Bridge: UNAVAILABLE");
                false
            }
        };

        // TODO (Phase C): Check Holochain conductor connection
        println!("  ⏳ Holochain Conductor: PENDING (Phase C)");

        Ok(did_ok || matl_ok)  // At least one service should be available
    }

    //
    // ===== GETTER METHODS =====
    //

    /// Get reference to configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Get the user's DID
    pub fn get_my_did(&self) -> Result<String> {
        self.whoami()
    }

    /// Get the user's agent public key
    pub fn get_my_agent_key(&self) -> Result<String> {
        self.config.identity.agent_pub_key.clone()
            .context("No agent key configured. Run 'mycelix-mail init' first.")
    }

    /// Get the conductor URL
    pub fn get_conductor_url(&self) -> &str {
        &self.conductor_url
    }

    /// Get the DID registry URL
    pub fn get_did_registry_url(&self) -> &str {
        &self.did_registry_url
    }

    /// Get the MATL bridge URL
    pub fn get_matl_bridge_url(&self) -> &str {
        &self.matl_bridge_url
    }

    /// Sync a single trust score (alias for set_trust_score)
    pub async fn sync_trust_score(&self, did: String) -> Result<TrustScore> {
        if let Some(score) = self.get_trust_score(did.clone()).await? {
            return Ok(score);
        }

        Ok(TrustScore {
            did,
            score: 0.5,
            last_updated: chrono::Utc::now().timestamp(),
            source: "local-cache".to_string(),
        })
    }

    /// Sync all trust scores from MATL (alias for sync_trust_from_matl)
    pub async fn sync_all_trust_scores(&self) -> Result<Vec<TrustScore>> {
        self.sync_trust_from_matl().await
    }

    fn stub_did_resolution(&self, did: String) -> DidResolution {
        let agent_key = self
            .config
            .identity
            .agent_pub_key
            .clone()
            .unwrap_or_else(|| "uhCAkDemoAgentKey".to_string());
        let now = chrono::Utc::now().timestamp();

        DidResolution {
            did,
            agent_pub_key: agent_key,
            created_at: now,
            updated_at: now,
        }
    }
}
