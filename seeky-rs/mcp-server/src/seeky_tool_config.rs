//! Configuration object accepted by the `seeky` MCP tool-call.

use std::path::PathBuf;

use mcp_types::Tool;
use mcp_types::ToolInputSchema;
use schemars::JsonSchema;
use schemars::r#gen::SchemaSettings;
use serde::Deserialize;

use seeky_core::protocol::AskForApproval;
use seeky_core::protocol::SandboxPolicy;

/// Client-supplied configuration for a `seeky` tool-call.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct SeekyToolCallParam {
    /// The *initial user prompt* to start the Seeky conversation.
    pub prompt: String,

    /// Optional override for the model name (e.g. "o3", "o4-mini")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Configuration profile from config.toml to specify default options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Working directory for the session. If relative, it is resolved against
    /// the server process's current working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Execution approval policy expressed as the kebab-case variant name
    /// (`unless-allow-listed`, `auto-edit`, `on-failure`, `never`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<SeekyToolCallApprovalPolicy>,

    /// Sandbox permissions using the same string values accepted by the CLI
    /// (e.g. "disk-write-cwd", "network-full-access").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_permissions: Option<Vec<SeekyToolCallSandboxPermission>>,

    /// Disable server-side response storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_response_storage: Option<bool>,
    // Custom system instructions.
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub instructions: Option<String>,
}

// Create custom enums for use with `SeekyToolCallApprovalPolicy` where we
// intentionally exclude docstrings from the generated schema because they
// introduce anyOf in the the generated JSON schema, which makes it more complex
// without adding any real value since we aspire to use self-descriptive names.

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SeekyToolCallApprovalPolicy {
    AutoEdit,
    UnlessAllowListed,
    OnFailure,
    Never,
}

impl From<SeekyToolCallApprovalPolicy> for AskForApproval {
    fn from(value: SeekyToolCallApprovalPolicy) -> Self {
        match value {
            SeekyToolCallApprovalPolicy::AutoEdit => AskForApproval::AutoEdit,
            SeekyToolCallApprovalPolicy::UnlessAllowListed => AskForApproval::UnlessAllowListed,
            SeekyToolCallApprovalPolicy::OnFailure => AskForApproval::OnFailure,
            SeekyToolCallApprovalPolicy::Never => AskForApproval::Never,
        }
    }
}

// TODO: Support additional writable folders via a separate property on
// SeekyToolCallParam.

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SeekyToolCallSandboxPermission {
    DiskFullReadAccess,
    DiskWriteCwd,
    DiskWritePlatformUserTempFolder,
    DiskWritePlatformGlobalTempFolder,
    DiskFullWriteAccess,
    NetworkFullAccess,
}

impl From<SeekyToolCallSandboxPermission> for seeky_core::protocol::SandboxPermission {
    fn from(value: SeekyToolCallSandboxPermission) -> Self {
        match value {
            SeekyToolCallSandboxPermission::DiskFullReadAccess => {
                seeky_core::protocol::SandboxPermission::DiskFullReadAccess
            }
            SeekyToolCallSandboxPermission::DiskWriteCwd => {
                seeky_core::protocol::SandboxPermission::DiskWriteCwd
            }
            SeekyToolCallSandboxPermission::DiskWritePlatformUserTempFolder => {
                seeky_core::protocol::SandboxPermission::DiskWritePlatformUserTempFolder
            }
            SeekyToolCallSandboxPermission::DiskWritePlatformGlobalTempFolder => {
                seeky_core::protocol::SandboxPermission::DiskWritePlatformGlobalTempFolder
            }
            SeekyToolCallSandboxPermission::DiskFullWriteAccess => {
                seeky_core::protocol::SandboxPermission::DiskFullWriteAccess
            }
            SeekyToolCallSandboxPermission::NetworkFullAccess => {
                seeky_core::protocol::SandboxPermission::NetworkFullAccess
            }
        }
    }
}

pub(crate) fn create_tool_for_seeky_tool_call_param() -> Tool {
    let schema = SchemaSettings::draft2019_09()
        .with(|s| {
            s.inline_subschemas = true;
            s.option_add_null_type = false
        })
        .into_generator()
        .into_root_schema_for::<SeekyToolCallParam>();

    #[expect(clippy::expect_used)]
    let schema_value =
        serde_json::to_value(&schema).expect("Seeky tool schema should serialise to JSON");

    let tool_input_schema =
        serde_json::from_value::<ToolInputSchema>(schema_value).unwrap_or_else(|e| {
            panic!("failed to create Tool from schema: {e}");
        });
    Tool {
        name: "seeky".to_string(),
        input_schema: tool_input_schema,
        description: Some(
            "Run a Seeky session. Accepts configuration parameters matching the Seeky Config struct."
                .to_string(),
        ),
        annotations: None,
    }
}

impl SeekyToolCallParam {
    /// Returns the initial user prompt to start the Seeky conversation and the
    /// Config.
    pub fn into_config(
        self,
        seeky_linux_sandbox_exe: Option<PathBuf>,
    ) -> std::io::Result<(String, seeky_core::config::Config)> {
        let Self {
            prompt,
            model,
            profile,
            cwd,
            approval_policy,
            sandbox_permissions,
            disable_response_storage,
        } = self;
        let sandbox_policy = sandbox_permissions.map(|perms| {
            SandboxPolicy::from(perms.into_iter().map(Into::into).collect::<Vec<_>>())
        });

        // Build ConfigOverrides recognised by seeky-core.
        let overrides = seeky_core::config::ConfigOverrides {
            model,
            config_profile: profile,
            cwd: cwd.map(PathBuf::from),
            approval_policy: approval_policy.map(Into::into),
            sandbox_policy,
            disable_response_storage,
            model_provider: None,
            seeky_linux_sandbox_exe,
        };

        let cfg = seeky_core::config::Config::load_with_overrides(overrides)?;

        Ok((prompt, cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// We include a test to verify the exact JSON schema as "executable
    /// documentation" for the schema. When can track changes to this test as a
    /// way to audit changes to the generated schema.
    ///
    /// Seeing the fully expanded schema makes it easier to casually verify that
    /// the generated JSON for enum types such as "approval-policy" is compact.
    /// Ideally, modelcontextprotocol/inspector would provide a simpler UI for
    /// enum fields versus open string fields to take advantage of this.
    ///
    /// As of 2025-05-04, there is an open PR for this:
    /// https://github.com/modelcontextprotocol/inspector/pull/196
    #[test]
    fn verify_seeky_tool_json_schema() {
        let tool = create_tool_for_seeky_tool_call_param();
        #[expect(clippy::expect_used)]
        let tool_json = serde_json::to_value(&tool).expect("tool serializes");
        let expected_tool_json = serde_json::json!({
          "name": "seeky",
          "description": "Run a Seeky session. Accepts configuration parameters matching the Seeky Config struct.",
          "inputSchema": {
            "type": "object",
            "properties": {
              "approval-policy": {
                "description": "Execution approval policy expressed as the kebab-case variant name (`unless-allow-listed`, `auto-edit`, `on-failure`, `never`).",
                "enum": [
                  "auto-edit",
                  "unless-allow-listed",
                  "on-failure",
                  "never"
                ],
                "type": "string"
              },
              "cwd": {
                "description": "Working directory for the session. If relative, it is resolved against the server process's current working directory.",
                "type": "string"
              },
              "disable-response-storage": {
                "description": "Disable server-side response storage.",
                "type": "boolean"
              },
              "model": {
                "description": "Optional override for the model name (e.g. \"o3\", \"o4-mini\")",
                "type": "string"
              },
              "profile": {
                "description": "Configuration profile from config.toml to specify default options.",
                "type": "string"
              },
              "prompt": {
                "description": "The *initial user prompt* to start the Seeky conversation.",
                "type": "string"
              },
              "sandbox-permissions": {
                "description": "Sandbox permissions using the same string values accepted by the CLI (e.g. \"disk-write-cwd\", \"network-full-access\").",
                "items": {
                  "enum": [
                    "disk-full-read-access",
                    "disk-write-cwd",
                    "disk-write-platform-user-temp-folder",
                    "disk-write-platform-global-temp-folder",
                    "disk-full-write-access",
                    "network-full-access"
                  ],
                  "type": "string"
                },
                "type": "array"
              }
            },
            "required": [
              "prompt"
            ]
          }
        });
        assert_eq!(expected_tool_json, tool_json);
    }
}
