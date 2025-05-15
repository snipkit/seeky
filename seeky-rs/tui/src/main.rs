use clap::Parser;
use seeky_tui::Cli;
use seeky_tui::run_main;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    run_main(cli)?;
    Ok(())
}
