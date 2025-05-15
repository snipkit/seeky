//! Asynchronous worker that executes a **Seeky** tool-call inside a spawned
//! Tokio task. Separated from `message_processor.rs` to keep that file small
//! and to make future feature-growth easier to manage.

use seeky_core::seeky_wrapper::init_seeky;
use seeky_core::config::Config as SeekyConfig;
use seeky_core::protocol::AgentMessageEvent;
use seeky_core::protocol::Event;
use seeky_core::protocol::EventMsg;
use seeky_core::protocol::InputItem;
use seeky_core::protocol::Op;
use mcp_types::CallToolResult;
use mcp_types::CallToolResultContent;
use mcp_types::JSONRPC_VERSION;
use mcp_types::JSONRPCMessage;
use mcp_types::JSONRPCResponse;
use mcp_types::RequestId;
use mcp_types::TextContent;
use tokio::sync::mpsc::Sender;

/// Convert a Seeky [`Event`] to an MCP notification.
fn seeky_event_to_notification(event: &Event) -> JSONRPCMessage {
    #[expect(clippy::expect_used)]
    JSONRPCMessage::Notification(mcp_types::JSONRPCNotification {
        jsonrpc: JSONRPC_VERSION.into(),
        method: "seeky/event".into(),
        params: Some(serde_json::to_value(event).expect("Event must serialize")),
    })
}

/// Run a complete Seeky session and stream events back to the client.
///
/// On completion (success or error) the function sends the appropriate
/// `tools/call` response so the LLM can continue the conversation.
pub async fn run_seeky_tool_session(
    id: RequestId,
    initial_prompt: String,
    config: SeekyConfig,
    outgoing: Sender<JSONRPCMessage>,
) {
    let (seeky, first_event, _ctrl_c) = match init_seeky(config).await {
        Ok(res) => res,
        Err(e) => {
            let result = CallToolResult {
                content: vec![CallToolResultContent::TextContent(TextContent {
                    r#type: "text".to_string(),
                    text: format!("Failed to start Seeky session: {e}"),
                    annotations: None,
                })],
                is_error: Some(true),
            };
            let _ = outgoing
                .send(JSONRPCMessage::Response(JSONRPCResponse {
                    jsonrpc: JSONRPC_VERSION.into(),
                    id,
                    result: result.into(),
                }))
                .await;
            return;
        }
    };

    // Send initial SessionConfigured event.
    let _ = outgoing
        .send(seeky_event_to_notification(&first_event))
        .await;

    if let Err(e) = seeky
        .submit(Op::UserInput {
            items: vec![InputItem::Text {
                text: initial_prompt.clone(),
            }],
        })
        .await
    {
        tracing::error!("Failed to submit initial prompt: {e}");
    }

    let mut last_agent_message: Option<String> = None;

    // Stream events until the task needs to pause for user interaction or
    // completes.
    loop {
        match seeky.next_event().await {
            Ok(event) => {
                let _ = outgoing.send(seeky_event_to_notification(&event)).await;

                match &event.msg {
                    EventMsg::AgentMessage(AgentMessageEvent { message }) => {
                        last_agent_message = Some(message.clone());
                    }
                    EventMsg::ExecApprovalRequest(_) => {
                        let result = CallToolResult {
                            content: vec![CallToolResultContent::TextContent(TextContent {
                                r#type: "text".to_string(),
                                text: "EXEC_APPROVAL_REQUIRED".to_string(),
                                annotations: None,
                            })],
                            is_error: None,
                        };
                        let _ = outgoing
                            .send(JSONRPCMessage::Response(JSONRPCResponse {
                                jsonrpc: JSONRPC_VERSION.into(),
                                id: id.clone(),
                                result: result.into(),
                            }))
                            .await;
                        break;
                    }
                    EventMsg::ApplyPatchApprovalRequest(_) => {
                        let result = CallToolResult {
                            content: vec![CallToolResultContent::TextContent(TextContent {
                                r#type: "text".to_string(),
                                text: "PATCH_APPROVAL_REQUIRED".to_string(),
                                annotations: None,
                            })],
                            is_error: None,
                        };
                        let _ = outgoing
                            .send(JSONRPCMessage::Response(JSONRPCResponse {
                                jsonrpc: JSONRPC_VERSION.into(),
                                id: id.clone(),
                                result: result.into(),
                            }))
                            .await;
                        break;
                    }
                    EventMsg::TaskComplete => {
                        let result = if let Some(msg) = last_agent_message {
                            CallToolResult {
                                content: vec![CallToolResultContent::TextContent(TextContent {
                                    r#type: "text".to_string(),
                                    text: msg,
                                    annotations: None,
                                })],
                                is_error: None,
                            }
                        } else {
                            CallToolResult {
                                content: vec![CallToolResultContent::TextContent(TextContent {
                                    r#type: "text".to_string(),
                                    text: String::new(),
                                    annotations: None,
                                })],
                                is_error: None,
                            }
                        };
                        let _ = outgoing
                            .send(JSONRPCMessage::Response(JSONRPCResponse {
                                jsonrpc: JSONRPC_VERSION.into(),
                                id: id.clone(),
                                result: result.into(),
                            }))
                            .await;
                        break;
                    }
                    EventMsg::SessionConfigured(_) => {
                        tracing::error!("unexpected SessionConfigured event");
                    }
                    EventMsg::Error(_)
                    | EventMsg::TaskStarted
                    | EventMsg::AgentReasoning(_)
                    | EventMsg::McpToolCallBegin(_)
                    | EventMsg::McpToolCallEnd(_)
                    | EventMsg::ExecCommandBegin(_)
                    | EventMsg::ExecCommandEnd(_)
                    | EventMsg::BackgroundEvent(_)
                    | EventMsg::PatchApplyBegin(_)
                    | EventMsg::PatchApplyEnd(_) => {
                        // For now, we do not do anything extra for these
                        // events. Note that
                        // send(seeky_event_to_notification(&event)) above has
                        // already dispatched these events as notifications,
                        // though we may want to do give different treatment to
                        // individual events in the future.
                    }
                }
            }
            Err(e) => {
                let result = CallToolResult {
                    content: vec![CallToolResultContent::TextContent(TextContent {
                        r#type: "text".to_string(),
                        text: format!("Seeky runtime error: {e}"),
                        annotations: None,
                    })],
                    is_error: Some(true),
                };
                let _ = outgoing
                    .send(JSONRPCMessage::Response(JSONRPCResponse {
                        jsonrpc: JSONRPC_VERSION.into(),
                        id: id.clone(),
                        result: result.into(),
                    }))
                    .await;
                break;
            }
        }
    }
}
