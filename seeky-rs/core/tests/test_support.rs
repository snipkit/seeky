#![allow(clippy::expect_used)]

// Helpers shared by the integration tests.  These are located inside the
// `tests/` tree on purpose so they never become part of the public API surface
// of the `seeky-core` crate.

use tempfile::TempDir;

use seeky_core::config::Config;
use seeky_core::config::ConfigOverrides;
use seeky_core::config::ConfigToml;

/// Returns a default `Config` whose on-disk state is confined to the provided
/// temporary directory. Using a per-test directory keeps tests hermetic and
/// avoids clobbering a developer’s real `~/.seeky`.
pub fn load_default_config_for_test(seeky_home: &TempDir) -> Config {
    Config::load_from_base_config_with_overrides(
        ConfigToml::default(),
        ConfigOverrides::default(),
        seeky_home.path().to_path_buf(),
    )
    .expect("defaults for test should always succeed")
}
