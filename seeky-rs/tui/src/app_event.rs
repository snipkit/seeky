use seeky_core::protocol::Event;
use crossterm::event::KeyEvent;

use crate::slash_command::SlashCommand;

#[allow(clippy::large_enum_variant)]
pub(crate) enum AppEvent {
    SeekyEvent(Event),

    Redraw,

    KeyEvent(KeyEvent),

    /// Scroll event with a value representing the "scroll delta" as the net
    /// scroll up/down events within a short time window.
    Scroll(i32),

    /// Request to exit the application gracefully.
    ExitRequest,

    /// Forward an `Op` to the Agent. Using an `AppEvent` for this avoids
    /// bubbling channels through layers of widgets.
    SeekyOp(seeky_core::protocol::Op),

    /// Latest formatted log line emitted by `tracing`.
    LatestLog(String),

    /// Dispatch a recognized slash command from the UI (composer) to the app
    /// layer so it can be handled centrally.
    DispatchCommand(SlashCommand),
}
