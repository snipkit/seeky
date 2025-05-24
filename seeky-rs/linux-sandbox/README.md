# seeky-linux-sandbox

This crate is responsible for producing:

- a `seeky-linux-sandbox` standalone executable for Linux that is bundled with the Node.js version of the Seeky CLI
- a lib crate that exposes the business logic of the executable as `run_main()` so that
  - the `seeky-exec` CLI can check if its arg0 is `seeky-linux-sandbox` and, if so, execute as if it were `seeky-linux-sandbox`
  - this should also be true of the `seeky` multitool CLI
