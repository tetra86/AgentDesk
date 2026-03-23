use std::fs;

use super::runtime_store::shared_agent_memory_root;

/// Read shared_knowledge.md from the shared_agent_memory directory.
/// Returns the file content wrapped in a [Shared Agent Knowledge] section,
/// or None if the file doesn't exist or is empty.
pub(super) fn load_shared_knowledge() -> Option<String> {
    let root = shared_agent_memory_root()?;
    let path = root.join("shared_knowledge.md");
    let content = fs::read_to_string(&path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("[Shared Agent Knowledge]\n{}", trimmed))
}
