use std::sync::Arc;

use crate::Seeky;
use crate::config::Config;
use crate::protocol::Event;
use crate::protocol::EventMsg;
use crate::util::notify_on_sigint;
use tokio::sync::Notify;

/// Spawn a new [`Seeky`] and initialize the session.
///
/// Returns the wrapped [`Seeky`] **and** the `SessionInitialized` event that
/// is received as a response to the initial `ConfigureSession` submission so
/// that callers can surface the information to the UI.
pub async fn init_seeky(config: Config) -> anyhow::Result<(Seeky, Event, Arc<Notify>)> {
    let ctrl_c = notify_on_sigint();
    let (seeky, init_id) = Seeky::spawn(config, ctrl_c.clone()).await?;

    // The first event must be `SessionInitialized`. Validate and forward it to
    // the caller so that they can display it in the conversation history.
    let event = seeky.next_event().await?;
    if event.id != init_id
        || !matches!(
            &event,
            Event {
                id: _id,
                msg: EventMsg::SessionConfigured(_),
            }
        )
    {
        return Err(anyhow::anyhow!(
            "expected SessionInitialized but got {event:?}"
        ));
    }

    Ok((seeky, event, ctrl_c))
}
