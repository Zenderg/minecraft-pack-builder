use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::ptr::{null, null_mut};

use serde::Serialize;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Credentials::{
    CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

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

pub fn read_curseforge_key() -> Result<String, CredentialError> {
    platform_read_curseforge_key().map(|value| value.trim().to_string())
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
        return windows_has_curseforge_key();
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
        return windows_save_curseforge_key(api_key);
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

fn platform_read_curseforge_key() -> Result<String, CredentialError> {
    if cfg!(target_os = "macos") {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-a",
                ACCOUNT_NAME,
                "-s",
                SERVICE_NAME,
                "-w",
            ])
            .output()
            .map_err(command_error)?;

        if !output.status.success() {
            return Err(CredentialError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    if cfg!(target_os = "windows") {
        return windows_read_curseforge_key();
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

        if !output.status.success() {
            return Err(CredentialError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
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

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn windows_credential_target_name() -> &'static str {
    SERVICE_NAME
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn windows_secret_to_blob(secret: &str) -> Vec<u8> {
    secret
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>()
}

#[cfg(target_os = "windows")]
fn windows_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn windows_has_curseforge_key() -> Result<bool, CredentialError> {
    match windows_read_raw_credential() {
        Ok(Some(credential)) => {
            unsafe { CredFree(credential.cast()) };
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(not(target_os = "windows"))]
fn windows_has_curseforge_key() -> Result<bool, CredentialError> {
    Err(CredentialError::CommandFailed(
        "Windows Credential Manager is not available on this platform".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn windows_save_curseforge_key(api_key: &str) -> Result<(), CredentialError> {
    let mut target_name = windows_wide_null(windows_credential_target_name());
    let mut username = windows_wide_null(ACCOUNT_NAME);
    let mut credential_blob = windows_secret_to_blob(api_key);
    let credential = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target_name.as_mut_ptr(),
        Comment: null_mut(),
        LastWritten: unsafe { std::mem::zeroed() },
        CredentialBlobSize: credential_blob.len() as u32,
        CredentialBlob: credential_blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: null_mut(),
        TargetAlias: null_mut(),
        UserName: username.as_mut_ptr(),
    };

    let written = unsafe { CredWriteW(&credential, 0) };
    if written != 0 {
        Ok(())
    } else {
        Err(windows_last_error("could not save CurseForge API key"))
    }
}

#[cfg(not(target_os = "windows"))]
fn windows_save_curseforge_key(_api_key: &str) -> Result<(), CredentialError> {
    Err(CredentialError::CommandFailed(
        "Windows Credential Manager is not available on this platform".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn windows_read_curseforge_key() -> Result<String, CredentialError> {
    let credential = windows_read_raw_credential()?.ok_or_else(|| {
        CredentialError::CommandFailed("No CurseForge API key is saved".to_string())
    })?;
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (*credential).CredentialBlob,
            (*credential).CredentialBlobSize as usize,
        )
    };
    if bytes.len() % 2 != 0 {
        unsafe { CredFree(credential.cast()) };
        return Err(CredentialError::CommandFailed(
            "saved Windows credential blob is not valid UTF-16".to_string(),
        ));
    }

    let words = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    let secret = String::from_utf16(&words).map_err(|error| {
        CredentialError::CommandFailed(format!(
            "saved Windows credential is not valid UTF-16: {error}"
        ))
    });
    unsafe { CredFree(credential.cast()) };
    secret
}

#[cfg(not(target_os = "windows"))]
fn windows_read_curseforge_key() -> Result<String, CredentialError> {
    Err(CredentialError::CommandFailed(
        "Windows Credential Manager is not available on this platform".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn windows_read_raw_credential() -> Result<Option<*mut CREDENTIALW>, CredentialError> {
    let target_name = windows_wide_null(windows_credential_target_name());
    let mut credential: *mut CREDENTIALW = null_mut();
    let found = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if found != 0 {
        return Ok(Some(credential));
    }

    let error = unsafe { GetLastError() };
    if error == ERROR_NOT_FOUND {
        Ok(None)
    } else {
        Err(CredentialError::CommandFailed(format!(
            "Windows Credential Manager read failed with error code {error}"
        )))
    }
}

#[cfg(target_os = "windows")]
fn windows_last_error(context: &str) -> CredentialError {
    CredentialError::CommandFailed(format!("{context}: Windows error code {}", unsafe {
        GetLastError()
    }))
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

    #[test]
    fn windows_credential_helpers_use_stable_target_and_utf16_blob() {
        assert_eq!(windows_credential_target_name(), SERVICE_NAME);
        assert_eq!(
            windows_secret_to_blob("ключ-123"),
            vec![58, 4, 59, 4, 78, 4, 71, 4, 45, 0, 49, 0, 50, 0, 51, 0]
        );
    }
}
