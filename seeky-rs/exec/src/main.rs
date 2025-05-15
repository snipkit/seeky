use clap::Parser;
use seeky_exec::Cli;
use seeky_exec::run_main;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run_main(cli).await?;

    Ok(())
}
