mod cli;
mod event_processor;

use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub use cli::Cli;
use seeky_core::seeky_wrapper;
use seeky_core::config::Config;
use seeky_core::config::ConfigOverrides;
use seeky_core::protocol::AskForApproval;
use seeky_core::protocol::Event;
use seeky_core::protocol::EventMsg;
use seeky_core::protocol::InputItem;
use seeky_core::protocol::Op;
use seeky_core::protocol::SandboxPolicy;
use seeky_core::protocol::TaskCompleteEvent;
use seeky_core::util::is_inside_git_repo;
use event_processor::EventProcessor;
use event_processor::print_config_summary;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub async fn run_main(cli: Cli, seeky_linux_sandbox_exe: Option<PathBuf>) -> anyhow::Result<()> {
    let Cli {
        images,
        model,
        config_profile,
        full_auto,
        sandbox,
        cwd,
        skip_git_repo_check,
        disable_response_storage,
        color,
        last_message_file,
        prompt,
    } = cli;

    let (stdout_with_ansi, stderr_with_ansi) = match color {
        cli::Color::Always => (true, true),
        cli::Color::Never => (false, false),
        cli::Color::Auto => (
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ),
    };

    let sandbox_policy = if full_auto {
        Some(SandboxPolicy::new_full_auto_policy())
    } else {
        sandbox.permissions.clone().map(Into::into)
    };

    // Load configuration and determine approval policy
    let overrides = ConfigOverrides {
        model,
        config_profile,
        // This CLI is intended to be headless and has no affordances for asking
        // the user for approval.
        approval_policy: Some(AskForApproval::Never),
        sandbox_policy,
        disable_response_storage: if disable_response_storage {
            Some(true)
        } else {
            None
        },
        cwd: cwd.map(|p| p.canonicalize().unwrap_or(p)),
        model_provider: None,
        seeky_linux_sandbox_exe,
    };
    let config = Config::load_with_overrides(overrides)?;
    // Print the effective configuration so users can see what Seeky is using.
    print_config_summary(&config, stdout_with_ansi);

    if !skip_git_repo_check && !is_inside_git_repo(&config) {
        eprintln!("Not inside a Git repo and --skip-git-repo-check was not specified.");
        std::process::exit(1);
    }

    // TODO(mbolin): Take a more thoughtful approach to logging.
    let default_level = "error";
    let _ = tracing_subscriber::fmt()
        // Fallback to the `default_level` log filter if the environment
        // variable is not set _or_ contains an invalid value
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new(default_level))
                .unwrap_or_else(|_| EnvFilter::new(default_level)),
        )
        .with_ansi(stderr_with_ansi)
        .with_writer(std::io::stderr)
        .try_init();

    let (seeky_wrapper, event, ctrl_c) = seeky_wrapper::init_seeky(config).await?;
    let seeky = Arc::new(seeky_wrapper);
    info!("Seeky initialized with event: {event:?}");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
    {
        let seeky = seeky.clone();
        tokio::spawn(async move {
            loop {
                let interrupted = ctrl_c.notified();
                tokio::select! {
                    _ = interrupted => {
                        // Forward an interrupt to the seeky so it can abort any in‑flight task.
                        let _ = seeky
                            .submit(
                                Op::Interrupt,
                            )
                            .await;

                        // Exit the inner loop and return to the main input prompt.  The seeky
                        // will emit a `TurnInterrupted` (Error) event which is drained later.
                        break;
                    }
                    res = seeky.next_event() => match res {
                        Ok(event) => {
                            debug!("Received event: {event:?}");
                            if let Err(e) = tx.send(event) {
                                error!("Error sending event: {e:?}");
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Error receiving event: {e:?}");
                            break;
                        }
                    }
                }
            }
        });
    }

    // Send images first, if any.
    if !images.is_empty() {
        let items: Vec<InputItem> = images
            .into_iter()
            .map(|path| InputItem::LocalImage { path })
            .collect();
        let initial_images_event_id = seeky.submit(Op::UserInput { items }).await?;
        info!("Sent images with event ID: {initial_images_event_id}");
        while let Ok(event) = seeky.next_event().await {
            if event.id == initial_images_event_id
                && matches!(
                    event.msg,
                    EventMsg::TaskComplete(TaskCompleteEvent {
                        last_agent_message: _,
                    })
                )
            {
                break;
            }
        }
    }

    // Send the prompt.
    let items: Vec<InputItem> = vec![InputItem::Text { text: prompt }];
    let initial_prompt_task_id = seeky.submit(Op::UserInput { items }).await?;
    info!("Sent prompt with event ID: {initial_prompt_task_id}");

    // Run the loop until the task is complete.
    let mut event_processor = EventProcessor::create_with_ansi(stdout_with_ansi);
    while let Some(event) = rx.recv().await {
        let (is_last_event, last_assistant_message) = match &event.msg {
            EventMsg::TaskComplete(TaskCompleteEvent { last_agent_message }) => {
                (true, last_agent_message.clone())
            }
            _ => (false, None),
        };
        event_processor.process_event(event);
        if is_last_event {
            handle_last_message(last_assistant_message, last_message_file.as_deref())?;
            break;
        }
    }

    Ok(())
}

fn handle_last_message(
    last_agent_message: Option<String>,
    last_message_file: Option<&Path>,
) -> std::io::Result<()> {
    match (last_agent_message, last_message_file) {
        (Some(last_agent_message), Some(last_message_file)) => {
            // Last message and a file to write to.
            std::fs::write(last_message_file, last_agent_message)?;
        }
        (None, Some(last_message_file)) => {
            eprintln!(
                "Warning: No last message to write to file: {}",
                last_message_file.to_string_lossy()
            );
        }
        (_, None) => {
            // No last message and no file to write to.
        }
    }
    Ok(())
}
