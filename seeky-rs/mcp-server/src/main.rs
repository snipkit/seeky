use seeky_mcp_server::run_main;

fn main() -> anyhow::Result<()> {
    seeky_linux_sandbox::run_with_sandbox(|seeky_linux_sandbox_exe| async move {
        run_main(seeky_linux_sandbox_exe).await?;
        Ok(())
    })
}
