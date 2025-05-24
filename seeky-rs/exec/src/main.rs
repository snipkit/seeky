//! Entry-point for the `seeky-exec` binary.
//!
//! When this CLI is invoked normally, it parses the standard `seeky-exec` CLI
//! options and launches the non-interactive Seeky agent. However, if it is
//! invoked with arg0 as `seeky-linux-sandbox`, we instead treat the invocation
//! as a request to run the logic for the standalone `seeky-linux-sandbox`
//! executable (i.e., parse any -s args and then run a *sandboxed* command under
//! Landlock + seccomp.
//!
//! This allows us to ship a completely separate set of functionality as part
//! of the `seeky-exec` binary.
use clap::Parser;
use seeky_exec::Cli;
use seeky_exec::run_main;

fn main() -> anyhow::Result<()> {
    seeky_linux_sandbox::run_with_sandbox(|seeky_linux_sandbox_exe| async move {
        let cli = Cli::parse();
        run_main(cli, seeky_linux_sandbox_exe).await?;
        Ok(())
    })
}
