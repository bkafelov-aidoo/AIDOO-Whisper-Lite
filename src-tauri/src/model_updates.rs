//! A curated data catalog, never executable configuration or an endpoint override.
use crate::models::{AppSettings, TRANSCRIPTION_MODEL};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::Duration;

pub const CATALOG_URL: &str = "https://raw.githubusercontent.com/bkafelov-aidoo/AIDOO-Whisper-Lite/main/catalog/transcription-models.json";
const MODELS_URL: &str = "https://api.openai.com/v1/models";
const CATALOG_LIMIT: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogModel {
    pub id: String,
    pub rank: u32,
    pub protocol: String,
    pub estimated_usd_per_minute: f64,
    pub languages: Vec<String>,
    pub benefit_bg: String,
    pub benefit_en: String,
    pub source: String,
}

impl CatalogModel {
    pub fn valid(&self) -> bool {
        self.id.starts_with("gpt-")
            && self.id.contains("transcribe")
            && self.id.len() <= 80
            && self
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'.')
            && self.protocol == "gpt-transcribe-v1"
            && (1..=1_000_000).contains(&self.rank)
            && self.estimated_usd_per_minute.is_finite()
            && (0.0..=1.0).contains(&self.estimated_usd_per_minute)
            && !self.languages.is_empty()
            && self.languages.len() <= 7
            && self
                .languages
                .iter()
                .all(|l| ["auto", "bg", "en", "de", "es", "fr", "it"].contains(&l.as_str()))
            && [&self.benefit_bg, &self.benefit_en].iter().all(|text| {
                !text.trim().is_empty()
                    && text.chars().count() <= 300
                    && !text.chars().any(char::is_control)
            })
            && self.source == format!("https://developers.openai.com/api/docs/models/{}", self.id)
    }
}

pub fn default_model() -> CatalogModel {
    // Audited and bundled; available even when the remote catalog cannot be read.
    serde_json::from_str::<Catalog>(include_str!("../../catalog/transcription-models.json"))
        .expect("bundled model catalog")
        .models
        .into_iter()
        .find(|model| model.id == TRANSCRIPTION_MODEL)
        .expect("bundled default model")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    models: Vec<CatalogModel>,
}

fn parse_catalog(bytes: &[u8]) -> Result<Catalog, String> {
    if bytes.len() > CATALOG_LIMIT {
        return Err("catalog too large".into());
    }
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|_| "invalid catalog")?;
    let mut ids = std::collections::HashSet::new();
    let mut ranks = std::collections::HashSet::new();
    if catalog.schema_version != 1
        || catalog.models.is_empty()
        || catalog.models.len() > 16
        || catalog
            .models
            .iter()
            .any(|m| !m.valid() || !ids.insert(&m.id) || !ranks.insert(m.rank))
    {
        return Err("incompatible catalog".into());
    }
    Ok(catalog)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProposal {
    #[serde(skip)]
    pub key_revision: u64,
    pub current_model: String,
    pub current_usd_per_minute: f64,
    pub candidate: CatalogModel,
}

#[derive(Default)]
pub struct ModelUpdateState {
    checked: tokio::sync::Mutex<bool>,
    pub key_revision: AtomicU64,
    pub proposal: Mutex<Option<ModelProposal>>,
}

async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err("request unavailable".into());
    }
    let mut result = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "request failed")?;
        if result.len().saturating_add(chunk.len()) > limit {
            return Err("response too large".into());
        }
        result.extend_from_slice(&chunk);
    }
    Ok(result)
}

fn choose_candidate(
    catalog: &Catalog,
    current: &CatalogModel,
    language: &str,
) -> Option<CatalogModel> {
    catalog
        .models
        .iter()
        .filter(|m| {
            m.rank > current.rank && m.id != current.id && m.languages.iter().any(|l| l == language)
        })
        .max_by_key(|m| m.rank)
        .cloned()
}

async fn discover(
    client: &reqwest::Client,
    catalog_url: &str,
    models_url: &str,
    api_key: &str,
    current: &CatalogModel,
    language: &str,
) -> Result<Option<ModelProposal>, String> {
    // Credentials are attached only to the fixed OpenAI URL, never to GitHub.
    let catalog = parse_catalog(
        &bounded_body(
            client
                .get(catalog_url)
                .send()
                .await
                .map_err(|_| "catalog unavailable")?,
            CATALOG_LIMIT,
        )
        .await?,
    )?;
    let Some(candidate) = choose_candidate(&catalog, current, language) else {
        return Ok(None);
    };
    let body = bounded_body(
        client
            .get(models_url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|_| "models unavailable")?,
        2_000_000,
    )
    .await?;
    #[derive(Deserialize)]
    struct Models {
        data: Vec<ModelId>,
    }
    #[derive(Deserialize)]
    struct ModelId {
        id: String,
    }
    let models: Models = serde_json::from_slice(&body).map_err(|_| "invalid model list")?;
    if !models.data.iter().any(|m| m.id == candidate.id) {
        return Ok(None);
    }
    Ok(Some(ModelProposal {
        key_revision: 0,
        current_model: current.id.clone(),
        current_usd_per_minute: current.estimated_usd_per_minute,
        candidate,
    }))
}

pub async fn check(app: &tauri::AppHandle) -> Option<ModelProposal> {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let key_revision = state.model_updates.key_revision.load(Ordering::Acquire);
    let key = state.api_key.lock().ok()?.clone()?;
    let settings = state.settings.lock().ok()?.clone();
    let current = crate::storage::load_model_choice().unwrap_or_else(default_model);
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let result = check_once(&state.model_updates, async {
        discover(
            &client,
            CATALOG_URL,
            MODELS_URL,
            &key,
            &current,
            &settings.language,
        )
        .await
        .map(|proposal| {
            proposal.map(|mut proposal| {
                proposal.key_revision = key_revision;
                proposal
            })
        })
    })
    .await;
    if state.model_updates.key_revision.load(Ordering::Acquire) != key_revision {
        *state.model_updates.proposal.lock().ok()? = None;
        return None;
    }
    result
}

async fn check_once(
    state: &ModelUpdateState,
    discovery: impl std::future::Future<Output = Result<Option<ModelProposal>, String>>,
) -> Option<ModelProposal> {
    let mut checked = state.checked.lock().await;
    if *checked {
        return state.proposal.lock().ok()?.clone();
    }
    *checked = true;
    let result = discovery.await.ok().flatten();
    *state.proposal.lock().ok()? = result.clone();
    result
}

pub fn invalidate_key(state: &ModelUpdateState) {
    state.key_revision.fetch_add(1, Ordering::AcqRel);
    if let Ok(mut proposal) = state.proposal.lock() {
        *proposal = None;
    }
}

pub fn approved_model_id() -> String {
    crate::storage::load_model_choice()
        .map(|model| model.id)
        .unwrap_or_else(|| TRANSCRIPTION_MODEL.into())
}

pub fn accepted_choice(
    proposal: &ModelProposal,
    requested: &str,
    settings: &AppSettings,
    key_revision: u64,
) -> Result<CatalogModel, String> {
    if proposal.key_revision != key_revision
        || proposal.candidate.id != requested
        || proposal.current_model != settings.model
        || !proposal.candidate.valid()
        || !proposal.candidate.languages.contains(&settings.language)
    {
        return Err(
            "Предложението за модел вече не е актуално. / The model proposal is no longer current."
                .into(),
        );
    }
    Ok(proposal.candidate.clone())
}

#[cfg(test)]
mod tests;
