@echo off
REM wole Windows Installer (Batch version)
REM Downloads and installs wole from GitHub releases

setlocal enabledelayedexpansion

set REPO=jplx05/wole

REM Detect architecture
REM Check PROCESSOR_ARCHITECTURE (may be x86 for 32-bit processes on 64-bit systems)
REM Also check PROCESSOR_ARCHITEW6432 for 64-bit architecture when running 32-bit process
if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set ARCH=x86_64
) else if "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set ARCH=arm64
) else if "%PROCESSOR_ARCHITECTURE%"=="x86" (
    REM Could be 32-bit system or 32-bit process on 64-bit system
    REM Check PROCESSOR_ARCHITEW6432 to see if running on 64-bit system
    if "%PROCESSOR_ARCHITEW6432%"=="AMD64" (
        set ARCH=x86_64
    ) else if "%PROCESSOR_ARCHITEW6432%"=="ARM64" (
        set ARCH=arm64
    ) else (
        REM True 32-bit system
        set ARCH=i686
    )
) else (
    REM Try to detect via PowerShell for better accuracy
    for /f "tokens=*" %%i in ('powershell -NoProfile -ExecutionPolicy Bypass -Command "if ([System.Environment]::Is64BitOperatingSystem) { if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) { 'arm64' } else { 'x86_64' } } else { 'i686' }"') do set ARCH=%%i
    if "!ARCH!"=="" (
        echo Warning: Could not detect architecture, defaulting to x86_64
        set ARCH=x86_64
    )
)

set ASSET=wole-windows-%ARCH%.zip
set URL=https://github.com/%REPO%/releases/latest/download/%ASSET%

echo Downloading wole for Windows-%ARCH%...

REM Create temp directory
set TEMP_DIR=%TEMP%\wole-install
if not exist "%TEMP_DIR%" mkdir "%TEMP_DIR%"

REM Download using PowerShell (works on Windows 7+)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$ProgressPreference = 'SilentlyContinue'; ^
     Invoke-WebRequest -Uri '%URL%' -OutFile '%TEMP_DIR%\wole.zip' -UseBasicParsing"

if errorlevel 1 (
    echo Failed to download wole
    exit /b 1
)

REM Extract using PowerShell
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "Expand-Archive -Path '%TEMP_DIR%\wole.zip' -DestinationPath '%TEMP_DIR%' -Force"

REM Find executable
set EXE_PATH=%TEMP_DIR%\wole.exe
if not exist "%EXE_PATH%" (
    REM Look for any .exe in extracted folder
    for /r "%TEMP_DIR%" %%f in (*.exe) do (
        set EXE_PATH=%%f
        goto :found_exe
    )
    echo Could not find wole.exe in downloaded archive
    exit /b 1
)
:found_exe

REM Determine install location
set INSTALL_DIR=%LOCALAPPDATA%\wole\bin
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"

REM Copy executable
copy /Y "%EXE_PATH%" "%INSTALL_DIR%\wole.exe" >nul

echo Installed to %INSTALL_DIR%\wole.exe

REM Add to PATH using PowerShell (with robust path handling)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$installDir = [System.IO.Path]::GetFullPath('%INSTALL_DIR%').TrimEnd('\', '/'); ^
     $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User'); ^
     $pathAlreadyAdded = $false; ^
     if (-not [string]::IsNullOrWhiteSpace($currentPath)) { ^
         $pathEntries = $currentPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }; ^
         foreach ($entry in $pathEntries) { ^
             $normalizedEntry = [System.IO.Path]::GetFullPath($entry.Trim()).TrimEnd('\', '/'); ^
             if ($normalizedEntry -eq $installDir) { ^
                 $pathAlreadyAdded = $true; ^
                 break; ^
             } ^
         } ^
     }; ^
     if (-not $pathAlreadyAdded) { ^
         if ([string]::IsNullOrWhiteSpace($currentPath)) { ^
             $newPath = $installDir; ^
         } else { ^
             $newPath = $currentPath + ';' + $installDir; ^
         }; ^
         [Environment]::SetEnvironmentVariable('Path', $newPath, 'User'); ^
         Write-Host 'Added to user PATH' -ForegroundColor Green; ^
     } else { ^
         Write-Host 'Already in PATH' -ForegroundColor Gray; ^
     }; ^
     $machinePath = [System.Environment]::GetEnvironmentVariable('Path', 'Machine'); ^
     $userPath = [System.Environment]::GetEnvironmentVariable('Path', 'User'); ^
     $newSessionPath = @(); ^
     if (-not [string]::IsNullOrWhiteSpace($machinePath)) { $newSessionPath += $machinePath }; ^
     if (-not [string]::IsNullOrWhiteSpace($userPath)) { $newSessionPath += $userPath }; ^
     if ($newSessionPath.Count -gt 0) { $env:Path = $newSessionPath -join ';' }"

REM Refresh PATH in current batch session (for immediate use within this script)
set PATH=%PATH%;%INSTALL_DIR%

echo.
echo ✓ wole installed successfully!
echo.
echo Note: If 'wole' is not recognized, restart your terminal or run:
echo   set PATH=%%PATH%%;%INSTALL_DIR%
echo.
echo Run 'wole --help' to get started.

REM Cleanup
rmdir /s /q "%TEMP_DIR%" 2>nul

endlocal
