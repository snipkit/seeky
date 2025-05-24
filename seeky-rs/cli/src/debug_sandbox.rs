use std::path::PathBuf;

use seeky_common::SandboxPermissionOption;
use seeky_core::config::Config;
use seeky_core::config::ConfigOverrides;
use seeky_core::exec::StdioPolicy;
use seeky_core::exec::spawn_command_under_linux_sandbox;
use seeky_core::exec::spawn_command_under_seatbelt;
use seeky_core::exec_env::create_env;
use seeky_core::protocol::SandboxPolicy;

use crate::LandlockCommand;
use crate::SeatbeltCommand;
use crate::exit_status::handle_exit_status;

pub async fn run_command_under_seatbelt(
    command: SeatbeltCommand,
    seeky_linux_sandbox_exe: Option<PathBuf>,
) -> anyhow::Result<()> {
    let SeatbeltCommand {
        full_auto,
        sandbox,
        command,
    } = command;
    run_command_under_sandbox(
        full_auto,
        sandbox,
        command,
        seeky_linux_sandbox_exe,
        SandboxType::Seatbelt,
    )
    .await
}

pub async fn run_command_under_landlock(
    command: LandlockCommand,
    seeky_linux_sandbox_exe: Option<PathBuf>,
) -> anyhow::Result<()> {
    let LandlockCommand {
        full_auto,
        sandbox,
        command,
    } = command;
    run_command_under_sandbox(
        full_auto,
        sandbox,
        command,
        seeky_linux_sandbox_exe,
        SandboxType::Landlock,
    )
    .await
}

enum SandboxType {
    Seatbelt,
    Landlock,
}

async fn run_command_under_sandbox(
    full_auto: bool,
    sandbox: SandboxPermissionOption,
    command: Vec<String>,
    seeky_linux_sandbox_exe: Option<PathBuf>,
    sandbox_type: SandboxType,
) -> anyhow::Result<()> {
    let sandbox_policy = create_sandbox_policy(full_auto, sandbox);
    let cwd = std::env::current_dir()?;
    let config = Config::load_with_overrides(ConfigOverrides {
        sandbox_policy: Some(sandbox_policy),
        seeky_linux_sandbox_exe,
        ..Default::default()
    })?;
    let stdio_policy = StdioPolicy::Inherit;
    let env = create_env(&config.shell_environment_policy);

    let mut child = match sandbox_type {
        SandboxType::Seatbelt => {
            spawn_command_under_seatbelt(command, &config.sandbox_policy, cwd, stdio_policy, env)
                .await?
        }
        SandboxType::Landlock => {
            #[expect(clippy::expect_used)]
            let seeky_linux_sandbox_exe = config
                .seeky_linux_sandbox_exe
                .expect("seeky-linux-sandbox executable not found");
            spawn_command_under_linux_sandbox(
                seeky_linux_sandbox_exe,
                command,
                &config.sandbox_policy,
                cwd,
                stdio_policy,
                env,
            )
            .await?
        }
    };
    let status = child.wait().await?;

    handle_exit_status(status);
}

pub fn create_sandbox_policy(full_auto: bool, sandbox: SandboxPermissionOption) -> SandboxPolicy {
    if full_auto {
        SandboxPolicy::new_full_auto_policy()
    } else {
        match sandbox.permissions.map(Into::into) {
            Some(sandbox_policy) => sandbox_policy,
            None => SandboxPolicy::new_read_only_policy(),
        }
    }
}
