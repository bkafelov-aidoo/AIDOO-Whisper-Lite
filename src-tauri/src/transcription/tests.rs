use super::{
    compact_api_message, encode_wav_to_flac, response_status_is_retry_safe, streamed_audio_part,
    transcribe_with_client, TranscriptionFailure, MAX_TRANSCRIPTION_FILE_BYTES,
};
use crate::models::AppSettings;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::Duration;

fn temporary_audio() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "aidoo-lite-http-transcription-test-{}.flac",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, b"fLaC-test-audio").unwrap();
    path
}

fn spawn_http_response(status: &str, body: &str, delay: Duration) -> (String, JoinHandle<()>) {
    spawn_response_with_type(status, "application/json", body, delay)
}

fn spawn_response_with_type(
    status: &str,
    content_type: &str,
    body: &str,
    delay: Duration,
) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_string();
    let content_type = content_type.to_string();
    let body = body.as_bytes().to_vec();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let _request = read_request(&mut stream);
        std::thread::sleep(delay);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&body);
    });
    (format!("http://{address}/v1/audio/transcriptions"), handle)
}

fn local_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()
        .unwrap()
}

fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut buffer).unwrap();
        assert!(read > 0, "request ended before headers");
        request.extend_from_slice(&buffer[..read]);
        if let Some(offset) = request.windows(4).position(|part| part == b"\r\n\r\n") {
            break offset + 4;
        }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    while request.len() - header_end < content_length {
        let read = stream.read(&mut buffer).unwrap();
        assert!(read > 0, "request ended before body");
        request.extend_from_slice(&buffer[..read]);
    }
    request
}

#[tokio::test]
async fn single_model_receives_dictionary_and_language_fields_on_one_reused_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/v1/audio/transcriptions",
        listener.local_addr().unwrap()
    );
    let server = std::thread::spawn(move || {
        // Exactly one TCP connection serves all four dictations.
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut requests = Vec::new();
        for _ in 0..4 {
            requests.push(String::from_utf8(read_request(&mut socket)).unwrap());
            let body = r#"{"text":"result"}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: keep-alive\r\n\r\n{body}", body.len()).unwrap();
            socket.flush().unwrap();
        }
        requests
    });
    let path = temporary_audio();
    let client = local_client(Duration::from_secs(3));
    for language in ["bg", "auto"] {
        for enabled in [true, false] {
            let settings = AppSettings {
                language: language.into(),
                model: crate::models::TRANSCRIPTION_MODEL.into(),
                vocabulary_enabled: enabled,
                vocabulary_terms: vec![" кофердам ".into(), "Кофердам".into(), "".into()],
                ..AppSettings::default()
            };
            assert_eq!(
                transcribe_with_client(&path, "sk-test-only", &settings, None, &client, &endpoint)
                    .await
                    .unwrap(),
                "result"
            );
        }
    }
    let requests = server.join().unwrap();
    for (index, request) in requests.iter().enumerate() {
        assert!(!request.contains("name=\"stream\""));
        assert!(request.contains("name=\"response_format\"\r\n\r\njson"));
        assert!(request.contains("name=\"model\"\r\n\r\ngpt-transcribe"));
        assert_eq!(request.contains("name=\"languages[]\""), index < 2);
        assert!(!request.contains("name=\"language\""));
        assert_eq!(request.contains("name=\"keywords[]\""), index % 2 == 0);
        assert!(!request.contains("name=\"prompt\""));
        assert_eq!(
            request.matches("кофердам").count(),
            usize::from(index % 2 == 0)
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn invalid_vocabulary_is_rejected_before_any_api_request() {
    let path = temporary_audio();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!(
        "http://{}/v1/audio/transcriptions",
        listener.local_addr().unwrap()
    );
    let settings = AppSettings {
        vocabulary_terms: vec!["<invalid>".into()],
        ..AppSettings::default()
    };
    let failure = transcribe_with_client(
        &path,
        "sk-test-only",
        &settings,
        None,
        &local_client(Duration::from_secs(2)),
        &endpoint,
    )
    .await
    .unwrap_err();
    assert_eq!(failure.message, crate::vocabulary::INVALID_VOCABULARY);
    assert!(failure.retryable);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        super::api_key_phase(&Err(failure)),
        None,
        "local vocabulary rejection must not change API-key health"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn api_error_messages_are_single_line_and_bounded() {
    let message = format!("  first\n\tsecond {}", "x".repeat(700));
    let compact = compact_api_message(&message);
    assert!(!compact.contains('\n'));
    assert_eq!(compact.chars().count(), 500);
    assert!(compact.ends_with('…'));
}

#[test]
fn retry_safety_distinguishes_preflight_and_ambiguous_api_failures() {
    assert!(
        !TranscriptionFailure::before_request(
            "Аудио файлът е по-голям от лимита на OpenAI от 25 MB. Направете по-кратък запис."
                .into()
        )
        .retryable
    );
    assert!(TranscriptionFailure::before_request("file unavailable".into()).retryable);
    assert!(response_status_is_retry_safe(
        reqwest::StatusCode::UNAUTHORIZED
    ));
    assert!(response_status_is_retry_safe(
        reqwest::StatusCode::TOO_MANY_REQUESTS
    ));
    assert!(!response_status_is_retry_safe(
        reqwest::StatusCode::REQUEST_TIMEOUT
    ));
    assert!(!response_status_is_retry_safe(
        reqwest::StatusCode::PAYLOAD_TOO_LARGE
    ));
    assert!(!response_status_is_retry_safe(
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    ));
}

#[tokio::test]
async fn oversized_audio_is_rejected_before_a_request_can_start() {
    let path = std::env::temp_dir().join(format!(
        "aidoo-lite-oversized-audio-test-{}.flac",
        uuid::Uuid::new_v4()
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_TRANSCRIPTION_FILE_BYTES + 1).unwrap();
    drop(file);

    let result = streamed_audio_part(&path, None).await;

    let error = match result {
        Ok(_) => panic!("an oversized file must not become a request body"),
        Err(error) => error,
    };
    assert!(error.starts_with(super::FILE_TOO_LARGE_PREFIX));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn exact_upload_limit_is_accepted_by_the_preflight_guard() {
    let path = std::env::temp_dir().join(format!(
        "aidoo-lite-exact-limit-audio-test-{}.flac",
        uuid::Uuid::new_v4()
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_TRANSCRIPTION_FILE_BYTES).unwrap();
    drop(file);

    assert!(streamed_audio_part(&path, None).await.is_ok());
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn ambiguous_http_outcomes_are_never_marked_retryable() {
    let path = temporary_audio();
    let settings = AppSettings::default();

    let (endpoint, server) = spawn_http_response(
        "500 Internal Server Error",
        r#"{"error":{"message":"temporary upstream failure"}}"#,
        Duration::ZERO,
    );
    let server_error = transcribe_with_client(
        &path,
        "sk-test-only",
        &settings,
        None,
        &local_client(Duration::from_secs(2)),
        &endpoint,
    )
    .await
    .unwrap_err();
    server.join().unwrap();
    assert!(!server_error.retryable);
    assert!(server_error.message.contains("temporary upstream failure"));

    let (endpoint, server) = spawn_http_response("200 OK", "{invalid-json", Duration::ZERO);
    let invalid_success = transcribe_with_client(
        &path,
        "sk-test-only",
        &settings,
        None,
        &local_client(Duration::from_secs(2)),
        &endpoint,
    )
    .await
    .unwrap_err();
    server.join().unwrap();
    assert!(!invalid_success.retryable);
    assert!(invalid_success.message.contains("невалиден отговор"));

    let (endpoint, server) = spawn_http_response(
        "200 OK",
        r#"{"text":"too late"}"#,
        Duration::from_millis(300),
    );
    let timeout = transcribe_with_client(
        &path,
        "sk-test-only",
        &settings,
        None,
        &local_client(Duration::from_millis(50)),
        &endpoint,
    )
    .await
    .unwrap_err();
    server.join().unwrap();
    assert!(!timeout.retryable);
    assert!(timeout.message.contains("Няма връзка с OpenAI"));

    std::fs::remove_file(path).unwrap();
}

#[test]
fn wav_is_encoded_as_flac() {
    let root = std::env::temp_dir().join(format!("aidoo-lite-flac-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let wav = root.join("sample.wav");
    let flac = root.join("sample.flac");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
    for i in 0..16_000 {
        writer
            .write_sample::<i16>(((i as f32 / 14.0).sin() * 4_000.0) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    encode_wav_to_flac(&wav, &flac).unwrap();
    assert!(std::fs::read(&flac).unwrap().starts_with(b"fLaC"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn accepted_successor_is_used_in_the_actual_upload() {
    let directory =
        std::env::temp_dir().join(format!("lite-successor-upload-{}", uuid::Uuid::new_v4()));
    crate::storage::with_test_data_dir(directory.clone(), || {
        let mut choice = crate::model_updates::default_model();
        choice.id = "gpt-test-transcribe".into();
        choice.rank = 2;
        choice.source = format!(
            "https://developers.openai.com/api/docs/models/{}",
            choice.id
        );
        crate::storage::save_model_choice(&choice).unwrap();
        let settings = crate::storage::load_settings();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/audio/transcriptions",
            listener.local_addr().unwrap()
        );
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let request = String::from_utf8(read_request(&mut socket)).unwrap();
            let body = r#"{"text":"result"}"#;
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            request
        });
        let audio = temporary_audio();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime
            .block_on(transcribe_with_client(
                &audio,
                "sk-test-only",
                &settings,
                None,
                &local_client(Duration::from_secs(3)),
                &endpoint,
            ))
            .unwrap();
        assert_eq!(result, "result");
        let request = server.join().unwrap();
        assert!(request.contains("name=\"model\"\r\n\r\ngpt-test-transcribe"));
        assert!(request.contains("name=\"languages[]\""));
        assert_eq!(settings.model, choice.id);
        std::fs::remove_file(audio).unwrap();
    });
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn every_file_model_uses_its_own_parameters_without_streaming_or_retries() {
    let models = crate::model_updates::bundled_models();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/audio/transcriptions",
        listener.local_addr().unwrap()
    );
    let count = models.len();
    let server = std::thread::spawn(move || {
        let mut requests = vec![];
        for _ in 0..count {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            requests.push(String::from_utf8(read_request(&mut socket)).unwrap());
            let body = r#"{"text":"result"}"#;
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    let path = temporary_audio();
    let client = local_client(Duration::from_secs(3));
    for model in &models {
        let settings = AppSettings {
            model: model.id.clone(),
            model_details: Some(model.clone()),
            vocabulary_terms: vec!["кофердам".into(), "Brinker".into()],
            ..AppSettings::default()
        };
        assert_eq!(
            transcribe_with_client(&path, "sk-test-only", &settings, None, &client, &endpoint)
                .await
                .unwrap(),
            "result"
        );
    }
    let requests = server.join().unwrap();
    for (model, request) in models.iter().zip(requests) {
        assert!(request.contains(&format!("name=\"model\"\r\n\r\n{}", model.id)));
        assert!(!request.contains("name=\"stream\""));
        assert_eq!(
            request.contains("name=\"languages[]\""),
            model.protocol == "gpt-transcribe-v1"
        );
        assert_eq!(
            request.contains("name=\"language\""),
            model.protocol != "gpt-transcribe-v1"
        );
        assert_eq!(
            request.contains("name=\"keywords[]\""),
            model.protocol == "gpt-transcribe-v1"
        );
        assert_eq!(
            request.contains("name=\"prompt\""),
            ["legacy-transcribe-v1", "whisper-v1"].contains(&model.protocol.as_str())
        );
        assert_eq!(
            request.contains("name=\"chunking_strategy\"\r\n\r\nauto"),
            model.protocol == "diarize-v1"
        );
        assert_eq!(
            request.contains("name=\"response_format\"\r\n\r\ndiarized_json"),
            model.protocol == "diarize-v1"
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn local_failure_leaves_health_untouched_but_api_failures_update_it() {
    for message in [
        "file unavailable",
        "Неподдържан модел.",
        crate::vocabulary::INVALID_VOCABULARY,
    ] {
        assert_eq!(
            super::api_key_phase(&Err(TranscriptionFailure::before_request(message.into()))),
            None
        );
    }
    let mut failure = TranscriptionFailure::after_request("quota".into(), true);
    failure.http_status = Some(429);
    assert_eq!(super::api_key_phase(&Err(failure)), Some("limited"));
    assert_eq!(
        super::api_key_phase(&Err(TranscriptionFailure::after_request(
            "timeout".into(),
            false
        ))),
        Some("unavailable")
    );
    assert_eq!(super::api_key_phase(&Ok("text".into())), Some("working"));
}
