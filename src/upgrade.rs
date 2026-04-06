use crate::backend;
use crate::config::{BackendType, JailConfig};
use crate::error::Result;
use crate::git_gpg::create_claude_json_in_container;
use crate::jail::JailManager;
use std::path::PathBuf;

pub async fn resolve_jail_name(name: Option<String>) -> Result<String> {
    if let Some(name) = name {
        Ok(name)
    } else {
        let cwd = std::env::current_dir()?;
        let workspace_dir = crate::agent_commands::get_git_root().unwrap_or(cwd);
        let matching_jails =
            crate::agent_commands::find_jails_for_directory(&workspace_dir).await?;

        let jail_name = if matching_jails.is_empty() {
            return Err(crate::error::JailError::Config(
                "No jails found for this directory. Create one first.".to_string(),
            ));
        } else if matching_jails.len() == 1 {
            matching_jails[0].clone()
        } else {
            crate::agent_commands::select_jail(&matching_jails)?
        };

        tracing::info!("Auto-detected jail: {}", jail_name);
        Ok(jail_name)
    }
}

pub async fn upgrade_single_jail(
    jail_name: &str,
    image: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let temp_config = JailConfig {
        name: jail_name.to_string(),
        ..Default::default()
    };
    let temp_jail = JailManager::new(temp_config);

    if !temp_jail.exists().await? {
        return Err(crate::error::JailError::NotFound(format!(
            "Jail '{jail_name}' does not exist"
        )));
    }

    let old_config = temp_jail.inspect().await?;
    tracing::info!("Current jail configuration: {:?}", old_config);

    let new_image = image.unwrap_or_else(|| old_config.base_image.clone());

    if !force {
        use std::io::{BufRead, Write};
        println!("Jail '{jail_name}' will be upgraded:");
        println!("  Current image: {}", old_config.base_image);
        println!("  New image:     {new_image}");
        println!("\nThis will:");
        println!("  1. Save the current configuration");
        println!("  2. Remove the existing jail");
        println!("  3. Recreate the jail with the new image");
        println!("  4. Restore the configuration (mounts, env, limits)");
        print!("\nProceed with upgrade? [y/N] ");
        std::io::stdout().flush()?;
        let stdin = std::io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            tracing::info!("Upgrade aborted");
            return Ok(());
        }
    }

    tracing::info!("Upgrading jail '{}'...", jail_name);
    tracing::info!("Removing old jail...");
    temp_jail.remove(false).await?;
    tracing::info!("Old jail removed");

    tracing::info!("Creating new jail with image '{}'...", new_image);
    let mut builder = crate::jail::JailBuilder::new(jail_name)
        .backend(old_config.backend)
        .base_image(new_image.clone())
        .network(old_config.network.enabled, old_config.network.private)
        .verbose(verbose);

    for (key, value) in &old_config.environment {
        builder = builder.env(key.clone(), value.clone());
    }

    for mount in &old_config.bind_mounts {
        builder = builder.bind_mount(mount.source.clone(), mount.target.clone(), mount.readonly);
    }

    for port_mapping in &old_config.port_mappings {
        builder = builder.port_mapping(
            port_mapping.host_port,
            port_mapping.container_port,
            &port_mapping.protocol,
        );
    }

    if let Some(memory) = old_config.limits.memory_mb {
        builder = builder.memory_limit(memory);
    }
    if let Some(cpu) = old_config.limits.cpu_quota {
        builder = builder.cpu_quota(cpu);
    }

    let new_jail = builder.build();
    new_jail.create().await?;

    if jail_name.ends_with("-claude") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let home_path = PathBuf::from(&home);
        if let Err(e) = create_claude_json_in_container(&home_path, &new_jail).await {
            tracing::warn!("Failed to create .claude.json in container: {}", e);
        }
    }

    println!("✓ Jail '{jail_name}' successfully upgraded to image '{new_image}'");
    tracing::info!("Upgrade completed successfully");

    Ok(())
}

pub async fn upgrade_all_jails(image: Option<String>, force: bool, verbose: bool) -> Result<()> {
    let backends = BackendType::all_available();
    if backends.is_empty() {
        tracing::warn!("No backends are available on this system");
        return Ok(());
    }

    let mut all_jails = Vec::new();

    for backend_type in &backends {
        let temp_config = JailConfig {
            name: "temp".to_string(),
            backend: *backend_type,
            ..Default::default()
        };
        let backend = backend::create_backend(&temp_config);
        let jails = backend.list_all().await?;

        for jail_name in jails {
            all_jails.push((jail_name, *backend_type));
        }
    }

    if all_jails.is_empty() {
        println!("No jails found to upgrade");
        return Ok(());
    }

    tracing::info!("Found {} jail(s) to upgrade", all_jails.len());

    if !force {
        use std::io::{BufRead, Write};
        println!(
            "The following {} jail(s) will be upgraded:",
            all_jails.len()
        );
        for (jail_name, backend_type) in &all_jails {
            println!("  - {jail_name} (backend: {backend_type:?})");
        }
        if let Some(ref img) = image {
            println!("\nAll jails will be upgraded to image: {img}");
        } else {
            println!("\nEach jail will be upgraded to its current image (refreshed)");
        }
        print!("\nProceed with upgrade? [y/N] ");
        std::io::stdout().flush()?;
        let stdin = std::io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            tracing::info!("Upgrade aborted");
            return Ok(());
        }
    }

    let mut success_count = 0;
    let mut error_count = 0;

    for (jail_name, _backend_type) in all_jails {
        tracing::info!("Upgrading jail: {}", jail_name);
        match upgrade_single_jail(&jail_name, image.clone(), true, verbose).await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                tracing::error!("Failed to upgrade jail {}: {}", jail_name, e);
                error_count += 1;
            }
        }
    }

    println!("\n✓ Upgrade complete: {success_count} succeeded, {error_count} failed");
    tracing::info!("Upgrade-all operation completed");

    Ok(())
}
