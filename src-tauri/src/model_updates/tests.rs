use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
#[test]
fn bundled_models_cover_all_completed_file_protocols() {
    let models = bundled_models();
    assert_eq!(models.len(), 7);
    for id in [
        "gpt-transcribe",
        "gpt-4o-transcribe",
        "gpt-4o-mini-transcribe",
        "gpt-4o-transcribe-diarize",
        "whisper-1",
    ] {
        assert!(models
            .iter()
            .any(|m| m.id == id && m.valid() && m.languages.contains(&"bg".into())));
    }
    assert!(language_supported("ja"));
    assert!(language_supported("ar"));
    assert!(!language_supported("xyz"));
}
#[test]
fn malformed_catalogs_and_protocol_id_mismatches_fail_closed() {
    assert!(parse_catalog(&vec![b' '; CATALOG_LIMIT + 1]).is_err());
    assert!(parse_catalog(br#"{"schemaVersion":99,"models":[]}"#).is_err());
    let mut model = default_model();
    model.protocol = "legacy-transcribe-v1".into();
    assert!(!model.valid());
    model = default_model();
    model.estimated_usd_per_minute = -1.0;
    assert!(!model.valid());
    model = default_model();
    model.source = "https://untrusted.example/model".into();
    assert!(!model.valid());
    let duplicates = serde_json::to_vec(
        &serde_json::json!({"schemaVersion":2,"models":[default_model(),default_model()]}),
    )
    .unwrap();
    assert!(parse_catalog(&duplicates).is_err());
}
#[test]
fn selecting_model_persists_exact_rate_and_preserves_other_preferences() {
    let root = std::env::temp_dir().join(format!("lite-selector-{}", uuid::Uuid::new_v4()));
    crate::storage::with_test_data_dir(root.clone(), || {
        let state = ModelUpdateState::default();
        let mut settings = AppSettings {
            language: "ja".into(),
            auto_paste: false,
            vocabulary_terms: vec!["Brinker".into()],
            ..AppSettings::default()
        };
        let model = resolve_selection(&state, "whisper-1", "ja", &settings).unwrap();
        settings.model = model.id.clone();
        settings.model_details = Some(model);
        settings.normalize();
        crate::storage::save_settings(&settings).unwrap();
        let saved = crate::storage::load_settings();
        assert_eq!(saved.model, "whisper-1");
        assert_eq!(saved.model_details.unwrap().estimated_usd_per_minute, 0.006);
        assert_eq!(saved.language, "ja");
        assert!(!saved.auto_paste);
        assert_eq!(saved.vocabulary_terms, vec!["Brinker"]);
        assert!(resolve_selection(&state, "unapproved-model", "bg", &settings).is_err());
        assert!(resolve_selection(&state, "whisper-1", "yue", &settings).is_err());
        *state.catalog.lock().unwrap() = vec![ModelOption {
            model: default_model(),
            available: Some(false),
            is_new: false,
        }];
        assert!(resolve_selection(&state, "gpt-transcribe", "bg", &settings).is_err());
    });
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn old_approved_choice_migrates_but_never_overrides_new_selection() {
    let root = std::env::temp_dir().join(format!("lite-migration-{}", uuid::Uuid::new_v4()));
    crate::storage::with_test_data_dir(root.clone(), || {
        crate::storage::save_model_choice(&bundled_model("whisper-1").unwrap()).unwrap();
        assert_eq!(crate::storage::load_settings().model, "whisper-1");
        let settings = AppSettings::default();
        crate::storage::save_settings(&settings).unwrap();
        assert_eq!(crate::storage::load_settings().model, TRANSCRIPTION_MODEL);
    });
    std::fs::remove_dir_all(root).unwrap();
}
async fn discovery_case(
    status: u16,
    model_list: &str,
) -> (CatalogReport, &'static str, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let mut newer = default_model();
    newer.id = "gpt-test-transcribe".into();
    newer.source = format!("https://developers.openai.com/api/docs/models/{}", newer.id);
    let body =
        serde_json::to_string(&serde_json::json!({"schemaVersion":2,"models":[newer]})).unwrap();
    let replies = [(200, body), (status, model_list.to_owned())];
    let server = std::thread::spawn(move || {
        let mut requests = vec![];
        for (status, body) in replies {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = vec![];
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            requests.push(String::from_utf8(request).unwrap());
            write!(
                socket,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let (report, health) = discover(
        &client,
        &format!("{root}/catalog"),
        &format!("{root}/models"),
        "sk-test-only",
    )
    .await;
    (report, health, server.join().unwrap())
}
#[tokio::test]
async fn availability_is_account_specific_and_github_never_receives_the_key() {
    let (report, health, requests) = discovery_case(
        200,
        r#"{"data":[{"id":"gpt-test-transcribe"},{"id":"whisper-1"}]}"#,
    )
    .await;
    assert_eq!(health, "verified");
    assert_eq!(report.models.len(), 8);
    let newer = report
        .models
        .iter()
        .find(|m| m.model.id == "gpt-test-transcribe")
        .unwrap();
    assert_eq!(newer.available, Some(true));
    assert!(newer.is_new);
    assert!(!requests[0].to_lowercase().contains("authorization"));
    assert!(!requests[0].contains("sk-test-only"));
    assert!(requests[1].contains("Bearer sk-test-only"));
    let (report, health, _) = discovery_case(403, "restricted").await;
    assert_eq!(health, "restricted");
    assert!(report.models.iter().all(|m| m.available.is_none()));
    let (_, health, _) = discovery_case(401, "revoked").await;
    assert_eq!(health, "invalid");
    let (_, health, _) = discovery_case(200, "invalid JSON").await;
    assert_eq!(health, "unavailable");
}

#[tokio::test]
async fn concurrent_startup_checks_share_one_discovery_and_key_changes_discard_old_results() {
    let state = ModelUpdateState::default();
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let fetch = || async {
        calls.fetch_add(1, Ordering::AcqRel);
        tokio::task::yield_now().await;
        (
            CatalogReport {
                models: vec![],
                catalog_updated: true,
            },
            "verified",
        )
    };
    let (a, b) = tokio::join!(
        cached_discover(&state, 0, fetch()),
        cached_discover(&state, 0, fetch())
    );
    assert!(a.is_some() && b.is_some());
    assert_eq!(calls.load(Ordering::Acquire), 1);
    invalidate_key(&state);
    let stale = cached_discover(&state, 1, async {
        invalidate_key(&state);
        (
            CatalogReport {
                models: vec![],
                catalog_updated: true,
            },
            "invalid",
        )
    })
    .await;
    assert!(stale.is_none());
    let fresh = cached_discover(&state, 2, fetch()).await.unwrap();
    assert_eq!(fresh.1, Some("verified"));
    assert_eq!(calls.load(Ordering::Acquire), 2);
}
