//! Curated model data and account discovery. Never an endpoint override.
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
const CATALOG_LIMIT: usize = 131_072;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogModel {
    pub id: String,
    pub rank: u32,
    pub protocol: String,
    pub estimated_usd_per_minute: f64,
    #[serde(default = "duration_billing")]
    pub billing: String,
    #[serde(default = "legacy_pricing_date")]
    pub pricing_checked_at: String,
    pub languages: Vec<String>,
    pub benefit_bg: String,
    pub benefit_en: String,
    pub source: String,
}
fn duration_billing() -> String {
    "duration".into()
}
fn legacy_pricing_date() -> String {
    "2026-10-11".into()
}
pub fn language_supported(code: &str) -> bool {
    #[derive(Deserialize)]
    struct Language {
        code: String,
    }
    static CODES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let codes = CODES.get_or_init(|| {
        serde_json::from_str::<Vec<Language>>(include_str!("../../catalog/languages.json"))
            .expect("bundled languages")
            .into_iter()
            .map(|l| l.code)
            .collect()
    });
    ["auto", "yue", "zh-cn", "zh-tw", "zh-hk"].contains(&code) || codes.iter().any(|l| l == code)
}
impl CatalogModel {
    pub fn valid(&self) -> bool {
        let base = if self.id.starts_with("gpt-4o-mini-transcribe-") {
            "gpt-4o-mini-transcribe"
        } else {
            &self.id
        };
        let protocol_matches = match self.protocol.as_str() {
            "gpt-transcribe-v1" => {
                self.id.starts_with("gpt-")
                    && self.id.contains("transcribe")
                    && !self.id.starts_with("gpt-4o")
            }
            "legacy-transcribe-v1" => {
                ["gpt-4o-transcribe", "gpt-4o-mini-transcribe"].contains(&base)
            }
            "diarize-v1" => self.id == "gpt-4o-transcribe-diarize",
            "whisper-v1" => self.id == "whisper-1",
            _ => false,
        };
        protocol_matches
            && self.id.len() <= 80
            && self
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'.')
            && (1..=1_000_000).contains(&self.rank)
            && self.estimated_usd_per_minute.is_finite()
            && (0.0..=1.0).contains(&self.estimated_usd_per_minute)
            && ["tokens", "duration"].contains(&self.billing.as_str())
            && self.pricing_checked_at.len() == 10
            && chrono::NaiveDate::parse_from_str(&self.pricing_checked_at, "%Y-%m-%d").is_ok()
            && !self.languages.is_empty()
            && self.languages.len() <= 110
            && self.languages.iter().all(|l| language_supported(l))
            && [&self.benefit_bg, &self.benefit_en].iter().all(|t| {
                !t.trim().is_empty() && t.chars().count() <= 300 && !t.chars().any(char::is_control)
            })
            && self.source == format!("https://developers.openai.com/api/docs/models/{base}")
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    models: Vec<CatalogModel>,
}
fn parse_catalog(bytes: &[u8]) -> Result<Vec<CatalogModel>, String> {
    if bytes.len() > CATALOG_LIMIT {
        return Err("catalog too large".into());
    }
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|_| "invalid catalog")?;
    let mut ids = std::collections::HashSet::new();
    if ![1, 2].contains(&catalog.schema_version)
        || catalog.models.is_empty()
        || catalog.models.len() > 32
        || catalog
            .models
            .iter()
            .any(|m| !m.valid() || !ids.insert(&m.id))
    {
        return Err("incompatible catalog".into());
    }
    Ok(catalog.models)
}
pub fn bundled_models() -> Vec<CatalogModel> {
    parse_catalog(include_bytes!("../../catalog/transcription-models.json"))
        .expect("bundled model catalog")
}
pub fn default_model() -> CatalogModel {
    bundled_models()
        .into_iter()
        .find(|m| m.id == TRANSCRIPTION_MODEL)
        .expect("default model")
}
pub fn bundled_model(id: &str) -> Option<CatalogModel> {
    bundled_models().into_iter().find(|m| m.id == id)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    #[serde(flatten)]
    pub model: CatalogModel,
    pub available: Option<bool>,
    pub is_new: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReport {
    pub models: Vec<ModelOption>,
    pub catalog_updated: bool,
}
#[derive(Default)]
pub struct ModelUpdateState {
    cached: tokio::sync::Mutex<Option<(u64, CatalogReport)>>,
    pub key_revision: AtomicU64,
    pub catalog: Mutex<Vec<ModelOption>>,
}
async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err("request unavailable".into());
    }
    let mut body = vec![];
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "request failed")?;
        if body.len() + chunk.len() > limit {
            return Err("response too large".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
async fn discover(
    client: &reqwest::Client,
    catalog_url: &str,
    models_url: &str,
    key: &str,
) -> (CatalogReport, &'static str) {
    let remote = async {
        parse_catalog(
            &bounded_body(
                client
                    .get(catalog_url)
                    .send()
                    .await
                    .map_err(|_| "catalog unavailable")?,
                CATALOG_LIMIT,
            )
            .await?,
        )
    }
    .await
    .ok();
    let catalog_updated = remote.is_some();
    // Bundled compatible models stay selectable when an older online catalog is present.
    let mut models = bundled_models();
    if let Some(remote) = remote {
        for m in remote {
            if let Some(old) = models.iter_mut().find(|old| old.id == m.id) {
                *old = m;
            } else {
                models.push(m);
            }
        }
    }
    let (ids, health) = crate::api_key_status::probe_with_client(client, models_url, key).await;
    let models = models
        .into_iter()
        .map(|model| ModelOption {
            available: ids.as_ref().map(|ids| ids.contains(&model.id)),
            is_new: bundled_model(&model.id).is_none(),
            model,
        })
        .collect();
    (
        CatalogReport {
            models,
            catalog_updated,
        },
        health,
    )
}
pub async fn check(app: &tauri::AppHandle) -> CatalogReport {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let fallback = || CatalogReport {
        models: bundled_models()
            .into_iter()
            .map(|model| ModelOption {
                model,
                available: None,
                is_new: false,
            })
            .collect(),
        catalog_updated: false,
    };
    let revision = state.model_updates.key_revision.load(Ordering::Acquire);
    let Some(key) = state.api_key.lock().ok().and_then(|k| k.clone()) else {
        return fallback();
    };
    let Ok(client) = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .build()
    else {
        return fallback();
    };
    let Some((report, health)) = cached_discover(
        &state.model_updates,
        revision,
        discover(&client, CATALOG_URL, MODELS_URL, &key),
    )
    .await
    else {
        return fallback();
    };
    // Bind results to the same credential while publishing them. Key replacement
    // cannot race a stale check into the new account's status or availability.
    let Ok(current_key) = state.api_key.lock() else {
        return fallback();
    };
    if current_key.as_deref().map(|s| s.as_str()) != Some(key.as_str()) {
        return fallback();
    }
    if let Ok(mut catalog) = state.model_updates.catalog.lock() {
        if state.model_updates.key_revision.load(Ordering::Acquire) != revision {
            return fallback();
        }
        *catalog = report.models.clone();
        if let Some(health) = health {
            crate::api_key_status::record(app, health);
        }
    }

    report
}
async fn cached_discover(
    state: &ModelUpdateState,
    revision: u64,
    discovery: impl std::future::Future<Output = (CatalogReport, &'static str)>,
) -> Option<(CatalogReport, Option<&'static str>)> {
    let mut cache = state.cached.lock().await;
    if let Some((rev, report)) = cache.as_ref() {
        if *rev == revision {
            return Some((report.clone(), None));
        }
    }
    let (report, health) = discovery.await;
    if state.key_revision.load(Ordering::Acquire) != revision {
        return None;
    }
    *cache = Some((revision, report.clone()));
    Some((report, Some(health)))
}
pub fn invalidate_key(state: &ModelUpdateState) {
    state.key_revision.fetch_add(1, Ordering::AcqRel);
    if let Ok(mut options) = state.catalog.lock() {
        options.clear();
    }
}
pub fn resolve_selection(
    state: &ModelUpdateState,
    requested: &str,
    language: &str,
    current: &AppSettings,
) -> Result<CatalogModel, String> {
    let options = state
        .catalog
        .lock()
        .map_err(|_| "Model catalog unavailable")?;
    let option = options.iter().find(|o| o.model.id == requested);
    if option.is_some_and(|o| o.available == Some(false)) && requested != current.model {
        return Err("Моделът не е достъпен с този API ключ. / This model is not available with this API key.".into());
    }
    let model = option
        .map(|o| o.model.clone())
        .or_else(|| bundled_model(requested))
        .or_else(|| {
            current
                .model_details
                .clone()
                .filter(|m| m.id == requested && m.valid())
        })
        .ok_or("Неподдържан модел. / Unsupported model.")?;
    if !model.languages.iter().any(|l| l == language) {
        return Err(
            "Моделът не поддържа този език. / This model does not support this language.".into(),
        );
    }
    Ok(model)
}
#[cfg(test)]
mod tests;
