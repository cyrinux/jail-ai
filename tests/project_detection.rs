use jail_ai::project_detection::detect_project_type_with_options;
use jail_ai::ProjectType;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn test_detect_rust_project() {
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    File::create(cargo_toml).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Rust);
}

#[test]
fn test_detect_golang_project() {
    let temp_dir = TempDir::new().unwrap();
    let go_mod = temp_dir.path().join("go.mod");
    File::create(go_mod).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Golang);
}

#[test]
fn test_detect_python_project() {
    let temp_dir = TempDir::new().unwrap();
    let requirements = temp_dir.path().join("requirements.txt");
    File::create(requirements).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Python);
}

#[test]
fn test_detect_nodejs_project() {
    let temp_dir = TempDir::new().unwrap();
    let package_json = temp_dir.path().join("package.json");
    File::create(package_json).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::NodeJS);
}

#[test]
fn test_detect_java_project() {
    let temp_dir = TempDir::new().unwrap();
    let pom_xml = temp_dir.path().join("pom.xml");
    File::create(pom_xml).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Java);
}

#[test]
fn test_detect_multi_project() {
    let temp_dir = TempDir::new().unwrap();
    File::create(temp_dir.path().join("Cargo.toml")).unwrap();
    File::create(temp_dir.path().join("package.json")).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
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

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Nix);
}

#[test]
fn test_detect_generic_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Generic);
}

#[test]
fn test_detect_nix_project_with_no_nix() {
    let temp_dir = TempDir::new().unwrap();
    let flake_nix = temp_dir.path().join("flake.nix");
    File::create(flake_nix).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), true);
    assert_eq!(project_type, ProjectType::Generic);

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Nix);
}

#[test]
fn test_nix_takes_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let flake_nix = temp_dir.path().join("flake.nix");
    File::create(flake_nix).unwrap();
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    File::create(cargo_toml).unwrap();
    let package_json = temp_dir.path().join("package.json");
    File::create(package_json).unwrap();

    let project_type = detect_project_type_with_options(temp_dir.path(), false);
    assert_eq!(project_type, ProjectType::Nix);
}
