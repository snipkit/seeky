use clap::Parser;
use seeky_cli::LandlockCommand;
use seeky_cli::SeatbeltCommand;
use seeky_cli::proto;
use seeky_exec::Cli as ExecCli;
use seeky_tui::Cli as TuiCli;
use std::path::PathBuf;

use crate::proto::ProtoCli;

/// Seeky CLI
///
/// If no subcommand is specified, options will be forwarded to the interactive CLI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true
)]
struct MultitoolCli {
    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run Seeky non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Experimental: run Seeky as an MCP server.
    Mcp,

    /// Run the Protocol stream via stdin/stdout
    #[clap(visible_alias = "p")]
    Proto(ProtoCli),

    /// Internal debugging commands.
    Debug(DebugArgs),
}

#[derive(Debug, Parser)]
struct DebugArgs {
    #[command(subcommand)]
    cmd: DebugCommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugCommand {
    /// Run a command under Seatbelt (macOS only).
    Seatbelt(SeatbeltCommand),

    /// Run a command under Landlock+seccomp (Linux only).
    Landlock(LandlockCommand),
}

#[derive(Debug, Parser)]
struct ReplProto {}

fn main() -> anyhow::Result<()> {
    seeky_linux_sandbox::run_with_sandbox(|seeky_linux_sandbox_exe| async move {
        cli_main(seeky_linux_sandbox_exe).await?;
        Ok(())
    })
}

async fn cli_main(seeky_linux_sandbox_exe: Option<PathBuf>) -> anyhow::Result<()> {
    let cli = MultitoolCli::parse();

    match cli.subcommand {
        None => {
            seeky_tui::run_main(cli.interactive, seeky_linux_sandbox_exe)?;
        }
        Some(Subcommand::Exec(exec_cli)) => {
            seeky_exec::run_main(exec_cli, seeky_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Mcp) => {
            seeky_mcp_server::run_main(seeky_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Proto(proto_cli)) => {
            proto::run_main(proto_cli).await?;
        }
        Some(Subcommand::Debug(debug_args)) => match debug_args.cmd {
            DebugCommand::Seatbelt(seatbelt_command) => {
                seeky_cli::debug_sandbox::run_command_under_seatbelt(
                    seatbelt_command,
                    seeky_linux_sandbox_exe,
                )
                .await?;
            }
            DebugCommand::Landlock(landlock_command) => {
                seeky_cli::debug_sandbox::run_command_under_landlock(
                    landlock_command,
                    seeky_linux_sandbox_exe,
                )
                .await?;
            }
        },
    }

    Ok(())
}
