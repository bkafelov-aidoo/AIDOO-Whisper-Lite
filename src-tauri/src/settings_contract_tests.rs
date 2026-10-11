//! Cross-language IPC regression: actual React picker output must decode at the command seam.
use crate::models::AppSettings;

#[test]
fn frontend_model_selection_decodes_as_update_settings_arguments() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let fixture = std::env::temp_dir().join(format!(
        "aidoo-settings-contract-{}.json",
        uuid::Uuid::new_v4()
    ));
    let generated = std::process::Command::new("node")
        .current_dir(root)
        .arg("scripts/check-settings-contract.mjs")
        .arg("--capture-only")
        .arg("--output")
        .arg(&fixture)
        .output()
        .expect("Node must be available for the frontend/native contract test");
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let samples: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(&fixture).unwrap()).unwrap();
    std::fs::remove_file(&fixture).unwrap();
    assert!(samples.len() >= 14);
    for payload in samples {
        let expected_id = payload["model"].as_str().unwrap().to_owned();
        let settings: AppSettings = serde_json::from_value(payload)
            .expect("update_settings must accept the actual model-picker payload");
        assert_eq!(settings.model, expected_id);
        assert_eq!(settings.model_details.unwrap().id, expected_id);
    }
}
