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
        try {
            $procArch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
            if ($procArch -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
                $ARCH = "arm64"
            } else {
                $ARCH = "x86_64"
            }
        } catch {
            # RuntimeInformation not available (older .NET), check env var
            if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
                $ARCH = "arm64"
            } else {
                $ARCH = "x86_64"
            }
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
        try {
            $procArch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
            if ($procArch -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
                $ARCH = "arm64"
            } else {
                $ARCH = "x86_64"
            }
        } catch {
            # Fallback: assume x86_64 for 64-bit OS
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
    $CURRENT_PATH = [Environment]::GetEnvironmentVariable("Path", "User")
    $INSTALL_DIR_NORMALIZED = $INSTALL_DIR.TrimEnd('\')
    
    if ($CURRENT_PATH -notlike "*$INSTALL_DIR_NORMALIZED*") {
        Write-Host "Adding to PATH..." -ForegroundColor Gray
        # Add to user PATH (no admin required)
        $NEW_PATH = "$CURRENT_PATH;$INSTALL_DIR_NORMALIZED"
        [Environment]::SetEnvironmentVariable("Path", $NEW_PATH, "User")
        Write-Host "Added $INSTALL_DIR_NORMALIZED to user PATH" -ForegroundColor Green
    } else {
        Write-Host "Already in PATH" -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host "✓ sweeper installed successfully!" -ForegroundColor Green
    Write-Host ""
    
    if ($CURRENT_PATH -notlike "*$INSTALL_DIR_NORMALIZED*") {
        Write-Host "Note: Restart your terminal or run this to use sweeper immediately:" -ForegroundColor Yellow
        Write-Host ('  $env:Path += ";' + $INSTALL_DIR_NORMALIZED + '"') -ForegroundColor White
        Write-Host ""
    }
    
    Write-Host "Run 'sweeper --help' to get started." -ForegroundColor Cyan
    
} finally {
    # Cleanup
    Remove-Item -Path $TEMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
