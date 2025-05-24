use clap::Parser;
use seeky_tui::Cli;
use seeky_tui::run_main;

fn main() -> anyhow::Result<()> {
    seeky_linux_sandbox::run_with_sandbox(|seeky_linux_sandbox_exe| async move {
        let cli = Cli::parse();
        run_main(cli, seeky_linux_sandbox_exe)?;
        Ok(())
    })
}
