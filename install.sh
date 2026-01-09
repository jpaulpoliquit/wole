#!/bin/bash
set -e

REPO="jpaulpoliquit/wole"

# Detect OS and architecture
OS="windows"
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  i686|i386|x86) 
    # Check if running on 64-bit system via PowerShell
    if command -v powershell.exe >/dev/null 2>&1; then
      IS_64BIT=$(powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[System.Environment]::Is64BitOperatingSystem" 2>/dev/null | tr -d '\r\n')
      if [ "$IS_64BIT" = "True" ]; then
        # 32-bit process on 64-bit system - check actual architecture
        PROC_ARCH=$(powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture" 2>/dev/null | tr -d '\r\n')
        if [ "$PROC_ARCH" = "Arm64" ]; then
          ARCH="arm64"
        else
          ARCH="x86_64"
        fi
      else
        # True 32-bit system
        ARCH="i686"
      fi
    else
      # Fallback: assume 32-bit if we can't detect
      ARCH="i686"
    fi
    ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ASSET="wole-windows-${ARCH}.zip"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

echo "Downloading wole for Windows-${ARCH}..."

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download
echo "Downloading from $URL..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$TEMP_DIR/wole.zip"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$URL" -O "$TEMP_DIR/wole.zip"
else
  echo "Error: curl or wget is required"
  exit 1
fi

# Extract (use unzip if available, otherwise PowerShell)
if command -v unzip >/dev/null 2>&1; then
  unzip -q "$TEMP_DIR/wole.zip" -d "$TEMP_DIR"
else
  # Fall back to PowerShell on Windows
  powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \
    "Expand-Archive -Path '$TEMP_DIR/wole.zip' -DestinationPath '$TEMP_DIR' -Force"
fi

# Find executable
EXE_PATH=""
if [ -f "$TEMP_DIR/wole.exe" ]; then
  EXE_PATH="$TEMP_DIR/wole.exe"
else
  # Look for any .exe in extracted folder
  EXE_PATH=$(find "$TEMP_DIR" -name "*.exe" -type f | head -n 1)
  if [ -z "$EXE_PATH" ]; then
    echo "Error: Could not find wole.exe in downloaded archive"
    exit 1
  fi
fi

# Determine install location
INSTALL_DIR=""

# Try to use Windows paths
if [ -n "$LOCALAPPDATA" ]; then
  # Convert Windows path to Unix-style for Git Bash
  INSTALL_DIR=$(cygpath -u "$LOCALAPPDATA")/wole/bin
elif [ -n "$USERPROFILE" ]; then
  INSTALL_DIR=$(cygpath -u "$USERPROFILE")/.local/bin
else
  INSTALL_DIR="$HOME/.local/bin"
fi

# Create install directory
mkdir -p "$INSTALL_DIR"

# Copy executable
cp "$EXE_PATH" "$INSTALL_DIR/wole.exe"
chmod +x "$INSTALL_DIR/wole.exe"

echo "Installed to $INSTALL_DIR/wole.exe"

# Add to PATH using PowerShell (works better on Windows)
INSTALL_DIR_WIN=$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")

powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \
  "\$currentPath = [Environment]::GetEnvironmentVariable('Path', 'User'); \
   \$installDir = '$INSTALL_DIR_WIN'; \
   if (\$currentPath -notlike \"*\$installDir*\") { \
     \$newPath = \$currentPath + ';' + \$installDir; \
     [Environment]::SetEnvironmentVariable('Path', \$newPath, 'User'); \
     Write-Host 'Added to user PATH' \
   } else { \
     Write-Host 'Already in PATH' \
   }"

echo ""
echo "✓ wole installed successfully!"
echo ""
echo "Note: Restart your terminal or run this to use wole immediately:"
echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
echo ""
echo "Run 'wole --help' to get started."
