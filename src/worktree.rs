use crate::error::{JailError, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Information about a git worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Path to the worktree directory (current working directory)
    pub worktree_path: PathBuf,

    /// Path to the main repository's .git directory
    pub main_git_dir: PathBuf,
}

/// Detect if the current directory is a git worktree
/// Returns WorktreeInfo if it is a worktree, None otherwise
pub fn detect_worktree(dir: &Path) -> Result<Option<WorktreeInfo>> {
    let git_path = dir.join(".git");

    // Check if .git exists
    if !git_path.exists() {
        debug!("No .git found at {}", git_path.display());
        return Ok(None);
    }

    // Check if .git is a file (worktree indicator) rather than a directory
    if !git_path.is_file() {
        debug!(
            ".git at {} is a directory (regular repo, not worktree)",
            git_path.display()
        );
        return Ok(None);
    }

    // Read the .git file
    let git_content = std::fs::read_to_string(&git_path)
        .map_err(|e| JailError::Config(format!("Failed to read .git file: {}", e)))?;

    // Parse the gitdir line
    let gitdir_line = git_content.trim();
    if !gitdir_line.starts_with("gitdir: ") {
        return Err(JailError::Config(format!(
            "Invalid .git file format, expected 'gitdir: <path>', got: {}",
            gitdir_line
        )));
    }

    let gitdir_path_str = gitdir_line.strip_prefix("gitdir: ").unwrap().trim();
    let worktree_git_dir = PathBuf::from(gitdir_path_str);

    // Make it absolute if it's relative
    let worktree_git_dir = if worktree_git_dir.is_relative() {
        dir.join(&worktree_git_dir).canonicalize().map_err(|e| {
            JailError::Config(format!(
                "Failed to resolve worktree git directory path {}: {}",
                worktree_git_dir.display(),
                e
            ))
        })?
    } else {
        worktree_git_dir.canonicalize().map_err(|e| {
            JailError::Config(format!(
                "Failed to resolve worktree git directory path {}: {}",
                worktree_git_dir.display(),
                e
            ))
        })?
    };

    // Extract main repo .git directory
    // worktree_git_dir is like: /path/to/main-repo/.git/worktrees/feature-branch
    // We need to get: /path/to/main-repo/.git

    // Go up two levels: remove "feature-branch" and "worktrees"
    let main_git_dir = worktree_git_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            JailError::Config(format!(
                "Unexpected worktree git directory structure: {}",
                worktree_git_dir.display()
            ))
        })?
        .to_path_buf();

    // Verify this is actually a .git directory
    if !main_git_dir.join("config").exists() {
        return Err(JailError::Config(format!(
            "Expected main git directory at {} but it doesn't appear to be a git directory",
            main_git_dir.display()
        )));
    }

    info!("Detected git worktree:");
    info!("  Worktree path: {}", dir.display());
    info!("  Main .git dir: {}", main_git_dir.display());

    Ok(Some(WorktreeInfo {
        worktree_path: dir.to_path_buf(),
        main_git_dir,
    }))
}

/// Get all parent directories that need to be created in the container
/// Returns a list of directory paths to create, in order (parent to child)
pub fn get_required_parent_dirs(paths: &[&Path]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for path in paths {
        let mut current = path.parent();
        let mut path_dirs = Vec::new();

        while let Some(parent) = current {
            if parent == Path::new("/") {
                break;
            }
            path_dirs.push(parent.to_path_buf());
            current = parent.parent();
        }

        // Reverse so we get parent-to-child order
        path_dirs.reverse();

        for dir in path_dirs {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }

    // Sort to ensure parent directories come before children
    dirs.sort();
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_required_parent_dirs() {
        let path1 = Path::new("/home/user/work/feature");
        let path2 = Path::new("/home/user/projects/main/.git");

        let dirs = get_required_parent_dirs(&[path1, path2]);

        // Should include: /home, /home/user, /home/user/work, /home/user/projects, /home/user/projects/main
        assert!(dirs.contains(&PathBuf::from("/home")));
        assert!(dirs.contains(&PathBuf::from("/home/user")));
        assert!(dirs.contains(&PathBuf::from("/home/user/work")));
        assert!(dirs.contains(&PathBuf::from("/home/user/projects")));
        assert!(dirs.contains(&PathBuf::from("/home/user/projects/main")));
    }

    #[test]
    fn test_get_required_parent_dirs_ordering() {
        let path = Path::new("/a/b/c/d");
        let dirs = get_required_parent_dirs(&[path]);

        // Ensure parent comes before child
        let a_pos = dirs.iter().position(|p| p == Path::new("/a"));
        let b_pos = dirs.iter().position(|p| p == Path::new("/a/b"));
        let c_pos = dirs.iter().position(|p| p == Path::new("/a/b/c"));

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }
}
