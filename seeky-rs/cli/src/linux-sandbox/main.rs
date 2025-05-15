#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    eprintln!("seeky-linux-sandbox is not supported on this platform.");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use seeky_cli::LandlockCommand;
    use seeky_cli::create_sandbox_policy;
    use seeky_cli::landlock;

    let LandlockCommand {
        full_auto,
        sandbox,
        command,
    } = LandlockCommand::parse();
    let sandbox_policy = create_sandbox_policy(full_auto, sandbox);
    landlock::run_landlock(command, sandbox_policy)?;
    Ok(())
}
