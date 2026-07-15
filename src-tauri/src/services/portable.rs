use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::AppError;
use crate::models::{Folder, Prompt, PromptRevisionSource, PromptVersion};
use crate::storage::mapping::{folder_from_row, prompt_from_row, prompt_version_from_row};
use crate::storage::time::{iso8601_to_millis, now_millis};

const FORMAT_VERSION: u32 = 1;
const MANIFEST_NAME: &str = "manifest.json";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MEDIA_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptBundleManifest {
    pub format_version: u32,
    pub exported_at: String,
    pub prompts: Vec<Prompt>,
    pub revisions: Vec<PromptVersion>,
    pub folders: Vec<Folder>,
    pub media_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportConflictPolicy {
    Skip,
    Duplicate,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreview {
    pub format_version: u32,
    pub prompts: usize,
    pub revisions: usize,
    pub folders: usize,
    pub media_files: usize,
    pub additions: usize,
    pub conflicts: usize,
    pub private_prompts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableExportResult {
    pub file_path: String,
    pub prompts: usize,
    pub revisions: usize,
    pub media_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableImportResult {
    pub added: usize,
    pub skipped: usize,
    pub replaced: usize,
    pub backup_id: String,
}

#[derive(Debug)]
struct LoadedBundle {
    manifest: PromptBundleManifest,
    media: Vec<(PathBuf, Vec<u8>)>,
}

struct BundleData {
    prompts: Vec<Prompt>,
    revisions: Vec<PromptVersion>,
    folders: Vec<Folder>,
}

fn db_err(context: &str, error: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

fn enum_wire<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_value(value)
        .map_err(|error| AppError::internal(format!("failed to encode enum: {error}")))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| AppError::internal("enum did not encode as a string"))
}

fn json<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|error| AppError::internal(format!("failed to encode bundle value: {error}")))
}

fn safe_relative_path(raw: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(raw);
    if raw.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::validation(format!("unsafe bundle path `{raw}`")));
    }
    Ok(path.to_path_buf())
}

fn archive_name(prefix: &str, path: &Path) -> String {
    let suffix = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    format!("{prefix}/{suffix}")
}

fn load_all(conn: &Connection) -> Result<BundleData, AppError> {
    let prompts = crate::services::prompt::list(conn)?;
    let revisions = {
        let mut stmt = conn
            .prepare("SELECT * FROM prompt_versions ORDER BY prompt_id, version")
            .map_err(|error| db_err("failed to prepare revision export", error))?;
        let rows = stmt
            .query_map([], prompt_version_from_row)
            .map_err(|error| db_err("failed to query revisions", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| db_err("failed to read revisions", error))?;
        rows
    };
    let folders = {
        let mut stmt = conn
            .prepare("SELECT * FROM folders ORDER BY sort_order, id")
            .map_err(|error| db_err("failed to prepare folder export", error))?;
        let rows = stmt
            .query_map([], folder_from_row)
            .map_err(|error| db_err("failed to query folders", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| db_err("failed to read folders", error))?;
        rows
    };
    Ok(BundleData {
        prompts,
        revisions,
        folders,
    })
}

pub fn export_bundle(
    conn: &Connection,
    media_root: &Path,
    destination: &Path,
) -> Result<PortableExportResult, AppError> {
    let BundleData {
        prompts,
        revisions,
        folders,
    } = load_all(conn)?;
    let mut media_files = Vec::new();
    let mut seen = HashSet::new();
    for reference in prompts
        .iter()
        .flat_map(|prompt| prompt.images.iter().chain(prompt.videos.iter()))
    {
        let relative = safe_relative_path(reference)?;
        if media_root.join(&relative).is_file() && seen.insert(reference.clone()) {
            media_files.push(reference.clone());
        }
    }
    media_files.sort();

    let manifest = PromptBundleManifest {
        format_version: FORMAT_VERSION,
        exported_at: crate::storage::time::millis_to_iso8601(now_millis()),
        prompts,
        revisions,
        folders,
        media_files,
    };
    validate_manifest(&manifest)?;
    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| AppError::internal(format!("failed to encode bundle: {error}")))?;

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| AppError::io(format!("failed to create export directory: {error}")))?;
    }
    let temporary = destination.with_extension("prompthub.partial");
    let file = fs::File::create(&temporary)
        .map_err(|error| AppError::io(format!("failed to create bundle: {error}")))?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    archive
        .start_file(MANIFEST_NAME, options)
        .map_err(|error| AppError::io(format!("failed to add manifest: {error}")))?;
    archive
        .write_all(&manifest_json)
        .map_err(|error| AppError::io(format!("failed to write manifest: {error}")))?;
    for reference in &manifest.media_files {
        let relative = safe_relative_path(reference)?;
        let name = archive_name("media", &relative);
        archive
            .start_file(&name, options)
            .map_err(|error| AppError::io(format!("failed to add media `{name}`: {error}")))?;
        let mut source = fs::File::open(media_root.join(&relative))
            .map_err(|error| AppError::io(format!("failed to read media `{name}`: {error}")))?;
        std::io::copy(&mut source, &mut archive)
            .map_err(|error| AppError::io(format!("failed to write media `{name}`: {error}")))?;
    }
    archive
        .finish()
        .map_err(|error| AppError::io(format!("failed to finalize bundle: {error}")))?;
    fs::rename(&temporary, destination)
        .map_err(|error| AppError::io(format!("failed to publish bundle: {error}")))?;

    Ok(PortableExportResult {
        file_path: destination.to_string_lossy().to_string(),
        prompts: manifest.prompts.len(),
        revisions: manifest.revisions.len(),
        media_files: manifest.media_files.len(),
    })
}

fn load_bundle(path: &Path) -> Result<LoadedBundle, AppError> {
    let file = fs::File::open(path)
        .map_err(|error| AppError::io(format!("failed to open prompt bundle: {error}")))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| AppError::parse(format!("invalid prompt bundle ZIP: {error}")))?;
    let mut manifest = None;
    let mut media = Vec::new();
    let mut media_total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::parse(format!("invalid bundle entry: {error}")))?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| AppError::validation("bundle contains a traversal path"))?
            .to_path_buf();
        let normalized = archive_name("", &enclosed)
            .trim_start_matches('/')
            .to_string();
        if normalized == MANIFEST_NAME {
            if entry.size() > MAX_MANIFEST_BYTES || manifest.is_some() {
                return Err(AppError::validation(
                    "bundle manifest is missing or too large",
                ));
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| AppError::io(format!("failed to read manifest: {error}")))?;
            manifest =
                Some(serde_json::from_slice(&bytes).map_err(|error| {
                    AppError::parse(format!("invalid bundle manifest: {error}"))
                })?);
        } else if let Ok(relative) = enclosed.strip_prefix("media") {
            if entry.is_dir() {
                continue;
            }
            let relative = safe_relative_path(&relative.to_string_lossy())?;
            media_total = media_total.saturating_add(entry.size());
            if media_total > MAX_MEDIA_BYTES {
                return Err(AppError::validation("bundle media exceeds 100 MB"));
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| AppError::io(format!("failed to read bundle media: {error}")))?;
            media.push((relative, bytes));
        } else {
            return Err(AppError::validation(format!(
                "unsupported bundle entry `{normalized}`"
            )));
        }
    }
    let manifest = manifest.ok_or_else(|| AppError::validation("bundle has no manifest"))?;
    validate_manifest(&manifest)?;
    let payloads: HashSet<String> = media
        .iter()
        .map(|(path, _)| archive_name("", path).trim_start_matches('/').to_string())
        .collect();
    if payloads.len() != media.len() {
        return Err(AppError::validation(
            "bundle contains duplicate media payloads",
        ));
    }
    let expected_payloads = manifest
        .media_files
        .iter()
        .map(|path| {
            safe_relative_path(path)
                .map(|path| archive_name("", &path).trim_start_matches('/').to_string())
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if payloads != expected_payloads {
        return Err(AppError::validation(
            "bundle media payloads do not match the manifest",
        ));
    }
    Ok(LoadedBundle { manifest, media })
}

fn validate_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for value in values {
        if value.is_empty() || !seen.insert(value) {
            return Err(AppError::validation(format!(
                "bundle has an empty or duplicate {label} `{value}`"
            )));
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &PromptBundleManifest) -> Result<(), AppError> {
    if manifest.format_version != FORMAT_VERSION {
        return Err(AppError::validation(format!(
            "unsupported prompt bundle version {}",
            manifest.format_version
        )));
    }
    validate_unique(
        manifest.prompts.iter().map(|prompt| prompt.id.as_str()),
        "prompt id",
    )?;
    validate_unique(
        manifest
            .revisions
            .iter()
            .map(|revision| revision.id.as_str()),
        "revision id",
    )?;
    validate_unique(
        manifest.folders.iter().map(|folder| folder.id.as_str()),
        "folder id",
    )?;
    let folders_by_id: HashMap<&str, &Folder> = manifest
        .folders
        .iter()
        .map(|folder| (folder.id.as_str(), folder))
        .collect();
    for folder in &manifest.folders {
        let mut ancestors = HashSet::new();
        let mut parent = folder.parent_id.as_deref();
        while let Some(parent_id) = parent {
            let parent_folder = folders_by_id.get(parent_id).ok_or_else(|| {
                AppError::validation(format!(
                    "folder `{}` references missing parent `{parent_id}`",
                    folder.id
                ))
            })?;
            if !ancestors.insert(parent_id) {
                return Err(AppError::validation(format!(
                    "folder `{}` has a cyclic parent chain",
                    folder.id
                )));
            }
            parent = parent_folder.parent_id.as_deref();
        }
    }
    let prompt_ids: HashSet<&str> = manifest
        .prompts
        .iter()
        .map(|prompt| prompt.id.as_str())
        .collect();
    for prompt in &manifest.prompts {
        if let Some(folder_id) = prompt.folder_id.as_deref() {
            if !folders_by_id.contains_key(folder_id) {
                return Err(AppError::validation(format!(
                    "prompt `{}` references missing folder `{folder_id}`",
                    prompt.id
                )));
            }
        }
    }
    let revisions_by_id: HashMap<&str, &PromptVersion> = manifest
        .revisions
        .iter()
        .map(|revision| (revision.id.as_str(), revision))
        .collect();
    let mut prompt_versions = HashSet::new();
    for revision in &manifest.revisions {
        if !prompt_ids.contains(revision.prompt_id.as_str()) {
            return Err(AppError::validation(format!(
                "revision `{}` references missing prompt",
                revision.id
            )));
        }
        if revision.version < 1
            || !prompt_versions.insert((revision.prompt_id.as_str(), revision.version))
        {
            return Err(AppError::validation(format!(
                "revision `{}` has an invalid or duplicate version",
                revision.id
            )));
        }
        if let Some(folder_id) = revision.folder_id.as_deref() {
            if !folders_by_id.contains_key(folder_id) {
                return Err(AppError::validation(format!(
                    "revision `{}` references missing folder `{folder_id}`",
                    revision.id
                )));
            }
        }
        if let Some(parent) = revision.parent_revision_id.as_deref() {
            let Some(parent_revision) = revisions_by_id.get(parent) else {
                return Err(AppError::validation(format!(
                    "revision `{}` references missing parent",
                    revision.id
                )));
            };
            if parent_revision.prompt_id != revision.prompt_id
                || parent_revision.version >= revision.version
            {
                return Err(AppError::validation(format!(
                    "revision `{}` has an invalid parent",
                    revision.id
                )));
            }
        }
    }
    for path in &manifest.media_files {
        safe_relative_path(path)?;
    }
    Ok(())
}

fn require_encrypted(label: &str, value: Option<&str>) -> Result<(), AppError> {
    if value.is_some_and(|value| !crate::services::security::is_encrypted_value(value)) {
        return Err(AppError::validation(format!(
            "private bundle field `{label}` is not encrypted"
        )));
    }
    Ok(())
}

fn validate_private_bundle_key(
    manifest: &PromptBundleManifest,
    encryption_key: Option<&[u8]>,
) -> Result<(), AppError> {
    if !manifest.prompts.iter().any(|prompt| prompt.is_private)
        && !manifest
            .revisions
            .iter()
            .any(|revision| revision.is_private)
    {
        return Ok(());
    }
    let key = encryption_key.ok_or_else(|| {
        AppError::unauthorized(
            "unlock with the bundle encryption key before importing private prompts",
        )
    })?;

    for prompt in manifest.prompts.iter().filter(|prompt| prompt.is_private) {
        require_encrypted("prompt.description", prompt.description.as_deref())?;
        require_encrypted("prompt.systemPrompt", prompt.system_prompt.as_deref())?;
        require_encrypted("prompt.userPrompt", Some(&prompt.user_prompt))?;
        for message in &prompt.messages {
            require_encrypted("prompt.messages.content", Some(&message.content))?;
        }
        require_encrypted("prompt.source", prompt.source.as_deref())?;
        require_encrypted("prompt.notes", prompt.notes.as_deref())?;
        require_encrypted("prompt.lastAiResponse", prompt.last_ai_response.as_deref())?;
        crate::services::prompt::present_prompt(prompt.clone(), Some(key))?;
    }
    for revision in manifest
        .revisions
        .iter()
        .filter(|revision| revision.is_private)
    {
        require_encrypted("revision.description", revision.description.as_deref())?;
        require_encrypted("revision.systemPrompt", revision.system_prompt.as_deref())?;
        require_encrypted("revision.userPrompt", Some(&revision.user_prompt))?;
        for message in &revision.messages {
            require_encrypted("revision.messages.content", Some(&message.content))?;
        }
        require_encrypted("revision.source", revision.source.as_deref())?;
        require_encrypted("revision.notes", revision.notes.as_deref())?;
        require_encrypted("revision.aiResponse", revision.ai_response.as_deref())?;
        crate::services::prompt::present_version(revision.clone(), Some(key))?;
    }
    Ok(())
}

pub fn preview_bundle(conn: &Connection, path: &Path) -> Result<BundlePreview, AppError> {
    let loaded = load_bundle(path)?;
    let mut conflicts = 0;
    for prompt in &loaded.manifest.prompts {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM prompts WHERE id = ?1)",
                [&prompt.id],
                |row| row.get(0),
            )
            .map_err(|error| db_err("failed to preview prompt conflict", error))?;
        conflicts += usize::from(exists);
    }
    Ok(BundlePreview {
        format_version: loaded.manifest.format_version,
        prompts: loaded.manifest.prompts.len(),
        revisions: loaded.manifest.revisions.len(),
        folders: loaded.manifest.folders.len(),
        media_files: loaded.manifest.media_files.len(),
        additions: loaded.manifest.prompts.len() - conflicts,
        conflicts,
        private_prompts: loaded
            .manifest
            .prompts
            .iter()
            .filter(|prompt| prompt.is_private)
            .count(),
    })
}

fn insert_folder(
    tx: &Transaction<'_>,
    folder: &Folder,
    id: &str,
    parent_id: Option<&str>,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT OR IGNORE INTO folders (id,name,icon,parent_id,sort_order,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            id,
            folder.name,
            folder.icon,
            parent_id,
            folder.sort_order,
            iso8601_to_millis(&folder.created_at)?,
            folder.updated_at.as_deref().map(iso8601_to_millis).transpose()?,
        ],
    )
    .map_err(|error| db_err("failed to import folder", error))?;
    Ok(())
}

fn insert_folders(
    tx: &Transaction<'_>,
    folders: &[Folder],
    folder_map: &HashMap<String, String>,
) -> Result<(), AppError> {
    let mut inserted = HashSet::new();
    while inserted.len() < folders.len() {
        let mut progressed = false;
        for folder in folders {
            if inserted.contains(&folder.id)
                || folder
                    .parent_id
                    .as_ref()
                    .is_some_and(|parent| !inserted.contains(parent))
            {
                continue;
            }
            let parent = folder
                .parent_id
                .as_ref()
                .and_then(|parent| folder_map.get(parent))
                .map(String::as_str);
            insert_folder(tx, folder, &folder_map[&folder.id], parent)?;
            inserted.insert(folder.id.clone());
            progressed = true;
        }
        if !progressed {
            return Err(AppError::validation(
                "bundle folder hierarchy could not be ordered",
            ));
        }
    }
    Ok(())
}

fn insert_prompt(
    tx: &Transaction<'_>,
    prompt: &Prompt,
    id: &str,
    folder_id: Option<&str>,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO prompts (id,title,description,prompt_type,system_prompt,user_prompt,messages,variables,tags,folder_id,images,videos,is_favorite,is_pinned,is_private,current_version,usage_count,source,notes,last_ai_response,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
        params![
            id, prompt.title, prompt.description, enum_wire(&prompt.prompt_type)?,
            prompt.system_prompt, prompt.user_prompt, json(&prompt.messages)?, json(&prompt.variables)?, json(&prompt.tags)?,
            folder_id, json(&prompt.images)?, json(&prompt.videos)?, prompt.is_favorite,
            prompt.is_pinned, prompt.is_private, prompt.current_version, prompt.usage_count,
            prompt.source, prompt.notes, prompt.last_ai_response,
            iso8601_to_millis(&prompt.created_at)?, iso8601_to_millis(&prompt.updated_at)?,
        ],
    )
    .map_err(|error| db_err("failed to import prompt", error))?;
    Ok(())
}

fn insert_revision(
    tx: &Transaction<'_>,
    revision: &PromptVersion,
    id: &str,
    prompt_id: &str,
    parent_revision_id: Option<&str>,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO prompt_versions (id,prompt_id,version,system_prompt,user_prompt,messages,variables,title,description,prompt_type,tags,folder_id,images,videos,is_favorite,is_pinned,is_private,source,notes,note,ai_response,source_action,parent_revision_id,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
        params![
            id, prompt_id, revision.version, revision.system_prompt, revision.user_prompt, json(&revision.messages)?,
            json(&revision.variables)?, revision.title, revision.description,
            enum_wire(&revision.prompt_type)?, json(&revision.tags)?, revision.folder_id,
            json(&revision.images)?, json(&revision.videos)?, revision.is_favorite,
            revision.is_pinned, revision.is_private, revision.source, revision.notes,
            revision.note, revision.ai_response, enum_wire(&revision.source_action)?,
            parent_revision_id, iso8601_to_millis(&revision.created_at)?,
        ],
    )
    .map_err(|error| db_err("failed to import revision", error))?;
    Ok(())
}

fn replace_prompt(tx: &Transaction<'_>, incoming: &Prompt) -> Result<(), AppError> {
    tx.execute(
        "UPDATE prompts SET title=?1,description=?2,prompt_type=?3,system_prompt=?4,user_prompt=?5,messages=?6,variables=?7,tags=?8,folder_id=?9,images=?10,videos=?11,is_favorite=?12,is_pinned=?13,is_private=?14,source=?15,notes=?16,last_ai_response=?17,updated_at=?18 WHERE id=?19",
        params![
            incoming.title, incoming.description, enum_wire(&incoming.prompt_type)?,
            incoming.system_prompt, incoming.user_prompt, json(&incoming.messages)?, json(&incoming.variables)?,
            json(&incoming.tags)?, incoming.folder_id, json(&incoming.images)?,
            json(&incoming.videos)?, incoming.is_favorite, incoming.is_pinned,
            incoming.is_private, incoming.source, incoming.notes, incoming.last_ai_response,
            now_millis(), incoming.id,
        ],
    )
    .map_err(|error| db_err("failed to replace prompt", error))?;
    let replaced = tx
        .query_row(
            "SELECT * FROM prompts WHERE id = ?1",
            [&incoming.id],
            prompt_from_row,
        )
        .map_err(|error| db_err("failed to read replaced prompt", error))?;
    crate::services::version::append_snapshot(
        tx,
        &replaced,
        Some("Portable bundle replacement".into()),
        PromptRevisionSource::Replace,
        None,
    )?;
    Ok(())
}

pub fn import_bundle(
    conn: &Connection,
    path: &Path,
    policy: ImportConflictPolicy,
    data_root: &Path,
    backup_root: &Path,
    media_root: &Path,
    encryption_key: Option<&[u8]>,
) -> Result<PortableImportResult, AppError> {
    let loaded = load_bundle(path)?;
    validate_private_bundle_key(&loaded.manifest, encryption_key)?;
    let backup = crate::services::sync::backup_create(data_root, backup_root)?;

    let import_id = uuid::Uuid::new_v4().to_string();
    let staging = media_root.join(format!(".prompthub-import-{import_id}"));
    let folder_map: HashMap<String, String> = loaded
        .manifest
        .folders
        .iter()
        .map(|folder| {
            let id = if matches!(policy, ImportConflictPolicy::Duplicate) {
                uuid::Uuid::new_v4().to_string()
            } else {
                folder.id.clone()
            };
            (folder.id.clone(), id)
        })
        .collect();
    let prompt_map: HashMap<String, String> = loaded
        .manifest
        .prompts
        .iter()
        .map(|prompt| {
            let id = if matches!(policy, ImportConflictPolicy::Duplicate) {
                uuid::Uuid::new_v4().to_string()
            } else {
                prompt.id.clone()
            };
            (prompt.id.clone(), id)
        })
        .collect();
    let revision_map: HashMap<String, String> = loaded
        .manifest
        .revisions
        .iter()
        .map(|revision| {
            let id = if matches!(policy, ImportConflictPolicy::Duplicate) {
                uuid::Uuid::new_v4().to_string()
            } else {
                revision.id.clone()
            };
            (revision.id.clone(), id)
        })
        .collect();

    let mut created_media = Vec::new();
    let result = (|| -> Result<PortableImportResult, AppError> {
        for (relative, bytes) in &loaded.media {
            let destination = staging.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| AppError::io(format!("failed to stage media: {error}")))?;
            }
            fs::write(&destination, bytes)
                .map_err(|error| AppError::io(format!("failed to stage media: {error}")))?;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|error| db_err("failed to start bundle import", error))?;
        insert_folders(&tx, &loaded.manifest.folders, &folder_map)?;

        let mut added = 0;
        let mut skipped = 0;
        let mut replaced = 0;
        let mut skipped_prompt_ids = HashSet::new();
        for prompt in &loaded.manifest.prompts {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM prompts WHERE id = ?1)",
                    [&prompt.id],
                    |row| row.get(0),
                )
                .map_err(|error| db_err("failed to check import conflict", error))?;
            if exists && matches!(policy, ImportConflictPolicy::Skip) {
                skipped += 1;
                skipped_prompt_ids.insert(prompt.id.clone());
                continue;
            }
            if exists && matches!(policy, ImportConflictPolicy::Replace) {
                replace_prompt(&tx, prompt)?;
                replaced += 1;
                skipped_prompt_ids.insert(prompt.id.clone());
                continue;
            }
            let folder = prompt
                .folder_id
                .as_ref()
                .and_then(|folder| folder_map.get(folder))
                .map(String::as_str);
            insert_prompt(&tx, prompt, &prompt_map[&prompt.id], folder)?;
            added += 1;
        }
        for revision in &loaded.manifest.revisions {
            if skipped_prompt_ids.contains(&revision.prompt_id) {
                continue;
            }
            let parent = revision
                .parent_revision_id
                .as_ref()
                .and_then(|parent| revision_map.get(parent))
                .map(String::as_str);
            insert_revision(
                &tx,
                revision,
                &revision_map[&revision.id],
                &prompt_map[&revision.prompt_id],
                parent,
            )?;
        }

        for (relative, bytes) in &loaded.media {
            let staged = staging.join(relative);
            let destination = media_root.join(relative);
            if destination.exists() {
                let existing = fs::read(&destination).map_err(|error| {
                    AppError::io(format!("failed to inspect existing media: {error}"))
                })?;
                if existing != *bytes {
                    return Err(AppError::validation(format!(
                        "bundle media `{}` conflicts with an existing file",
                        relative.display()
                    )));
                }
                continue;
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AppError::io(format!("failed to create media directory: {error}"))
                })?;
            }
            fs::rename(&staged, &destination).map_err(|error| {
                AppError::io(format!("failed to install bundle media: {error}"))
            })?;
            created_media.push(destination);
        }

        tx.commit()
            .map_err(|error| db_err("failed to commit bundle import", error))?;

        Ok(PortableImportResult {
            added,
            skipped,
            replaced,
            backup_id: backup.id.clone(),
        })
    })();

    if result.is_err() {
        cleanup_import_files(&staging, &created_media);
    } else {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn cleanup_import_files(staging: &Path, created: &[PathBuf]) {
    for path in created {
        let _ = fs::remove_file(path);
    }
    let _ = fs::remove_dir_all(staging);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::prompt::{self, PromptCreate};
    use crate::storage::{create_memory_pool, init_schema};
    use std::sync::Mutex;

    #[test]
    fn bundle_round_trip_preserves_unicode_revisions_media_and_preview() {
        let source_pool = create_memory_pool().unwrap();
        let source = source_pool.get().unwrap();
        init_schema(&source).unwrap();
        let root = tempfile::tempdir().unwrap();
        let media = root.path().join("media");
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("image.png"), b"image-bytes").unwrap();
        prompt::create(
            &source,
            PromptCreate {
                title: "Unicode prompt".into(),
                user_prompt: "hello".into(),
                images: Some(vec!["image.png".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        let bundle = root.path().join("bundle.prompthub");
        export_bundle(&source, &media, &bundle).unwrap();

        let target_pool = create_memory_pool().unwrap();
        let target = target_pool.get().unwrap();
        init_schema(&target).unwrap();
        let preview = preview_bundle(&target, &bundle).unwrap();
        assert_eq!(preview.additions, 1);
        assert_eq!(preview.revisions, 1);
        assert_eq!(preview.media_files, 1);

        let data = root.path().join("data");
        let backups = root.path().join("backups");
        let target_media = root.path().join("target-media");
        fs::create_dir_all(&data).unwrap();
        let imported = import_bundle(
            &target,
            &bundle,
            ImportConflictPolicy::Skip,
            &data,
            &backups,
            &target_media,
            None,
        )
        .unwrap();
        assert_eq!(imported.added, 1);
        assert_eq!(crate::services::prompt::list(&target).unwrap().len(), 1);
        assert_eq!(
            fs::read_to_string(target_media.join("image.png")).unwrap(),
            "image-bytes"
        );
    }

    #[test]
    fn bundle_rejects_traversal_and_duplicate_ids_before_writes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("bad.prompthub");
        let file = fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("../escape", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert_eq!(load_bundle(&path).unwrap_err().code_str(), "VALIDATION");
        assert!(!root.path().join("escape").exists());
    }

    #[test]
    fn bundle_rejects_missing_folder_and_undeclared_media() {
        let source_pool = create_memory_pool().unwrap();
        let source = source_pool.get().unwrap();
        init_schema(&source).unwrap();
        prompt::create(
            &source,
            PromptCreate {
                title: "Broken manifest".into(),
                user_prompt: "body".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let BundleData {
            mut prompts,
            revisions,
            folders,
        } = load_all(&source).unwrap();
        prompts[0].folder_id = Some("missing-folder".into());
        let manifest = PromptBundleManifest {
            format_version: FORMAT_VERSION,
            exported_at: crate::storage::time::millis_to_iso8601(now_millis()),
            prompts,
            revisions,
            folders,
            media_files: Vec::new(),
        };

        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("bad-manifest.prompthub");
        let file = fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file(MANIFEST_NAME, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        zip.start_file("media/extra.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"extra").unwrap();
        zip.finish().unwrap();

        let error = load_bundle(&path).unwrap_err();
        assert_eq!(error.code_str(), "VALIDATION");
        assert!(error.message.contains("missing folder"));

        let valid = load_all(&source).unwrap();
        let manifest = PromptBundleManifest {
            format_version: FORMAT_VERSION,
            exported_at: crate::storage::time::millis_to_iso8601(now_millis()),
            prompts: valid.prompts,
            revisions: valid.revisions,
            folders: valid.folders,
            media_files: Vec::new(),
        };
        let path = root.path().join("extra-media.prompthub");
        let file = fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file(MANIFEST_NAME, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        zip.start_file("media/extra.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"extra").unwrap();
        zip.finish().unwrap();

        let error = load_bundle(&path).unwrap_err();
        assert_eq!(error.code_str(), "VALIDATION");
        assert!(error.message.contains("do not match"));
    }

    #[test]
    fn private_bundle_requires_the_matching_unlocked_key_before_writes() {
        let source_pool = create_memory_pool().unwrap();
        let source = source_pool.get().unwrap();
        init_schema(&source).unwrap();
        let encryption = Mutex::new(crate::state::EncryptionState::default());
        crate::services::security::set_master_password(&source, &encryption, "source-password")
            .unwrap();
        prompt::create_secure(
            &source,
            &encryption,
            PromptCreate {
                title: "Private".into(),
                user_prompt: "secret body".into(),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("private.prompthub");
        export_bundle(&source, &root.path().join("source-media"), &bundle).unwrap();

        let target_pool = create_memory_pool().unwrap();
        let target = target_pool.get().unwrap();
        init_schema(&target).unwrap();
        let wrong_key = [7_u8; 32];
        let backup_root = root.path().join("backups");
        let error = import_bundle(
            &target,
            &bundle,
            ImportConflictPolicy::Skip,
            &root.path().join("data"),
            &backup_root,
            &root.path().join("target-media"),
            Some(&wrong_key),
        )
        .unwrap_err();

        assert_eq!(error.code_str(), "UNAUTHORIZED");
        assert!(prompt::list(&target).unwrap().is_empty());
        assert!(!backup_root.exists());
    }

    #[test]
    fn media_conflict_rolls_back_database_and_cleans_staging() {
        let source_pool = create_memory_pool().unwrap();
        let source = source_pool.get().unwrap();
        init_schema(&source).unwrap();
        let root = tempfile::tempdir().unwrap();
        let source_media = root.path().join("source-media");
        fs::create_dir_all(&source_media).unwrap();
        fs::write(source_media.join("image.png"), b"incoming").unwrap();
        prompt::create(
            &source,
            PromptCreate {
                title: "Media".into(),
                user_prompt: "body".into(),
                images: Some(vec!["image.png".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        let bundle = root.path().join("media.prompthub");
        export_bundle(&source, &source_media, &bundle).unwrap();

        let target_pool = create_memory_pool().unwrap();
        let target = target_pool.get().unwrap();
        init_schema(&target).unwrap();
        let target_media = root.path().join("target-media");
        fs::create_dir_all(&target_media).unwrap();
        fs::write(target_media.join("image.png"), b"existing").unwrap();
        let data = root.path().join("data");
        fs::create_dir_all(&data).unwrap();

        let error = import_bundle(
            &target,
            &bundle,
            ImportConflictPolicy::Skip,
            &data,
            &root.path().join("backups"),
            &target_media,
            None,
        )
        .unwrap_err();

        assert_eq!(error.code_str(), "VALIDATION");
        assert!(prompt::list(&target).unwrap().is_empty());
        assert_eq!(
            fs::read(target_media.join("image.png")).unwrap(),
            b"existing"
        );
        assert!(fs::read_dir(&target_media).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".prompthub-import-")));
    }

    #[test]
    fn nested_folders_import_even_when_child_sorts_before_parent() {
        let source_pool = create_memory_pool().unwrap();
        let source = source_pool.get().unwrap();
        init_schema(&source).unwrap();
        source
            .execute_batch(
                r#"
                INSERT INTO folders (id,name,sort_order,created_at)
                  VALUES ('z-parent','Parent',0,0);
                INSERT INTO folders (id,name,parent_id,sort_order,created_at)
                  VALUES ('a-child','Child','z-parent',0,0);
                "#,
            )
            .unwrap();
        prompt::create(
            &source,
            PromptCreate {
                title: "Nested".into(),
                user_prompt: "body".into(),
                folder_id: Some("a-child".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let root = tempfile::tempdir().unwrap();
        let bundle = root.path().join("nested.prompthub");
        export_bundle(&source, &root.path().join("source-media"), &bundle).unwrap();
        let target_pool = create_memory_pool().unwrap();
        let target = target_pool.get().unwrap();
        init_schema(&target).unwrap();
        let data = root.path().join("data");
        fs::create_dir_all(&data).unwrap();

        import_bundle(
            &target,
            &bundle,
            ImportConflictPolicy::Skip,
            &data,
            &root.path().join("backups"),
            &root.path().join("target-media"),
            None,
        )
        .unwrap();

        let parent: String = target
            .query_row(
                "SELECT parent_id FROM folders WHERE id = 'a-child'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parent, "z-parent");
    }
}
