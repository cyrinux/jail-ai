use std::path::Path;
use tracing::{debug, info};

/// Detected project type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Golang,
    Python,
    NodeJS,
    Java,
    /// Multiple project types detected
    Multi(Vec<ProjectType>),
    /// No specific project type detected
    Generic,
}

impl ProjectType {
    /// Get the language layer name for this project type
    pub fn language_layer(&self) -> &'static str {
        match self {
            ProjectType::Rust => "rust",
            ProjectType::Golang => "golang",
            ProjectType::Python => "python",
            ProjectType::NodeJS => "nodejs",
            ProjectType::Java => "java",
            ProjectType::Multi(_) => "multi",
            ProjectType::Generic => "base",
        }
    }

    /// Get all language layers for this project type
    pub fn language_layers(&self) -> Vec<&'static str> {
        match self {
            ProjectType::Multi(types) => types.iter().map(|t| t.language_layer()).collect(),
            _ => vec![self.language_layer()],
        }
    }

    /// Check if this project type includes a specific language
    pub fn includes(&self, language: &str) -> bool {
        match self {
            ProjectType::Multi(types) => types.iter().any(|t| t.language_layer() == language),
            _ => self.language_layer() == language,
        }
    }
}

/// Detect project type based on files in the directory
pub fn detect_project_type(path: &Path) -> ProjectType {
    let mut detected_types = Vec::new();

    // Check for Rust project
    if path.join("Cargo.toml").exists() {
        debug!("Detected Rust project (Cargo.toml)");
        detected_types.push(ProjectType::Rust);
    }

    // Check for Go project
    if path.join("go.mod").exists() || path.join("go.sum").exists() {
        debug!("Detected Go project (go.mod/go.sum)");
        detected_types.push(ProjectType::Golang);
    }

    // Check for Python project
    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("Pipfile").exists()
        || path.join("poetry.lock").exists()
    {
        debug!("Detected Python project");
        detected_types.push(ProjectType::Python);
    }

    // Check for Node.js project
    if path.join("package.json").exists() {
        debug!("Detected Node.js project (package.json)");
        detected_types.push(ProjectType::NodeJS);
    }

    // Check for Java project
    if path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
    {
        debug!("Detected Java project");
        detected_types.push(ProjectType::Java);
    }

    match detected_types.len() {
        0 => {
            info!("No specific project type detected, using base image");
            ProjectType::Generic
        }
        1 => {
            let project_type = detected_types.into_iter().next().unwrap();
            info!("Detected project type: {:?}", project_type);
            project_type
        }
        _ => {
            info!("Detected multiple project types: {:?}", detected_types);
            ProjectType::Multi(detected_types)
        }
    }
}

/// Determine which agent is needed based on command
pub fn detect_agent_type(agent_name: &str) -> Option<&'static str> {
    match agent_name.to_lowercase().as_str() {
        "claude" => Some("claude"),
        "copilot" => Some("copilot"),
        "cursor" => Some("cursor"),
        "gemini" => Some("gemini"),
        "codex" => Some("codex"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        File::create(cargo_toml).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Rust);
    }

    #[test]
    fn test_detect_golang_project() {
        let temp_dir = TempDir::new().unwrap();
        let go_mod = temp_dir.path().join("go.mod");
        File::create(go_mod).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Golang);
    }

    #[test]
    fn test_detect_python_project() {
        let temp_dir = TempDir::new().unwrap();
        let requirements = temp_dir.path().join("requirements.txt");
        File::create(requirements).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Python);
    }

    #[test]
    fn test_detect_nodejs_project() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        File::create(package_json).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::NodeJS);
    }

    #[test]
    fn test_detect_java_project() {
        let temp_dir = TempDir::new().unwrap();
        let pom_xml = temp_dir.path().join("pom.xml");
        File::create(pom_xml).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Java);
    }

    #[test]
    fn test_detect_multi_project() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("Cargo.toml")).unwrap();
        File::create(temp_dir.path().join("package.json")).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        if let ProjectType::Multi(types) = project_type {
            assert_eq!(types.len(), 2);
            assert!(types.contains(&ProjectType::Rust));
            assert!(types.contains(&ProjectType::NodeJS));
        } else {
            panic!("Expected Multi project type");
        }
    }

    #[test]
    fn test_detect_generic_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Generic);
    }

    #[test]
    fn test_detect_agent_type() {
        assert_eq!(detect_agent_type("claude"), Some("claude"));
        assert_eq!(detect_agent_type("copilot"), Some("copilot"));
        assert_eq!(detect_agent_type("cursor"), Some("cursor"));
        assert_eq!(detect_agent_type("gemini"), Some("gemini"));
        assert_eq!(detect_agent_type("codex"), Some("codex"));
        assert_eq!(detect_agent_type("unknown"), None);
    }
}
