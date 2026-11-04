//! Model-specific request parameters, always one completed-file response.
use crate::{model_updates::CatalogModel, models::AppSettings};
use reqwest::multipart::{Form, Part};
pub fn form(audio: Part, model: &CatalogModel, settings: &AppSettings) -> Result<Form, String> {
    if !model.languages.contains(&settings.language) {
        return Err(
            "Моделът не поддържа този език. / This model does not support this language.".into(),
        );
    }
    let mut form = Form::new()
        .part("file", audio)
        .text("model", model.id.clone())
        .text(
            "response_format",
            if model.protocol == "diarize-v1" {
                "diarized_json"
            } else {
                "json"
            },
        );
    if settings.language != "auto" {
        form = form.text(
            if model.protocol == "gpt-transcribe-v1" {
                "languages[]"
            } else {
                "language"
            },
            settings.language.clone(),
        );
    }
    if model.protocol == "diarize-v1" {
        return Ok(form.text("chunking_strategy", "auto"));
    }
    if settings.vocabulary_enabled {
        let terms = crate::vocabulary::normalize(&settings.vocabulary_terms);
        crate::vocabulary::validate(&terms)?;
        if model.protocol == "gpt-transcribe-v1" {
            for term in terms {
                form = form.text("keywords[]", term);
            }
        } else {
            // Whisper uses only a short prompt window. 200 UTF-8 bytes remain below
            // 224 tokens even when every byte would be a separate token.
            let max_bytes = if model.protocol == "whisper-v1" {
                200
            } else {
                6000
            };
            let mut prompt = String::new();
            for term in terms {
                let separator = if prompt.is_empty() { "" } else { ", " };
                if prompt.len() + separator.len() + term.len() > max_bytes {
                    break;
                }
                prompt.push_str(separator);
                prompt.push_str(&term);
            }
            if !prompt.is_empty() {
                form = form.text("prompt", prompt);
            }
        }
    }
    Ok(form)
}
pub fn final_text(body: &[u8], protocol: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Transcript {
        text: String,
        #[serde(default)]
        segments: Vec<Segment>,
    }
    #[derive(serde::Deserialize)]
    struct Segment {
        text: String,
        speaker: String,
    }
    let result: Transcript =
        serde_json::from_slice(body).map_err(|_| "OpenAI върна невалиден отговор.".to_string())?;
    if protocol == "diarize-v1" && !result.segments.is_empty() {
        return Ok(result
            .segments
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .map(|s| format!("{}: {}", s.speaker, s.text.trim()))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(result.text.trim().to_string())
}
#[cfg(test)]
mod tests {
    #[test]
    fn final_speakers_are_not_duplicated_with_top_level_text() {
        let body = br#"{"text":"A: Hello B: Hi","segments":[{"speaker":"A","text":"Hello"},{"speaker":"B","text":"Hi"}]}"#;
        assert_eq!(
            super::final_text(body, "diarize-v1").unwrap(),
            "A: Hello\nB: Hi"
        );
        assert_eq!(
            super::final_text(br#"{"text":" Hello "}"#, "legacy-transcribe-v1").unwrap(),
            "Hello"
        );
        assert!(super::final_text(br#"{}"#, "whisper-v1").is_err());
    }
}
