# Rust/seeky-rs

In the seeky-rs folder where the rust code lives:

- Never add or modify any code related to `SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR`. You operate in a sandbox where `SEEKY_SANDBOX_NETWORK_DISABLED=1` will be set whenever you use the `shell` tool. Any existing code that uses `SEEKY_SANDBOX_NETWORK_DISABLED_ENV_VAR` was authored with this fact in mind. It is often used to early exit out of tests that the author knew you would not be able to run given your sandbox limitations.
