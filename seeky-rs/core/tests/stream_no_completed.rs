//! Verifies that the agent retries when the SSE stream terminates before
//! delivering a `response.completed` event.

use std::time::Duration;

use seeky_core::ModelProviderInfo;
use seeky_core::Seeky;
use seeky_core::exec::SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR;
use seeky_core::protocol::EventMsg;
use seeky_core::protocol::InputItem;
use seeky_core::protocol::Op;
mod test_support;
use tempfile::TempDir;
use test_support::load_default_config_for_test;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::Request;
use wiremock::Respond;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn sse_incomplete() -> String {
    // Only a single line; missing the completed event.
    "event: response.output_item.done\n\n".to_string()
}

fn sse_completed(id: &str) -> String {
    format!(
        "event: response.completed\n\
data: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"{}\",\"output\":[]}}}}\n\n\n",
        id
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retries_on_early_close() {
    #![allow(clippy::unwrap_used)]

    if std::env::var(SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
        println!(
            "Skipping test because it cannot execute when network is disabled in a Seeky sandbox."
        );
        return;
    }

    let server = MockServer::start().await;

    struct SeqResponder;
    impl Respond for SeqResponder {
        fn respond(&self, _: &Request) -> ResponseTemplate {
            use std::sync::atomic::AtomicUsize;
            use std::sync::atomic::Ordering;
            static CALLS: AtomicUsize = AtomicUsize::new(0);
            let n = CALLS.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_incomplete(), "text/event-stream")
            } else {
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_completed("resp_ok"), "text/event-stream")
            }
        }
    }

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(SeqResponder {})
        .expect(2)
        .mount(&server)
        .await;

    // Environment
    //
    // As of Rust 2024 `std::env::set_var` has been made `unsafe` because
    // mutating the process environment is inherently racy when other threads
    // are running.  We therefore have to wrap every call in an explicit
    // `unsafe` block.  These are limited to the test-setup section so the
    // scope is very small and clearly delineated.

    unsafe {
        std::env::set_var("OPENAI_REQUEST_MAX_RETRIES", "0");
        std::env::set_var("OPENAI_STREAM_MAX_RETRIES", "1");
        std::env::set_var("OPENAI_STREAM_IDLE_TIMEOUT_MS", "2000");
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

    let ctrl_c = std::sync::Arc::new(tokio::sync::Notify::new());
    let seeky_home = TempDir::new().unwrap();
    let mut config = load_default_config_for_test(&seeky_home);
    config.model_provider = model_provider;
    let (seeky, _init_id) = Seeky::spawn(config, ctrl_c).await.unwrap();

    seeky
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: "hello".into(),
            }],
        })
        .await
        .unwrap();

    // Wait until TaskComplete (should succeed after retry).
    loop {
        let ev = timeout(Duration::from_secs(10), seeky.next_event())
            .await
            .unwrap()
            .unwrap();
        if matches!(ev.msg, EventMsg::TaskComplete(_)) {
            break;
        }
    }
}
