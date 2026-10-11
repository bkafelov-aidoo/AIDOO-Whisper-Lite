//! No paid inference is used for a credential check. Listing models cannot prove balance.
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub phase: String,
    pub checked_at: Option<String>,
    pub last_success_at: Option<String>,
}
pub fn record(app: &tauri::AppHandle, phase: &str) {
    let state = app.state::<crate::AppState>();
    if let Ok(mut status) = state.api_key_status.lock() {
        status.checked_at = Some(chrono::Utc::now().to_rfc3339());
        status.phase = if phase == "verified" && status.last_success_at.is_some() {
            "working"
        } else {
            phase
        }
        .into();
        if phase == "working" {
            status.last_success_at = status.checked_at.clone();
        }
        let _ = app.emit("api-key:status", status.clone());
    };
}
pub fn reset(app: &tauri::AppHandle) {
    let state = app.state::<crate::AppState>();
    if let Ok(mut status) = state.api_key_status.lock() {
        *status = ApiKeyStatus::default();
        let _ = app.emit("api-key:status", status.clone());
    };
}
pub fn phase_for_status(status: u16) -> &'static str {
    match status {
        200..=299 => "verified",
        401 => "invalid",
        403 => "restricted",
        429 => "limited",
        _ => "unavailable",
    }
}
pub async fn probe_with_client(
    client: &reqwest::Client,
    url: &str,
    key: &str,
) -> (Option<Vec<String>>, &'static str) {
    let Ok(response) = client.get(url).bearer_auth(key).send().await else {
        return (None, "unavailable");
    };
    let phase = phase_for_status(response.status().as_u16());
    if phase != "verified" {
        return (None, phase);
    }
    #[derive(Deserialize)]
    struct Models {
        data: Vec<Model>,
    }
    #[derive(Deserialize)]
    struct Model {
        id: String,
    }
    use futures_util::StreamExt;
    let mut body = vec![];
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            return (None, "unavailable");
        };
        if body.len() + chunk.len() > 2_000_000 {
            return (None, "unavailable");
        }
        body.extend_from_slice(&chunk);
    }
    match serde_json::from_slice::<Models>(&body) {
        Ok(models) => (
            Some(models.data.into_iter().map(|m| m.id).collect()),
            "verified",
        ),
        Err(_) => (None, "unavailable"),
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn forbidden_offline_and_quota_are_not_a_working_key() {
        use super::phase_for_status;
        assert_eq!(phase_for_status(200), "verified");
        assert_eq!(phase_for_status(401), "invalid");
        assert_eq!(phase_for_status(403), "restricted");
        assert_eq!(phase_for_status(429), "limited");
        assert_eq!(phase_for_status(500), "unavailable");
    }
}
