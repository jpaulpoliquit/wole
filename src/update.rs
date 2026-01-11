use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use crate::output::OutputMode;
use crate::theme::Theme;
use crate::uninstall;

/// Read a line from stdin, handling terminal focus loss issues on Windows.
/// This function ensures stdin is properly synchronized and clears any stale input
/// before reading, which fixes issues when the terminal loses and regains focus.
///
/// On Windows, when a terminal loses focus and regains it, stdin can be in a
/// problematic state. This function ensures we get a fresh stdin handle each time,
/// which helps resolve focus-related input issues.
fn read_line_from_stdin() -> io::Result<String> {
    // Flush stdout to ensure prompt is visible before reading
    io::stdout().flush()?;

    // Always get a fresh stdin handle to avoid issues with stale locks
    // This is especially important on Windows when the terminal loses focus
    let mut input = String::new();

    // Use BufRead for better control and proper buffering
    use std::io::BufRead;

    // Get a fresh stdin handle each time (don't reuse a locked handle)
    // This ensures we're reading from the current terminal state
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Read a line - this will block until the user types and presses Enter
    // On Windows, getting a fresh handle helps when the terminal has lost focus
    handle.read_line(&mut input)?;

    Ok(input)
}

const REPO: &str = "jplx05/wole";
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
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "if ([System.Environment]::Is64BitOperatingSystem) { if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) { 'arm64' } else { 'x86_64' } } else { 'i686' }"
            ])
            .output()
            .context("Failed to detect architecture")?;

        if output.status.success() {
            let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
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

    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();

    let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();

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

    std::io::copy(&mut response, &mut file).context("Failed to write downloaded file")?;

    Ok(())
}

/// Install the update
/// Returns Ok(true) if update was deferred, Ok(false) if installed immediately
fn install_update(zip_path: &PathBuf, output_mode: OutputMode) -> Result<bool> {
    let install_dir = uninstall::get_install_dir()?;

    // Create install directory if it doesn't exist
    fs::create_dir_all(&install_dir).with_context(|| {
        format!(
            "Failed to create install directory: {}",
            install_dir.display()
        )
    })?;

    // Extract zip file
    let extract_dir = env::temp_dir().join("wole-update");
    fs::create_dir_all(&extract_dir).context("Failed to create temp extraction directory")?;

    // Use PowerShell to extract (works on all Windows versions)
    let ps_script = format!(
        r#"
        Expand-Archive -Path '{}' -DestinationPath '{}' -Force
        "#,
        zip_path.display().to_string().replace('\\', "\\\\"),
        extract_dir.display().to_string().replace('\\', "\\\\")
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .output()
        .context("Failed to extract update")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to extract update: {}", error));
    }

    // Find the executable in the extracted folder
    let exe_name = "wole.exe";
    let extracted_exe = extract_dir.join(exe_name);
    let mut new_exe_path = None;

    if !extracted_exe.exists() {
        // Try to find it in a subdirectory
        for entry in fs::read_dir(&extract_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let candidate = entry.path().join(exe_name);
                if candidate.exists() {
                    new_exe_path = Some(candidate);
                    break;
                }
            }
        }

        if new_exe_path.is_none() {
            return Err(anyhow::anyhow!(
                "Executable not found in downloaded archive"
            ));
        }
    } else {
        new_exe_path = Some(extracted_exe);
    }

    let new_exe = new_exe_path.unwrap();
    let target_exe = install_dir.join(exe_name);

    // On Windows, we need to handle the case where the executable is currently running
    #[cfg(windows)]
    {
        // Try to copy directly first (in case the process isn't running)
        match fs::copy(&new_exe, &target_exe) {
            Ok(_) => {
                // Success! Clean up and return
                let _ = fs::remove_dir_all(&extract_dir);
                let _ = fs::remove_file(zip_path);

                if output_mode != OutputMode::Quiet {
                    println!(
                        "{} Update installed successfully to {}",
                        Theme::success("OK"),
                        install_dir.display()
                    );
                }
                return Ok(false); // Not deferred
            }
            Err(e) if e.raw_os_error() == Some(32) => {
                // File is locked (error 32), need to use deferred update approach
                // This happens when wole.exe is currently running
            }
            Err(e) => {
                return Err(e).context("Failed to copy executable")?;
            }
        }

        // Create a batch script to handle the deferred update
        let batch_script = env::temp_dir().join("wole-update-deferred.bat");
        let target_exe_str = target_exe.to_string_lossy().replace('\\', "\\\\");
        let new_exe_str = new_exe.to_string_lossy().replace('\\', "\\\\");
        let extract_dir_str = extract_dir.to_string_lossy().replace('\\', "\\\\");
        let zip_path_str = zip_path.to_string_lossy().replace('\\', "\\\\");

        // Create batch script that waits for the process to exit, then replaces the file
        // Use a more robust approach: try to copy with retries
        let batch_content = format!(
            r#"@echo off
setlocal enabledelayedexpansion
set MAX_RETRIES=30
set RETRY_COUNT=0

REM Wait for wole.exe processes to exit
:wait_loop
REM Check if any wole.exe processes are running
REM tasklist returns 0 if processes found, non-zero if not found
tasklist /FI "IMAGENAME eq wole.exe" 2>NUL | findstr /I /C:"wole.exe" >NUL 2>&1
if not errorlevel 1 (
    REM Process still running, wait and retry
    set /A RETRY_COUNT+=1
    if !RETRY_COUNT! GEQ %MAX_RETRIES% (
        echo Timeout waiting for wole.exe to exit
        exit /B 1
    )
    timeout /t 1 /nobreak >NUL
    goto wait_loop
)

REM No processes found, try to copy the new executable (with retry in case of transient locks)
set RETRY_COUNT=0
:copy_loop
copy /Y "{}" "{}" >NUL 2>&1
if not errorlevel 1 (
    REM Success! Clean up temp files
    rmdir /S /Q "{}" >NUL 2>&1
    del /F /Q "{}" >NUL 2>&1
    REM Delete this batch script itself
    (goto) 2>NUL & del "%~f0"
    exit /B 0
) else (
    REM Copy failed, retry
    set /A RETRY_COUNT+=1
    if !RETRY_COUNT! GEQ 5 (
        echo Failed to copy executable after retries
        exit /B 1
    )
    timeout /t 1 /nobreak >NUL
    goto copy_loop
)
"#,
            new_exe_str, target_exe_str, extract_dir_str, zip_path_str
        );

        fs::write(&batch_script, batch_content).context("Failed to create update batch script")?;

        // Execute the batch script in a detached process
        // Use PowerShell to start it in the background without a window
        // Properly escape the batch script path for PowerShell
        let batch_path_escaped = batch_script
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "`\"");

        let ps_start_script = format!(
            r#"
            $psi = New-Object System.Diagnostics.ProcessStartInfo;
            $psi.FileName = 'cmd.exe';
            $psi.Arguments = '/C "{}"';
            $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden;
            $psi.CreateNoWindow = $true;
            $psi.UseShellExecute = $false;
            [System.Diagnostics.Process]::Start($psi) | Out-Null
            "#,
            batch_path_escaped
        );

        Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &ps_start_script,
            ])
            .output()
            .context("Failed to start deferred update script")?;

        if output_mode != OutputMode::Quiet {
            println!(
                "{} Update will be installed automatically after wole exits.",
                Theme::success("OK")
            );
            println!("   The new version will be available the next time you run wole.");
        }

        Ok(true) // Update was deferred
    }

    #[cfg(not(windows))]
    {
        // On non-Windows systems, direct copy should work
        fs::copy(&new_exe, &target_exe).context("Failed to copy executable")?;

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

        Ok(false) // Not deferred
    }
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
                io::stdout().flush()?;

                let input = match read_line_from_stdin() {
                    Ok(line) => line.trim().to_lowercase(),
                    Err(_) => {
                        // If reading fails (e.g., stdin is not available), default to "no"
                        if output_mode != OutputMode::Quiet {
                            println!("\nUpdate cancelled (failed to read input).");
                        }
                        return Ok(());
                    }
                };

                if input != "y" && input != "yes" {
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

            let update_deferred = install_update(&zip_path, output_mode)?;

            if !update_deferred && output_mode != OutputMode::Quiet {
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
