use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

use serde::Serialize;

const SERVICE_NAME: &str = "com.minecraft-pack-builder.curseforge";
const ACCOUNT_NAME: &str = "curseforge-api-key";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeCredentialStatus {
    pub state: CurseForgeCredentialState,
    pub backend: String,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl CurseForgeCredentialStatus {
    pub fn missing(backend: impl Into<String>) -> Self {
        Self {
            state: CurseForgeCredentialState::Missing,
            backend: backend.into(),
            message: None,
            api_key: None,
        }
    }

    pub fn saved(backend: impl Into<String>) -> Self {
        Self {
            state: CurseForgeCredentialState::Saved,
            backend: backend.into(),
            message: None,
            api_key: None,
        }
    }

    pub fn unavailable(backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            state: CurseForgeCredentialState::Unavailable,
            backend: backend.into(),
            message: Some(message.into()),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CurseForgeCredentialState {
    Missing,
    Saved,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    EmptyApiKey,
    CommandFailed(String),
}

impl fmt::Display for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyApiKey => write!(formatter, "CurseForge API key cannot be empty"),
            Self::CommandFailed(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for CredentialError {}

pub fn validate_curseforge_api_key(api_key: &str) -> Result<(), CredentialError> {
    if api_key.trim().is_empty() {
        return Err(CredentialError::EmptyApiKey);
    }

    Ok(())
}

pub fn curseforge_key_status() -> CurseForgeCredentialStatus {
    match platform_has_curseforge_key() {
        Ok(true) => CurseForgeCredentialStatus::saved(secure_storage_backend_name()),
        Ok(false) => CurseForgeCredentialStatus::missing(secure_storage_backend_name()),
        Err(error) => CurseForgeCredentialStatus::unavailable(
            secure_storage_backend_name(),
            error.to_string(),
        ),
    }
}

pub fn save_curseforge_key(api_key: &str) -> Result<CurseForgeCredentialStatus, CredentialError> {
    validate_curseforge_api_key(api_key)?;

    match platform_save_curseforge_key(api_key.trim()) {
        Ok(()) => Ok(CurseForgeCredentialStatus::saved(
            secure_storage_backend_name(),
        )),
        Err(error) => Ok(CurseForgeCredentialStatus::unavailable(
            secure_storage_backend_name(),
            error.to_string(),
        )),
    }
}

fn secure_storage_backend_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS Keychain"
    } else if cfg!(target_os = "windows") {
        "Windows Credential Manager"
    } else if cfg!(target_os = "linux") {
        "Linux Secret Service"
    } else {
        "OS secure credential storage"
    }
}

fn platform_has_curseforge_key() -> Result<bool, CredentialError> {
    if cfg!(target_os = "macos") {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-a",
                ACCOUNT_NAME,
                "-s",
                SERVICE_NAME,
            ])
            .output()
            .map_err(command_error)?;

        return Ok(output.status.success());
    }

    if cfg!(target_os = "windows") {
        let output = Command::new("cmdkey")
            .arg(format!("/list:{SERVICE_NAME}"))
            .output()
            .map_err(command_error)?;

        return Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains(SERVICE_NAME));
    }

    if cfg!(target_os = "linux") {
        let output = Command::new("secret-tool")
            .args([
                "lookup",
                "application",
                "minecraft-pack-builder",
                "key",
                "curseforge",
            ])
            .output()
            .map_err(command_error)?;

        return Ok(output.status.success());
    }

    Err(CredentialError::CommandFailed(
        "secure credential storage is not supported on this platform".to_string(),
    ))
}

fn platform_save_curseforge_key(api_key: &str) -> Result<(), CredentialError> {
    if cfg!(target_os = "macos") {
        let output = Command::new("security")
            .args([
                "add-generic-password",
                "-a",
                ACCOUNT_NAME,
                "-s",
                SERVICE_NAME,
                "-w",
                api_key,
                "-U",
            ])
            .output()
            .map_err(command_error)?;

        return output_to_result(output);
    }

    if cfg!(target_os = "windows") {
        let output = Command::new("cmdkey")
            .args([
                &format!("/generic:{SERVICE_NAME}"),
                &format!("/user:{ACCOUNT_NAME}"),
                &format!("/pass:{api_key}"),
            ])
            .output()
            .map_err(command_error)?;

        return output_to_result(output);
    }

    if cfg!(target_os = "linux") {
        let mut child = Command::new("secret-tool")
            .args([
                "store",
                "--label",
                "Minecraft Pack Builder CurseForge API key",
                "application",
                "minecraft-pack-builder",
                "key",
                "curseforge",
            ])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(command_error)?;

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            CredentialError::CommandFailed("could not open secret-tool stdin".to_string())
        })?;
        stdin.write_all(api_key.as_bytes()).map_err(command_error)?;

        let output = child.wait_with_output().map_err(command_error)?;
        return output_to_result(output);
    }

    Err(CredentialError::CommandFailed(
        "secure credential storage is not supported on this platform".to_string(),
    ))
}

fn output_to_result(output: std::process::Output) -> Result<(), CredentialError> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let message = if !stderr.is_empty() { stderr } else { stdout };

    Err(CredentialError::CommandFailed(if message.is_empty() {
        "secure storage command failed".to_string()
    } else {
        message
    }))
}

fn command_error(error: std::io::Error) -> CredentialError {
    CredentialError::CommandFailed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_saved_and_unavailable_without_returning_secret_values() {
        let missing = CurseForgeCredentialStatus::missing("test backend");
        let saved = CurseForgeCredentialStatus::saved("test backend");
        let unavailable =
            CurseForgeCredentialStatus::unavailable("test backend", "secure store is locked");

        assert_eq!(missing.state, CurseForgeCredentialState::Missing);
        assert_eq!(saved.state, CurseForgeCredentialState::Saved);
        assert_eq!(unavailable.state, CurseForgeCredentialState::Unavailable);
        assert_eq!(saved.api_key, None);
        assert_eq!(unavailable.api_key, None);
    }

    #[test]
    fn rejects_blank_curseforge_api_keys_before_touching_secure_storage() {
        assert!(validate_curseforge_api_key("abc123").is_ok());
        assert_eq!(
            validate_curseforge_api_key("  ").unwrap_err().to_string(),
            "CurseForge API key cannot be empty"
        );
    }
}
