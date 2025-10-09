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
    Nix,
    Php,
    Cpp,
    CSharp,
    Terraform,
    Kubernetes,
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
            ProjectType::Nix => "nix",
            ProjectType::Php => "php",
            ProjectType::Cpp => "cpp",
            ProjectType::CSharp => "csharp",
            ProjectType::Terraform => "terraform",
            ProjectType::Kubernetes => "kubernetes",
            ProjectType::Multi(_) => "multi",
            ProjectType::Generic => "base",
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

    // Check for Nix project
    if path.join("flake.nix").exists() {
        debug!("Detected Nix project (flake.nix)");
        detected_types.push(ProjectType::Nix);
    }

    // Check for PHP project
    if path.join("composer.json").exists()
        || path.join("composer.lock").exists()
        || path.join("index.php").exists()
    {
        debug!("Detected PHP project");
        detected_types.push(ProjectType::Php);
    }

    // Check for C/C++ project
    if path.join("CMakeLists.txt").exists()
        || path.join("configure.ac").exists()
        || path.join("meson.build").exists()
    {
        debug!("Detected C/C++ project");
        detected_types.push(ProjectType::Cpp);
    }

    // Check for C# project
    if path.join(".csproj").exists()
        || path.join(".sln").exists()
        || path
            .read_dir()
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(Result::ok)
                    .find(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "csproj" || ext == "sln")
                            .unwrap_or(false)
                    })
            })
            .is_some()
    {
        debug!("Detected C# project");
        detected_types.push(ProjectType::CSharp);
    }

    // Check for Terraform project
    if path
        .read_dir()
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .find(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "tf" || ext == "tfvars")
                        .unwrap_or(false)
                })
        })
        .is_some()
        || path.join("terraform.tfstate").exists()
        || path.join(".terraform").exists()
        || path.join(".terraform.lock.hcl").exists()
    {
        debug!("Detected Terraform project");
        detected_types.push(ProjectType::Terraform);
    }

    // Check for Kubernetes project
    if path
        .read_dir()
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .find(|e| {
                    let filename = e.file_name();
                    let filename_str = filename.to_string_lossy();
                    filename_str.ends_with(".yaml") || filename_str.ends_with(".yml")
                })
        })
        .is_some()
        && (path.join("kustomization.yaml").exists()
            || path.join("Chart.yaml").exists()
            || path.join("values.yaml").exists()
            || path.join("k8s").exists()
            || path.join("kubernetes").exists())
    {
        debug!("Detected Kubernetes project");
        detected_types.push(ProjectType::Kubernetes);
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
    fn test_detect_nix_project() {
        let temp_dir = TempDir::new().unwrap();
        let flake_nix = temp_dir.path().join("flake.nix");
        File::create(flake_nix).unwrap();

        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Nix);
    }

    #[test]
    fn test_detect_generic_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_type = detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Generic);
    }
}
