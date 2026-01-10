use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::output::OutputMode;
use crate::theme::Theme;
use crate::uninstall;

const REPO: &str = "jpaulpoliquit/wole";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Detect the current system architecture
fn detect_architecture() -> Result<String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        
        // Try to detect via environment variables first
        if let Ok(arch) = env::var("PROCESSOR_ARCHITECTURE") {
            match arch.as_str() {
                "AMD64" => return Ok("x86_64".to_string()),
                "ARM64" => return Ok("arm64".to_string()),
                "x86" => {
                    // Could be 32-bit system or 32-bit process on 64-bit system
                    if let Ok(arch6432) = env::var("PROCESSOR_ARCHITEW6432") {
                        match arch6432.as_str() {
                            "AMD64" => return Ok("x86_64".to_string()),
                            "ARM64" => return Ok("arm64".to_string()),
                            _ => return Ok("i686".to_string()),
                        }
                    }
                    return Ok("i686".to_string());
                }
                _ => {}
            }
        }
        
        // Fallback: use PowerShell to detect architecture
        let output = Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "if ([System.Environment]::Is64BitOperatingSystem) { if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) { 'arm64' } else { 'x86_64' } } else { 'i686' }"
            ])
            .output()
            .context("Failed to detect architecture")?;
        
        if output.status.success() {
            let arch = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !arch.is_empty() {
                return Ok(arch);
            }
        }
        
        // Final fallback
        Ok("x86_64".to_string())
    }
    
    #[cfg(not(windows))]
    {
        Ok("x86_64".to_string())
    }
}

/// Get the latest release from GitHub
fn get_latest_release() -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    
    let response = ureq::get(&url)
        .set("User-Agent", "wole-updater")
        .call()
        .context("Failed to fetch latest release from GitHub")?;
    
    let release: GitHubRelease = response
        .into_json()
        .context("Failed to parse GitHub release response")?;
    
    Ok(release)
}

/// Compare version strings (simple semantic version comparison)
fn compare_versions(current: &str, latest: &str) -> std::cmp::Ordering {
    // Remove 'v' prefix if present
    let current = current.trim_start_matches('v');
    let latest = latest.trim_start_matches('v');
    
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    // Compare major, minor, patch
    for i in 0..3 {
        let current_val = current_parts.get(i).copied().unwrap_or(0);
        let latest_val = latest_parts.get(i).copied().unwrap_or(0);
        
        match current_val.cmp(&latest_val) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    
    std::cmp::Ordering::Equal
}

/// Download the update
fn download_update(asset_url: &str, output_path: &PathBuf) -> Result<()> {
    let mut response = ureq::get(asset_url)
        .set("User-Agent", "wole-updater")
        .call()
        .context("Failed to download update")?
        .into_reader();
    
    let mut file = fs::File::create(output_path)
        .with_context(|| format!("Failed to create file: {}", output_path.display()))?;
    
    std::io::copy(&mut response, &mut file)
        .context("Failed to write downloaded file")?;
    
    Ok(())
}

/// Install the update
fn install_update(zip_path: &PathBuf, output_mode: OutputMode) -> Result<()> {
    let install_dir = uninstall::get_install_dir()?;
    
    // Create install directory if it doesn't exist
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("Failed to create install directory: {}", install_dir.display()))?;
    
    // Extract zip file
    let extract_dir = env::temp_dir().join("wole-update");
    fs::create_dir_all(&extract_dir)
        .context("Failed to create temp extraction directory")?;
    
    // Use PowerShell to extract (works on all Windows versions)
    let ps_script = format!(
        r#"
        Expand-Archive -Path '{}' -DestinationPath '{}' -Force
        "#,
        zip_path.display().to_string().replace('\\', "\\\\"),
        extract_dir.display().to_string().replace('\\', "\\\\")
    );
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
        .output()
        .context("Failed to extract update")?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to extract update: {}", error));
    }
    
    // Find the executable in the extracted folder
    let exe_name = "wole.exe";
    let extracted_exe = extract_dir.join(exe_name);
    
    if !extracted_exe.exists() {
        // Try to find it in a subdirectory
        let mut found = false;
        for entry in fs::read_dir(&extract_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let candidate = entry.path().join(exe_name);
                if candidate.exists() {
                    fs::copy(&candidate, &install_dir.join(exe_name))
                        .context("Failed to copy executable")?;
                    found = true;
                    break;
                }
            }
        }
        
        if !found {
            return Err(anyhow::anyhow!("Executable not found in downloaded archive"));
        }
    } else {
        // Copy executable to install directory
        fs::copy(&extracted_exe, &install_dir.join(exe_name))
            .context("Failed to copy executable")?;
    }
    
    // Clean up temp files
    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(zip_path);
    
    if output_mode != OutputMode::Quiet {
        println!(
            "{} Update installed successfully to {}",
            Theme::success("OK"),
            install_dir.display()
        );
    }
    
    Ok(())
}

/// Check for updates and optionally install
pub fn check_and_update(yes: bool, check_only: bool, output_mode: OutputMode) -> Result<()> {
    if output_mode != OutputMode::Quiet {
        println!("{} Checking for updates...", Theme::primary("Checking"));
    }
    
    let latest_release = get_latest_release()?;
    let latest_version = latest_release.tag_name.trim_start_matches('v');
    let current_version = CURRENT_VERSION.trim_start_matches('v');
    
    match compare_versions(current_version, latest_version) {
        std::cmp::Ordering::Less => {
            // Update available
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Update available: {} (current: {})",
                    Theme::success("Update available"),
                    latest_version,
                    current_version
                );
            }
            
            if check_only {
                return Ok(());
            }
            
            // Detect architecture
            let arch = detect_architecture()?;
            let asset_name = format!("wole-windows-{}.zip", arch);
            
            // Find the matching asset
            let asset = latest_release
                .assets
                .iter()
                .find(|a| a.name == asset_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No release asset found for architecture: {} (looking for: {})",
                        arch,
                        asset_name
                    )
                })?;
            
            // Confirm installation
            if !yes {
                print!("Install update now? [y/N]: ");
                std::io::stdout().flush()?;
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if !input.trim().eq_ignore_ascii_case("y")
                    && !input.trim().eq_ignore_ascii_case("yes")
                {
                    if output_mode != OutputMode::Quiet {
                        println!("Update cancelled.");
                    }
                    return Ok(());
                }
            }
            
            // Download update
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Downloading update ({})...",
                    Theme::primary("Downloading"),
                    asset_name
                );
            }
            
            let temp_dir = env::temp_dir().join("wole-update");
            fs::create_dir_all(&temp_dir)?;
            let zip_path = temp_dir.join(&asset_name);
            
            download_update(&asset.browser_download_url, &zip_path)?;
            
            if output_mode != OutputMode::Quiet {
                println!("{} Installing update...", Theme::primary("Installing"));
            }
            
            install_update(&zip_path, output_mode)?;
            
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Update complete! Restart your terminal to use the new version.",
                    Theme::success("OK")
                );
            }
            
            Ok(())
        }
        std::cmp::Ordering::Equal => {
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Already up to date (version {})",
                    Theme::success("OK"),
                    current_version
                );
            }
            Ok(())
        }
        std::cmp::Ordering::Greater => {
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Running development version ({}), latest release is {}",
                    Theme::warning("Note"),
                    current_version,
                    latest_version
                );
            }
            Ok(())
        }
    }
}
