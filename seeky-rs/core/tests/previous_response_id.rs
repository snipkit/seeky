use std::time::Duration;

use seeky_core::ModelProviderInfo;
use seeky_core::Seeky;
use seeky_core::exec::SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR;
use seeky_core::protocol::ErrorEvent;
use seeky_core::protocol::EventMsg;
use seeky_core::protocol::InputItem;
use seeky_core::protocol::Op;
mod test_support;
use serde_json::Value;
use tempfile::TempDir;
use test_support::load_default_config_for_test;
use tokio::time::timeout;
use wiremock::Match;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::Request;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

/// Matcher asserting that JSON body has NO `previous_response_id` field.
struct NoPrevId;

impl Match for NoPrevId {
    fn matches(&self, req: &Request) -> bool {
        serde_json::from_slice::<Value>(&req.body)
            .map(|v| v.get("previous_response_id").is_none())
            .unwrap_or(false)
    }
}

/// Matcher asserting that JSON body HAS a `previous_response_id` field.
struct HasPrevId;

impl Match for HasPrevId {
    fn matches(&self, req: &Request) -> bool {
        serde_json::from_slice::<Value>(&req.body)
            .map(|v| v.get("previous_response_id").is_some())
            .unwrap_or(false)
    }
}

/// Build minimal SSE stream with completed marker.
fn sse_completed(id: &str) -> String {
    format!(
        "event: response.completed\n\
data: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"{}\",\"output\":[]}}}}\n\n\n",
        id
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keeps_previous_response_id_between_tasks() {
    #![allow(clippy::unwrap_used)]

    if std::env::var(SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
        println!(
            "Skipping test because it cannot execute when network is disabled in a Seeky sandbox."
        );
        return;
    }

    // Mock server
    let server = MockServer::start().await;

    // First request – must NOT include `previous_response_id`.
    let first = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse_completed("resp1"), "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(NoPrevId)
        .respond_with(first)
        .expect(1)
        .mount(&server)
        .await;

    // Second request – MUST include `previous_response_id`.
    let second = ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_raw(sse_completed("resp2"), "text/event-stream");

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(HasPrevId)
        .respond_with(second)
        .expect(1)
        .mount(&server)
        .await;

    // Environment
    // Update environment – `set_var` is `unsafe` starting with the 2024
    // edition so we group the calls into a single `unsafe { … }` block.
    unsafe {
        std::env::set_var("OPENAI_REQUEST_MAX_RETRIES", "0");
        std::env::set_var("OPENAI_STREAM_MAX_RETRIES", "0");
    }
    let model_provider = ModelProviderInfo {
        name: "openai".into(),
        base_url: format!("{}/v1", server.uri()),
        // Environment variable that should exist in the test environment.
        // ModelClient will return an error if the environment variable for the
        // provider is not set.
        env_key: Some("PATH".into()),
        env_key_instructions: None,
        wire_api: seeky_core::WireApi::Responses,
    };

    // Init session
    let seeky_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&seeky_home);
    config.model_provider = model_provider;
    let ctrl_c = std::sync::Arc::new(tokio::sync::Notify::new());
    let (seeky, _init_id) = Seeky::spawn(config, ctrl_c.clone()).await.unwrap();

    // Task 1 – triggers first request (no previous_response_id)
    seeky
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello".into(),
            }],
        })
        .await
        .unwrap();

    // Wait for TaskComplete
    loop {
        let ev = timeout(Duration::from_secs(1), seeky.next_event())
            .await
            .unwrap()
            .unwrap();
        if matches!(ev.msg, EventMsg::TaskComplete(_)) {
            break;
        }
    }

    // Task 2 – should include `previous_response_id` (triggers second request)
    seeky
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "again".into(),
            }],
        })
        .await
        .unwrap();

    // Wait for TaskComplete or error
    loop {
        let ev = timeout(Duration::from_secs(1), seeky.next_event())
            .await
            .unwrap()
            .unwrap();
        match ev.msg {
            EventMsg::TaskComplete(_) => break,
            EventMsg::Error(ErrorEvent { message }) => {
                panic!("unexpected error: {message}")
            }
            _ => {
                // Ignore other events.
            }
        }
    }
}
