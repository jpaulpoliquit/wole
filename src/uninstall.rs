use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::output::OutputMode;
use crate::theme::Theme;

/// Check if a path points to the currently running executable
#[cfg(windows)]
fn is_current_executable(path: &Path) -> bool {
    if let Ok(current_exe) = env::current_exe() {
        // Normalize both paths for comparison
        let current_exe_normalized = current_exe
            .canonicalize()
            .unwrap_or(current_exe)
            .to_string_lossy()
            .to_lowercase()
            .replace('/', "\\");
        let target_normalized = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
            .replace('/', "\\");
        current_exe_normalized == target_normalized
    } else {
        false
    }
}

#[cfg(not(windows))]
fn is_current_executable(_path: &Path) -> bool {
    false
}

/// Schedule a file for deletion on Windows reboot
#[cfg(windows)]
fn schedule_delete_on_reboot(path: &Path) -> Result<()> {
    use std::process::Command;

    // Use PowerShell to schedule deletion on reboot using MoveFileEx
    let path_str = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "`\"");

    let ps_script = format!(
        r#"
        $file = [System.IO.Path]::GetFullPath('{}');
        if ([System.IO.File]::Exists($file)) {{
            try {{
                # Use MoveFileEx with MOVEFILE_DELAY_UNTIL_REBOOT flag
                Add-Type -TypeDefinition @"
                    using System;
                    using System.Runtime.InteropServices;
                    public class Win32 {{
                        [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Auto)]
                        public static extern bool MoveFileEx(string lpExistingFileName, string lpNewFileName, int dwFlags);
                        public const int MOVEFILE_DELAY_UNTIL_REBOOT = 0x4;
                    }}
"@
                $result = [Win32]::MoveFileEx($file, $null, [Win32]::MOVEFILE_DELAY_UNTIL_REBOOT);
                if ($result) {{
                    Write-Host 'Scheduled for deletion on reboot' -ForegroundColor Green;
                }} else {{
                    $errorCode = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error();
                    Write-Host "Failed to schedule deletion: Error $errorCode" -ForegroundColor Red;
                    exit 1;
                }}
            }} catch {{
                Write-Host "Error: $_" -ForegroundColor Red;
                exit 1;
            }}
        }} else {{
            Write-Host 'File does not exist' -ForegroundColor Yellow;
        }}
        "#,
        path_str
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
        .context("Failed to execute PowerShell to schedule deletion")?;

    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to schedule deletion on reboot: {}",
            error
        ))
    }
}

#[cfg(not(windows))]
fn schedule_delete_on_reboot(_path: &Path) -> Result<()> {
    // Not needed on non-Windows systems
    Ok(())
}

/// Get the installation directory where wole.exe is located
pub fn get_install_dir() -> Result<PathBuf> {
    let localappdata =
        env::var("LOCALAPPDATA").context("LOCALAPPDATA environment variable not set")?;
    Ok(PathBuf::from(localappdata).join("wole").join("bin"))
}

/// Get the executable path
pub fn get_executable_path() -> Result<PathBuf> {
    Ok(get_install_dir()?.join("wole.exe"))
}

/// Get the config directory path
pub fn get_config_dir() -> Result<PathBuf> {
    let appdata = env::var("APPDATA").context("APPDATA environment variable not set")?;
    Ok(PathBuf::from(appdata).join("wole"))
}

/// Get the data directory path (contains history, etc.)
pub fn get_data_dir() -> Result<PathBuf> {
    let localappdata =
        env::var("LOCALAPPDATA").context("LOCALAPPDATA environment variable not set")?;
    Ok(PathBuf::from(localappdata).join("wole"))
}

/// Remove wole from PATH
#[allow(unused_variables)]
fn remove_from_path(output_mode: OutputMode) -> Result<()> {
    let install_dir = get_install_dir()?;

    // Normalize the path
    let install_dir_normalized = install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.clone())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_string();

    // Update user PATH in registry
    #[cfg(windows)]
    {
        use std::process::Command;

        // Use PowerShell to update the PATH in the registry
        let ps_script = format!(
            r#"
            $installDir = [System.IO.Path]::GetFullPath('{}').TrimEnd('\', '/');
            $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User');
            if (-not [string]::IsNullOrWhiteSpace($currentPath)) {{
                $pathEntries = $currentPath -split ';' | Where-Object {{ -not [string]::IsNullOrWhiteSpace($_) }};
                $filteredEntries = @();
                foreach ($entry in $pathEntries) {{
                    try {{
                        $normalizedEntry = [System.IO.Path]::GetFullPath($entry.Trim()).TrimEnd('\', '/');
                        if ($normalizedEntry -ne $installDir) {{
                            $filteredEntries += $entry;
                        }}
                    }} catch {{
                        $normalizedEntry = $entry.Trim().TrimEnd('\', '/');
                        if ($normalizedEntry -ne $installDir) {{
                            $filteredEntries += $entry;
                        }}
                    }}
                }};
                $newPath = $filteredEntries -join ';';
                [Environment]::SetEnvironmentVariable('Path', $newPath, 'User');
                Write-Host 'Removed from PATH' -ForegroundColor Green;
            }} else {{
                Write-Host 'PATH was empty' -ForegroundColor Gray;
            }}
            "#,
            install_dir_normalized.replace('\\', "\\\\")
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
            .context("Failed to execute PowerShell to update PATH")?;

        if output.status.success() {
            if output_mode != OutputMode::Quiet {
                println!("{} Removed wole from PATH", Theme::success("OK"));
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to remove from PATH: {}", error));
        }
    }

    Ok(())
}

/// Uninstall wole
pub fn uninstall(remove_config: bool, remove_data: bool, output_mode: OutputMode) -> Result<()> {
    // Check if executable exists
    let exe_path = get_executable_path()?;
    let install_dir = get_install_dir()?;

    let exe_existed = exe_path.exists();

    if !exe_existed {
        if output_mode != OutputMode::Quiet {
            eprintln!(
                "{} wole executable not found at {}",
                Theme::warning("Warning"),
                exe_path.display()
            );
            eprintln!("It may have already been removed or installed in a different location.");
            eprintln!("Continuing with PATH cleanup and optional directory removal...");
        }
    } else {
        // Create spinner for uninstall operation
        let spinner = if output_mode != OutputMode::Quiet {
            Some(crate::progress::create_spinner("Uninstalling wole..."))
        } else {
            None
        };

        if output_mode != OutputMode::Quiet {
            println!("  Executable: {}", exe_path.display());
        }

        // Track if we scheduled deletion for reboot (so we don't try to remove bin directory)
        let mut scheduled_for_reboot = false;

        // Check if we're trying to delete the currently running executable
        if is_current_executable(&exe_path) {
            // Can't delete ourselves while running - schedule for deletion on reboot
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Executable is currently running, scheduling for deletion on reboot...",
                    Theme::warning("Note")
                );
            }

            schedule_delete_on_reboot(&exe_path).with_context(|| {
                format!(
                    "Failed to schedule executable for deletion: {}",
                    exe_path.display()
                )
            })?;

            scheduled_for_reboot = true;

            // Clear spinner
            if let Some(sp) = spinner {
                crate::progress::finish_and_clear(&sp);
            }

            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Executable will be deleted on next reboot",
                    Theme::success("OK")
                );
                println!(
                    "   You can safely close this window and restart your computer to complete the uninstall."
                );
            }
        } else {
            // Not the current executable, can delete directly
            // Remove executable
            match fs::remove_file(&exe_path) {
                Ok(_) => {
                    // Clear spinner
                    if let Some(sp) = spinner {
                        crate::progress::finish_and_clear(&sp);
                    }

                    if output_mode != OutputMode::Quiet {
                        println!("{} Removed executable", Theme::success("OK"));
                    }
                }
                Err(e) => {
                    // If deletion fails with "access denied", it might be locked
                    // Try scheduling for reboot deletion as fallback
                    if e.kind() == std::io::ErrorKind::PermissionDenied
                        || e.raw_os_error() == Some(5)
                    // ERROR_ACCESS_DENIED
                    {
                        if output_mode != OutputMode::Quiet {
                            println!(
                                "{} File is locked, scheduling for deletion on reboot...",
                                Theme::warning("Note")
                            );
                        }

                        schedule_delete_on_reboot(&exe_path).with_context(|| {
                            format!(
                                "Failed to schedule executable for deletion: {}",
                                exe_path.display()
                            )
                        })?;

                        scheduled_for_reboot = true;

                        // Clear spinner
                        if let Some(sp) = spinner {
                            crate::progress::finish_and_clear(&sp);
                        }

                        if output_mode != OutputMode::Quiet {
                            println!(
                                "{} Executable will be deleted on next reboot",
                                Theme::success("OK")
                            );
                        }
                    } else {
                        // Other error - fail with original context
                        return Err(e).with_context(|| {
                            format!("Failed to remove executable: {}", exe_path.display())
                        });
                    }
                }
            }
        }

        // Remove bin directory if empty (only if we didn't schedule for reboot)
        if !scheduled_for_reboot && install_dir.exists() {
            match fs::read_dir(&install_dir) {
                Ok(mut entries) => {
                    if entries.next().is_none() {
                        // Directory is empty, remove it
                        fs::remove_dir(&install_dir).with_context(|| {
                            format!("Failed to remove directory: {}", install_dir.display())
                        })?;
                        if output_mode != OutputMode::Quiet {
                            println!("{} Removed empty bin directory", Theme::success("OK"));
                        }
                    }
                }
                Err(_) => {
                    // Can't read directory, skip
                }
            }
        } else if scheduled_for_reboot && output_mode != OutputMode::Quiet {
            // Inform user that bin directory will be removed after reboot
            println!(
                "{} Bin directory will be removed after reboot (when executable is deleted)",
                Theme::muted("Note")
            );
        }
    }

    // Remove from PATH (always attempt, even if exe doesn't exist)
    remove_from_path(output_mode)?;

    // Remove config directory if requested
    if remove_config {
        let config_dir = get_config_dir()?;
        if config_dir.exists() {
            fs::remove_dir_all(&config_dir).with_context(|| {
                format!(
                    "Failed to remove config directory: {}",
                    config_dir.display()
                )
            })?;
            if output_mode != OutputMode::Quiet {
                println!("{} Removed config directory", Theme::success("OK"));
            }
        }
    }

    // Remove data directory if requested
    if remove_data {
        let data_dir = get_data_dir()?;
        if data_dir.exists() {
            fs::remove_dir_all(&data_dir).with_context(|| {
                format!("Failed to remove data directory: {}", data_dir.display())
            })?;
            if output_mode != OutputMode::Quiet {
                println!(
                    "{} Removed data directory (including history)",
                    Theme::success("OK")
                );
            }
        }
    }

    if output_mode != OutputMode::Quiet {
        println!();
        if exe_existed {
            println!(
                "{} wole has been uninstalled successfully!",
                Theme::success("✓")
            );
        } else {
            println!("{} Cleanup completed!", Theme::success("✓"));
        }
        if !remove_config && !remove_data {
            println!("Note: Config and data directories were preserved.");
            println!("Use --config and --data flags to remove them as well.");
        }
    }

    Ok(())
}
