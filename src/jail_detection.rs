use crate::backend;
use crate::cli::Commands;
use crate::config::{BackendType, JailConfig};
use crate::error::Result;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn auto_detect_jail_name() -> Result<String> {
    let cwd = std::env::current_dir()?;
    let workspace_dir = get_git_root().unwrap_or(cwd);
    let jail_name = Commands::generate_jail_name(&workspace_dir);
    info!("Auto-detected jail name from workspace: {}", jail_name);
    Ok(jail_name)
}

pub fn get_git_root() -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let git_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !git_root.is_empty() {
                let path = PathBuf::from(git_root);
                if path.exists() {
                    info!("Found git root: {}", path.display());
                    return Some(path);
                }
            }
        }
        _ => {}
    }

    None
}

pub fn validate_workspace_directory(workspace_dir: &Path) -> Result<()> {
    use crate::error::JailError;

    let workspace_dir = workspace_dir.canonicalize().map_err(JailError::Io)?;

    let home_dir = std::env::var("HOME")
        .map_err(|_| JailError::Config("HOME environment variable not set".to_string()))?;
    let home_path = PathBuf::from(&home_dir)
        .canonicalize()
        .map_err(JailError::Io)?;

    if workspace_dir == home_path {
        return Err(JailError::UnsafeWorkspace(format!(
            "Cannot run jail-ai in home directory root: {}",
            workspace_dir.display()
        )));
    }

    let system_dirs = [
        "/",
        "/bin",
        "/sbin",
        "/usr",
        "/usr/bin",
        "/usr/sbin",
        "/usr/local",
        "/etc",
        "/var",
        "/lib",
        "/lib64",
        "/opt",
        "/root",
        "/sys",
        "/proc",
        "/dev",
    ];

    for system_dir in &system_dirs {
        if let Ok(system_path) = PathBuf::from(system_dir).canonicalize() {
            if workspace_dir == system_path {
                return Err(JailError::UnsafeWorkspace(format!(
                    "Cannot run jail-ai in system directory: {}",
                    workspace_dir.display()
                )));
            }
        }
    }

    Ok(())
}

pub async fn find_jails_for_directory(workspace_dir: &Path) -> Result<Vec<String>> {
    let base_name = Commands::generate_jail_name(workspace_dir);
    let backend_type = BackendType::detect();

    let temp_config = JailConfig {
        name: "temp".to_string(),
        backend: backend_type,
        ..Default::default()
    };
    let backend = backend::create_backend(&temp_config);

    let all_jails = backend.list_all().await?;

    let matching_jails: Vec<String> = all_jails
        .into_iter()
        .filter(|name| name.starts_with(&base_name) && name.len() > base_name.len())
        .collect();

    Ok(matching_jails)
}

pub fn extract_agent_name(jail_name: &str) -> &'static str {
    if let Some(agent) = jail_name.rsplit("__").next() {
        if let Some(a) = crate::agents::Agent::from_str(agent) {
            return a.normalized_name();
        }
    }
    "unknown"
}

pub fn select_jail(jails: &[String]) -> Result<String> {
    use std::io::{BufRead, Write};

    if jails.is_empty() {
        return Err(crate::error::JailError::Config(
            "No jails to select from".to_string(),
        ));
    }

    if jails.len() == 1 {
        return Ok(jails[0].clone());
    }

    println!("Multiple jails found for this directory:");
    for (i, jail) in jails.iter().enumerate() {
        let agent = extract_agent_name(jail);
        println!("  {} - {}", i + 1, jail);
        println!("      (agent: {})", agent);
    }

    print!("\nSelect a jail (1-{}) or 'q' to quit: ", jails.len());
    std::io::stdout().flush()?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let choice = line.trim();

    if choice.eq_ignore_ascii_case("q") {
        return Err(crate::error::JailError::Config("Aborted".to_string()));
    }

    let idx: usize = choice.parse().map_err(|_| {
        crate::error::JailError::Config("Invalid selection".to_string())
    })?;

    let idx = idx.saturating_sub(1);

    if idx >= jails.len() {
        return Err(crate::error::JailError::Config("Invalid selection".to_string()));
    }

    Ok(jails[idx].clone())
}