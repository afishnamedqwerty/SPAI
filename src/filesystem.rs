//! Filesystem integration for agent document access
//!
//! Inspired by Letta's folder system, allows agents to:
//! - Attach folders and files
//! - Search file contents
//! - Open and read files
//! - Grep through files

use crate::error::{Error, Result};
use crate::tools::{JsonSchema, Tool, ToolContext, ToolOutput};
use crate::types::AgentId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for an attached folder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FolderId(Uuid);

impl FolderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An attached folder accessible to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedFolder {
    /// Unique identifier
    pub id: FolderId,

    /// Human-readable name
    pub name: String,

    /// Absolute path to the folder
    pub path: PathBuf,

    /// Description of folder contents
    pub description: String,

    /// File patterns to include (e.g., ["*.pdf", "*.txt"])
    pub include_patterns: Vec<String>,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Whether to index subdirectories
    pub recursive: bool,

    /// Cached list of files (updated periodically)
    pub files: Vec<String>,
}

impl AttachedFolder {
    /// Create a new attached folder
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: FolderId::new(),
            name: name.into(),
            path: path.into(),
            description: String::new(),
            include_patterns: vec!["*".to_string()],
            exclude_patterns: Vec::new(),
            recursive: true,
            files: Vec::new(),
        }
    }

    /// Scan the folder and update the file list
    pub fn scan_files(&mut self) -> Result<()> {
        self.files.clear();

        if !self.path.exists() {
            return Err(Error::config(format!(
                "Folder path does not exist: {}",
                self.path.display()
            )));
        }

        let root_path = self.path.clone();
        self.scan_directory(&root_path)?;
        Ok(())
    }

    fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let relative_path = path.strip_prefix(&self.path).unwrap_or(&path);
                self.files.push(relative_path.to_string_lossy().to_string());
            } else if path.is_dir() && self.recursive {
                self.scan_directory(&path)?;
            }
        }

        Ok(())
    }

    /// Get full path to a file
    pub fn get_file_path(&self, relative_path: &str) -> PathBuf {
        self.path.join(relative_path)
    }
}

/// Filesystem manager for agent file access
#[derive(Debug, Clone)]
pub struct FilesystemManager {
    /// Attached folders
    folders: Arc<RwLock<HashMap<FolderId, AttachedFolder>>>,

    /// Mapping of agent ID to attached folder IDs
    agent_folders: Arc<RwLock<HashMap<AgentId, Vec<FolderId>>>>,
}

impl FilesystemManager {
    /// Create a new filesystem manager
    pub fn new() -> Self {
        Self {
            folders: Arc::new(RwLock::new(HashMap::new())),
            agent_folders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and attach a folder
    pub async fn create_folder(
        &self,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Result<FolderId> {
        let mut folder = AttachedFolder::new(name, path);
        folder.scan_files()?;

        let id = folder.id;
        let mut folders = self.folders.write().await;
        folders.insert(id, folder);

        Ok(id)
    }

    /// Attach a folder to an agent
    pub async fn attach_folder(&self, agent_id: AgentId, folder_id: FolderId) {
        let mut agent_folders = self.agent_folders.write().await;
        agent_folders
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(folder_id);
    }

    /// Get all folders attached to an agent
    pub async fn get_agent_folders(&self, agent_id: AgentId) -> Vec<AttachedFolder> {
        let agent_folders = self.agent_folders.read().await;
        let folders = self.folders.read().await;

        if let Some(folder_ids) = agent_folders.get(&agent_id) {
            folder_ids
                .iter()
                .filter_map(|id| folders.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get a specific folder
    pub async fn get_folder(&self, folder_id: FolderId) -> Option<AttachedFolder> {
        let folders = self.folders.read().await;
        folders.get(&folder_id).cloned()
    }
}

impl Default for FilesystemManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool for opening and reading a file
pub struct OpenFileTool {
    fs_manager: Arc<FilesystemManager>,
    agent_id: AgentId,
}

impl OpenFileTool {
    pub fn new(fs_manager: Arc<FilesystemManager>, agent_id: AgentId) -> Self {
        Self {
            fs_manager,
            agent_id,
        }
    }
}

#[async_trait]
impl Tool for OpenFileTool {
    fn id(&self) -> &str {
        "open_file"
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn description(&self) -> &str {
        "Open and read the contents of a file from attached folders. \
         Use this to access research papers, documents, or any text files."
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "file_path".to_string(),
            json!({
                "type": "string",
                "description": "Path to the file relative to an attached folder (use list_files first)."
            }),
        );

        JsonSchema::object(properties).with_required(vec!["file_path".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| Error::tool_execution("open_file", "Missing file_path"))?;

        // Search through attached folders
        let folders = self.fs_manager.get_agent_folders(self.agent_id).await;

        for folder in folders {
            if folder.files.contains(&file_path.to_string()) {
                let full_path = folder.get_file_path(file_path);

                match fs::read_to_string(&full_path) {
                    Ok(content) => {
                        return Ok(ToolOutput::success_with_data(
                            format!("File: {}\n\n{}", file_path, content),
                            json!({
                                "file_path": file_path,
                                "size": content.len(),
                                "content": content
                            }),
                        ));
                    }
                    Err(e) => {
                        return Err(Error::tool_execution(
                            "open_file",
                            format!("Failed to read file: {}", e),
                        ));
                    }
                }
            }
        }

        Err(Error::tool_execution(
            "open_file",
            format!("File '{}' not found in any attached folder", file_path),
        ))
    }
}

/// Tool for searching file contents
pub struct SearchFilesTool {
    fs_manager: Arc<FilesystemManager>,
    agent_id: AgentId,
}

impl SearchFilesTool {
    pub fn new(fs_manager: Arc<FilesystemManager>, agent_id: AgentId) -> Self {
        Self {
            fs_manager,
            agent_id,
        }
    }
}

#[async_trait]
impl Tool for SearchFilesTool {
    fn id(&self) -> &str {
        "search_files"
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn description(&self) -> &str {
        "Search for text across all files in attached folders. \
         Returns matching files with context around the match."
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_string(),
            json!({
                "type": "string",
                "description": "Text to search for in files"
            }),
        );
        properties.insert(
            "max_results".to_string(),
            json!({
                "type": "integer",
                "description": "Maximum number of results to return (default: 10)"
            }),
        );

        JsonSchema::object(properties).with_required(vec!["query".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| Error::tool_execution("search_files", "Missing query"))?;

        let max_results = params["max_results"].as_u64().unwrap_or(10) as usize;

        let folders = self.fs_manager.get_agent_folders(self.agent_id).await;
        let mut results = Vec::new();

        for folder in folders {
            for file_path in &folder.files {
                let full_path = folder.get_file_path(file_path);

                if let Ok(content) = fs::read_to_string(&full_path) {
                    if content.contains(query) {
                        // Find context around matches
                        let lines: Vec<&str> = content.lines().collect();
                        for (line_num, line) in lines.iter().enumerate() {
                            if line.contains(query) {
                                results.push(json!({
                                    "file": file_path,
                                    "line": line_num + 1,
                                    "content": line.trim()
                                }));

                                if results.len() >= max_results {
                                    break;
                                }
                            }
                        }
                    }
                }

                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(ToolOutput::success_with_data(
            format!("Found {} matches for '{}'", results.len(), query),
            json!({
                "query": query,
                "results": results,
                "count": results.len()
            }),
        ))
    }
}

/// Tool for listing all available files
pub struct ListFilesTool {
    fs_manager: Arc<FilesystemManager>,
    agent_id: AgentId,
}

impl ListFilesTool {
    pub fn new(fs_manager: Arc<FilesystemManager>, agent_id: AgentId) -> Self {
        Self {
            fs_manager,
            agent_id,
        }
    }
}

#[async_trait]
impl Tool for ListFilesTool {
    fn id(&self) -> &str {
        "list_files"
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn description(&self) -> &str {
        "List all files available in attached folders. \
         Use this to see what documents you have access to."
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema::object(HashMap::new())
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let folders = self.fs_manager.get_agent_folders(self.agent_id).await;

        let mut all_files = Vec::new();
        for folder in folders {
            for file in &folder.files {
                all_files.push(json!({
                    "folder": folder.name,
                    "path": file
                }));
            }
        }

        Ok(ToolOutput::success_with_data(
            format!("Found {} files across attached folders", all_files.len()),
            json!({
                "files": all_files,
                "count": all_files.len()
            }),
        ))
    }
}

/// Create all filesystem tools for an agent
pub fn create_filesystem_tools(
    fs_manager: Arc<FilesystemManager>,
    agent_id: AgentId,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(OpenFileTool::new(fs_manager.clone(), agent_id)),
        Arc::new(SearchFilesTool::new(fs_manager.clone(), agent_id)),
        Arc::new(ListFilesTool::new(fs_manager, agent_id)),
    ]
}
