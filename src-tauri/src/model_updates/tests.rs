use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

fn successor() -> CatalogModel {
    let mut model = default_model();
    model.id = "gpt-test-transcribe".into();
    model.source = format!("https://developers.openai.com/api/docs/models/{}", model.id);
    model.rank = 2;
    model
}
fn catalog_bytes(model: &CatalogModel) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schemaVersion": 1, "models": [default_model(), model]}))
        .unwrap()
}

#[test]
fn bundled_catalog_is_valid_and_has_no_upgrade() {
    let catalog =
        parse_catalog(include_bytes!("../../../catalog/transcription-models.json")).unwrap();
    assert!(choose_candidate(&catalog, &default_model(), "bg").is_none());
    assert!(default_model().valid());
}

#[test]
fn only_compatible_curated_successors_are_offered() {
    let mut newer = successor();
    let catalog = parse_catalog(&catalog_bytes(&newer)).unwrap();
    assert_eq!(
        choose_candidate(&catalog, &default_model(), "bg")
            .unwrap()
            .id,
        newer.id
    );
    assert!(choose_candidate(&catalog, &newer, "bg").is_none());
    newer.languages = vec!["en".into()];
    let catalog = parse_catalog(&catalog_bytes(&newer)).unwrap();
    assert!(choose_candidate(&catalog, &default_model(), "bg").is_none());
    newer.protocol = "unknown-upload-format".into();
    assert!(parse_catalog(&catalog_bytes(&newer)).is_err());
}

#[test]
fn malformed_or_ambiguous_catalogs_fail_closed() {
    assert!(parse_catalog(&vec![b' '; CATALOG_LIMIT + 1]).is_err());
    assert!(parse_catalog(br#"{"schemaVersion":2,"models":[]}"#).is_err());
    assert!(parse_catalog(&catalog_bytes(&default_model())).is_err());
    let mut newer = successor();
    newer.source = "https://untrusted.example/model".into();
    assert!(parse_catalog(&catalog_bytes(&newer)).is_err());
    newer = successor();
    newer.estimated_usd_per_minute = -1.0;
    assert!(parse_catalog(&catalog_bytes(&newer)).is_err());
}

#[test]
fn consent_binds_exact_proposal_and_survives_restart_without_changing_preferences() {
    let directory = std::env::temp_dir().join(format!("lite-model-test-{}", uuid::Uuid::new_v4()));
    crate::storage::with_test_data_dir(directory.clone(), || {
        let settings = AppSettings {
            language: "en".into(),
            vocabulary_terms: vec!["Brinker".into()],
            auto_paste: false,
            ..AppSettings::default()
        };
        crate::storage::save_settings(&settings).unwrap();
        let proposal = ModelProposal {
            key_revision: 0,
            current_model: settings.model.clone(),
            current_usd_per_minute: 0.0045,
            candidate: successor(),
        };
        assert_eq!(approved_model_id(), TRANSCRIPTION_MODEL); // Discovery alone has no effect.
        assert!(accepted_choice(&proposal, "gpt-unapproved-transcribe", &settings, 0).is_err());
        let mut stale = settings.clone();
        stale.model = "other-model".into();
        assert!(accepted_choice(&proposal, &proposal.candidate.id, &stale, 0).is_err());
        assert!(accepted_choice(&proposal, &proposal.candidate.id, &settings, 1).is_err());
        let choice = accepted_choice(&proposal, &proposal.candidate.id, &settings, 0).unwrap();
        crate::storage::save_model_choice(&choice).unwrap();
        let restarted = crate::storage::load_settings();
        assert_eq!(restarted.model, choice.id);
        assert_eq!(restarted.language, "en");
        assert_eq!(restarted.vocabulary_terms, vec!["Brinker"]);
        assert!(!restarted.auto_paste);
        std::fs::write(directory.join("model-choice.json"), b"invalid").unwrap();
        assert_eq!(crate::storage::load_settings().model, TRANSCRIPTION_MODEL);
    });
    std::fs::remove_dir_all(directory).unwrap();
}

// Local mock only: never transcribes audio, never uses a real API credential.
async fn discovery_case(status: u16, model_list: &str) -> (Option<ModelProposal>, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let catalog = String::from_utf8(catalog_bytes(&successor())).unwrap();
    let replies = [(200, catalog), (status, model_list.to_owned())];
    let server = std::thread::spawn(move || {
        let mut requests = vec![];
        for (status, body) in replies {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
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
    let state = ModelUpdateState::default();
    let current = default_model();
    let catalog_url = format!("{root}/catalog");
    let models_url = format!("{root}/models");
    let first = check_once(
        &state,
        discover(
            &client,
            &catalog_url,
            &models_url,
            "sk-test-only",
            &current,
            "bg",
        ),
    );
    let second = check_once(
        &state,
        discover(
            &client,
            &catalog_url,
            &models_url,
            "sk-test-only",
            &current,
            "bg",
        ),
    );
    let (result, repeated) = tokio::join!(first, second);
    assert_eq!(
        result.as_ref().map(|p| &p.candidate.id),
        repeated.as_ref().map(|p| &p.candidate.id)
    );
    (result, server.join().unwrap())
}

#[tokio::test]
async fn candidate_requires_account_availability_and_github_receives_no_key() {
    let (proposal, requests) =
        discovery_case(200, r#"{"data":[{"id":"gpt-test-transcribe"}]}"#).await;
    assert_eq!(proposal.unwrap().candidate.id, "gpt-test-transcribe");
    assert!(!requests[0].to_lowercase().contains("authorization"));
    assert!(!requests[0].contains("sk-test-only"));
    assert!(requests[1].contains("Bearer sk-test-only"));
    for (status, body) in [
        (200, r#"{"data":[]}"#),
        (403, "permission denied"),
        (200, "invalid JSON"),
    ] {
        assert!(discovery_case(status, body).await.0.is_none());
    }
}
