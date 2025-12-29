//! AgentAuri API client for MCP server

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// API client for AgentAuri backend
pub struct AgentAuriClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl AgentAuriClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    fn build_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);

        if let Some(ref key) = self.api_key {
            req = req.header("X-API-Key", key);
        }

        req
    }

    /// List triggers for the authenticated user
    pub async fn list_triggers(
        &self,
        page: Option<i32>,
        per_page: Option<i32>,
    ) -> Result<TriggerListResponse> {
        let mut req = self.build_request(reqwest::Method::GET, "/api/v1/triggers");

        if let Some(p) = page {
            req = req.query(&[("page", p.to_string())]);
        }
        if let Some(pp) = per_page {
            req = req.query(&[("per_page", pp.to_string())]);
        }

        let response = req.send().await.context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse trigger list response")
    }

    /// Get a specific trigger by ID
    pub async fn get_trigger(&self, trigger_id: &str) -> Result<TriggerResponse> {
        let path = format!("/api/v1/triggers/{}", trigger_id);
        let response = self
            .build_request(reqwest::Method::GET, &path)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse trigger response")
    }

    /// Create a new trigger
    pub async fn create_trigger(&self, request: CreateTriggerRequest) -> Result<TriggerResponse> {
        let response = self
            .build_request(reqwest::Method::POST, "/api/v1/triggers")
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse create trigger response")
    }

    /// Delete a trigger
    pub async fn delete_trigger(&self, trigger_id: &str) -> Result<()> {
        let path = format!("/api/v1/triggers/{}", trigger_id);
        let response = self
            .build_request(reqwest::Method::DELETE, &path)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        Ok(())
    }

    /// List linked agents
    pub async fn list_linked_agents(&self) -> Result<AgentListResponse> {
        let response = self
            .build_request(reqwest::Method::GET, "/api/v1/agents/linked")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse agent list response")
    }

    /// List followed agents
    pub async fn list_following(&self) -> Result<FollowingListResponse> {
        let response = self
            .build_request(reqwest::Method::GET, "/api/v1/agents/following")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse following list response")
    }

    /// Get Ponder events (blockchain events)
    pub async fn get_ponder_events(
        &self,
        event_type: Option<&str>,
        limit: Option<i32>,
    ) -> Result<PonderEventsResponse> {
        let mut req = self.build_request(reqwest::Method::GET, "/api/v1/ponder/events");

        if let Some(et) = event_type {
            req = req.query(&[("event_type", et)]);
        }
        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string())]);
        }

        let response = req.send().await.context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse ponder events response")
    }

    /// Get Ponder indexer status
    pub async fn get_ponder_status(&self) -> Result<PonderStatusResponse> {
        let response = self
            .build_request(reqwest::Method::GET, "/api/v1/ponder/status")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse ponder status response")
    }

    /// Get credit balance
    pub async fn get_credits(&self) -> Result<CreditBalanceResponse> {
        let response = self
            .build_request(reqwest::Method::GET, "/api/v1/billing/credits")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse credits response")
    }

    /// List organizations
    pub async fn list_organizations(&self) -> Result<OrganizationListResponse> {
        let response = self
            .build_request(reqwest::Method::GET, "/api/v1/organizations")
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse organizations response")
    }
}

// Response types

#[derive(Debug, Deserialize, Serialize)]
pub struct TriggerListResponse {
    pub data: Vec<TriggerResponse>,
    #[serde(default)]
    pub meta: Option<PaginationMeta>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TriggerResponse {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub registry: String,
    pub event_type: String,
    pub chain_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginationMeta {
    pub page: i32,
    pub per_page: i32,
    pub total: i64,
    pub total_pages: i32,
}

#[derive(Debug, Serialize)]
pub struct CreateTriggerRequest {
    pub name: String,
    pub registry: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentListResponse {
    pub data: Vec<AgentResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub agent_id: String,
    pub registry: String,
    pub chain_id: String,
    pub owner_address: String,
    pub linked_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FollowingListResponse {
    pub data: Vec<FollowingResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FollowingResponse {
    pub id: String,
    pub agent_id: String,
    pub registry: String,
    pub chain_id: String,
    pub followed_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PonderEventsResponse {
    pub events: Vec<Value>,
    pub total: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PonderStatusResponse {
    pub status: String,
    pub chains: Vec<ChainStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainStatus {
    pub chain_id: String,
    pub name: String,
    pub block_number: i64,
    pub syncing: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreditBalanceResponse {
    pub balance: i64,
    pub currency: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrganizationListResponse {
    pub data: Vec<OrganizationResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub created_at: String,
}
