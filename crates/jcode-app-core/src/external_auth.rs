//! Headless handling for credentials discovered outside `JCODE_HOME`.

pub fn external_auth_blocked_message(
    provider_name: &str,
    source_name: &str,
    path: &std::path::Path,
) -> String {
    format!(
        "Found existing {provider_name} credentials from {source_name} at {}, but this headless runtime will not read an untrusted external source. Copy the credential into the writable JCODE_HOME volume, or explicitly allow the source with JCODE_TRUSTED_EXTERNAL_AUTH_SOURCES.",
        path.display()
    )
}
