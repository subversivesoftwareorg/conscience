use crate::error::{ConscienceError, Result};
use crate::export::SnapshotExport;

/// Send an allowlisted export to the dashboard. This is the only function
/// that talks to the server, and it accepts only the export type, so raw
/// snapshots or analyses cannot be uploaded by accident.
pub async fn push_export(
    endpoint: &str,
    payload: &SnapshotExport,
    api_key: Option<&str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/analyses", endpoint.trim_end_matches('/'));

    let mut request = client.post(&url).json(payload);

    if let Some(key) = api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| ConscienceError::Other(anyhow::anyhow!("Failed to push to {}: {}", url, e)))?;

    let status = response.status();
    if status.is_success() {
        eprintln!(
            "Pushed snapshot {} to {} ({})",
            payload.snapshot_id, url, status
        );
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(ConscienceError::Other(anyhow::anyhow!(
            "Dashboard returned {}: {}",
            status,
            body
        )))
    }
}
