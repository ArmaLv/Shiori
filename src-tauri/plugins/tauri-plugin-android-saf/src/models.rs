use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectFolderResponse {
    pub uri: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectFilesResponse {
    pub files: Vec<DocumentInfo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveCloudflareResponse {
    pub cookies: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInfo {
    pub uri: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumerateTreeResponse {
    pub files: Vec<DocumentInfo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyDocumentResponse {
    pub path: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckStoragePermissionResponse {
    pub granted: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentResponse {
    pub uri: String,
}

/// Response for native commands that return no meaningful data (e.g. `writeDocument`).
///
/// The Android bridge resolves such commands with `null` (Kotlin's bare
/// `invoke.resolve()`), while desktop returns an empty struct. A *derived*
/// `Deserialize` on an empty struct rejects `null` with
/// "invalid type: null, expected struct WriteDocumentResponse" — which is
/// exactly the error that broke backups on Android.
///
/// This hand-written impl accepts and discards **any** payload — `null`, `{}`,
/// or an object carrying extra fields a future native layer might add — so this
/// class of "void" call can never fail on a shape mismatch again. Use this as
/// the return type for any keyless/void native command.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WriteDocumentResponse {}

impl<'de> Deserialize<'de> for WriteDocumentResponse {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Consume whatever the native layer sent (null, {}, ...) and ignore it.
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(WriteDocumentResponse {})
    }
}
