# sweeper Windows Installer
# Downloads and installs sweeper from GitHub releases

$ErrorActionPreference = "Stop"

$REPO = "jpaulpoliquit/sweeper"

# Detect architecture
# Check PROCESSOR_ARCHITECTURE environment variable first
$ARCH = $env:PROCESSOR_ARCHITECTURE

# Also check PROCESSOR_ARCHITEW6432 for 32-bit processes on 64-bit systems
if ([string]::IsNullOrEmpty($ARCH) -or $ARCH -eq "x86") {
    $ARCH = $env:PROCESSOR_ARCHITEW6432
}

# Determine architecture
if ([string]::IsNullOrEmpty($ARCH)) {
    # Fallback: check if system is 64-bit
    if ([System.Environment]::Is64BitOperatingSystem) {
        # Check if ARM64 (try RuntimeInformation first, fallback to env var)
        $isArm64 = $false
        try {
            # Try to use RuntimeInformation if available (.NET Core/.NET 5+ or .NET Framework 4.7.1+)
            $procArch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
            if ($procArch -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
                $isArm64 = $true
            }
        } catch {
            # RuntimeInformation not available (older .NET), check env var
            if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
                $isArm64 = $true
            }
        }
        
        if ($isArm64) {
            $ARCH = "arm64"
        } else {
            $ARCH = "x86_64"
        }
    } else {
        # 32-bit system
        $ARCH = "i686"
    }
} elseif ($ARCH -eq "AMD64" -or $ARCH -eq "x64") {
    $ARCH = "x86_64"
} elseif ($ARCH -eq "ARM64" -or $ARCH -eq "arm64") {
    $ARCH = "arm64"
} elseif ($ARCH -eq "x86" -or $ARCH -eq "X86") {
    # Could be 32-bit system or 32-bit process on 64-bit system
    # Check if running on 64-bit OS
    if ([System.Environment]::Is64BitOperatingSystem) {
        # 32-bit process on 64-bit system - check actual architecture
        $isArm64 = $false
        try {
            # Try to use RuntimeInformation if available
            $procArch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
            if ($procArch -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
                $isArm64 = $true
            }
        } catch {
            # RuntimeInformation not available, check env var
            if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
                $isArm64 = $true
            }
        }
        
        if ($isArm64) {
            $ARCH = "arm64"
        } else {
            $ARCH = "x86_64"
        }
    } else {
        # True 32-bit system
        $ARCH = "i686"
    }
} else {
    Write-Warning "Unknown architecture '$ARCH', defaulting to x86_64"
    $ARCH = "x86_64"
}

$ASSET = "sweeper-windows-${ARCH}.zip"
$URL = "https://github.com/${REPO}/releases/latest/download/${ASSET}"

Write-Host "Downloading sweeper for Windows-${ARCH}..." -ForegroundColor Cyan

# Create temp directory
$TEMP_DIR = Join-Path $env:TEMP "sweeper-install"
New-Item -ItemType Directory -Force -Path $TEMP_DIR | Out-Null

try {
    # Download the release
    $ZIP_PATH = Join-Path $TEMP_DIR "sweeper.zip"
    Write-Host "Downloading from $URL..." -ForegroundColor Gray
    Invoke-WebRequest -Uri $URL -OutFile $ZIP_PATH -UseBasicParsing
    
    # Extract
    Write-Host "Extracting..." -ForegroundColor Gray
    Expand-Archive -Path $ZIP_PATH -DestinationPath $TEMP_DIR -Force
    
    # Find the executable (could be sweeper.exe or sweeper-windows-x86_64.exe)
    $EXE_NAME = "sweeper.exe"
    $EXE_PATH = Join-Path $TEMP_DIR $EXE_NAME
    
    # If not found, look for any .exe in the extracted folder
    if (-not (Test-Path $EXE_PATH)) {
        $EXE_PATH = Get-ChildItem -Path $TEMP_DIR -Filter "*.exe" -Recurse | Select-Object -First 1 -ExpandProperty FullName
        if (-not $EXE_PATH) {
            Write-Error "Could not find sweeper.exe in downloaded archive"
            exit 1
        }
    }
    
    # Determine install location
    # Use user directory by default (no admin required)
    $INSTALL_DIR = Join-Path $env:LOCALAPPDATA "sweeper\bin"
    
    # Create install directory
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
    
    # Copy executable
    $TARGET_PATH = Join-Path $INSTALL_DIR $EXE_NAME
    Copy-Item -Path $EXE_PATH -Destination $TARGET_PATH -Force
    
    Write-Host "Installed to $TARGET_PATH" -ForegroundColor Green
    
    # Add to PATH
    $INSTALL_DIR_NORMALIZED = [System.IO.Path]::GetFullPath($INSTALL_DIR).TrimEnd('\', '/')
    $CURRENT_PATH = [Environment]::GetEnvironmentVariable("Path", "User")
    
    # Check if already in PATH (case-insensitive, handle trailing slashes)
    $pathAlreadyAdded = $false
    if (-not [string]::IsNullOrWhiteSpace($CURRENT_PATH)) {
        $pathEntries = $CURRENT_PATH -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
        foreach ($entry in $pathEntries) {
            $normalizedEntry = [System.IO.Path]::GetFullPath($entry.Trim()).TrimEnd('\', '/')
            if ($normalizedEntry -eq $INSTALL_DIR_NORMALIZED) {
                $pathAlreadyAdded = $true
                break
            }
        }
    }
    
    if (-not $pathAlreadyAdded) {
        Write-Host "Adding to PATH..." -ForegroundColor Gray
        # Add to user PATH (no admin required)
        if ([string]::IsNullOrWhiteSpace($CURRENT_PATH)) {
            $NEW_PATH = $INSTALL_DIR_NORMALIZED
        } else {
            $NEW_PATH = "$CURRENT_PATH;$INSTALL_DIR_NORMALIZED"
        }
        [Environment]::SetEnvironmentVariable("Path", $NEW_PATH, "User")
        Write-Host "Added $INSTALL_DIR_NORMALIZED to user PATH" -ForegroundColor Green
    } else {
        Write-Host "Already in PATH" -ForegroundColor Gray
    }
    
    # Refresh PATH in current session (handle null/empty values)
    $machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    
    $newSessionPath = @()
    if (-not [string]::IsNullOrWhiteSpace($machinePath)) {
        $newSessionPath += $machinePath
    }
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $newSessionPath += $userPath
    }
    
    if ($newSessionPath.Count -gt 0) {
        $env:Path = $newSessionPath -join ';'
    } elseif (-not [string]::IsNullOrWhiteSpace($env:Path)) {
        # Keep existing PATH if registry is empty (shouldn't happen, but be safe)
        Write-Warning "Could not refresh PATH from registry, keeping current session PATH"
    }
    
    Write-Host ""
    Write-Host "✓ sweeper installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Run 'sweeper --help' to get started." -ForegroundColor Cyan
    
} finally {
    # Cleanup
    Remove-Item -Path $TEMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
