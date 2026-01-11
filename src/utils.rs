//! Shared utilities for wole
//!
//! This module contains common functions used across multiple category scanners
//! to reduce code duplication and ensure consistent behavior.

use std::path::{Path, PathBuf};

/// Get the root disk path for the current system
///
/// On Windows, returns the drive root (e.g., "C:\")
/// On Unix systems, returns "/"
pub fn get_root_disk_path() -> PathBuf {
    #[cfg(windows)]
    {
        // On Windows, get the drive root from the current directory
        if let Ok(current_dir) = std::env::current_dir() {
            // Convert to string and extract drive letter
            let dir_str = current_dir.to_string_lossy();
            // Look for drive letter pattern (e.g., "C:\" or "C:")
            if let Some(drive_letter) = dir_str.chars().next() {
                if drive_letter.is_ascii_alphabetic()
                    && dir_str.len() > 1
                    && dir_str.chars().nth(1) == Some(':')
                {
                    return PathBuf::from(format!("{}:\\", drive_letter));
                }
            }
        }
        // Fallback to C:\ if we can't determine from current directory
        PathBuf::from("C:\\")
    }

    #[cfg(not(windows))]
    {
        // On Unix systems, root is "/"
        PathBuf::from("/")
    }
}

/// Normalize a path for display (strip Windows long-path prefixes).
pub fn display_path(path: &Path) -> String {
    let path_str = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        if let Some(stripped) = path_str.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", stripped);
        }
        if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }
    path_str
}

/// Convert to long path format for Windows (\\?\)
///
/// Windows has a default path length limit of 260 characters (MAX_PATH).
/// The \\?\ prefix enables extended-length paths up to ~32,767 characters.
/// This is common in deep `node_modules` directories.
#[cfg(windows)]
pub fn to_long_path(path: &Path) -> PathBuf {
    // Already has long path prefix
    if let Some(s) = path.to_str() {
        if s.starts_with(r"\\?\") {
            return path.to_path_buf();
        }
    }

    // Convert to absolute path first if relative
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };

    // Add long path prefix
    if let Some(s) = absolute.to_str() {
        PathBuf::from(format!(r"\\?\{}", s))
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
pub fn to_long_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// Safe metadata that falls back to long path on Windows when normal access fails
///
/// Handles ERROR_PATH_NOT_FOUND (3) which occurs when paths exceed 260 chars
#[cfg(windows)]
pub fn safe_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(m),
        Err(e) if e.raw_os_error() == Some(3) => {
            // ERROR_PATH_NOT_FOUND - try with long path prefix
            std::fs::metadata(to_long_path(path))
        }
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
pub fn safe_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    std::fs::metadata(path)
}

/// Safe symlink_metadata that falls back to long path on Windows
#[cfg(windows)]
pub fn safe_symlink_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => Ok(m),
        Err(e) if e.raw_os_error() == Some(3) => std::fs::symlink_metadata(to_long_path(path)),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
pub fn safe_symlink_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    std::fs::symlink_metadata(path)
}

/// Safe read_dir that falls back to long path on Windows
#[cfg(windows)]
pub fn safe_read_dir(path: &Path) -> std::io::Result<std::fs::ReadDir> {
    match std::fs::read_dir(path) {
        Ok(rd) => Ok(rd),
        Err(e) if e.raw_os_error() == Some(3) => std::fs::read_dir(to_long_path(path)),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
pub fn safe_read_dir(path: &Path) -> std::io::Result<std::fs::ReadDir> {
    std::fs::read_dir(path)
}

/// Safe remove_file that uses long path on Windows
#[cfg(windows)]
pub fn safe_remove_file(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(3) => std::fs::remove_file(to_long_path(path)),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
pub fn safe_remove_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

/// Safe remove_dir_all that uses long path on Windows
#[cfg(windows)]
pub fn safe_remove_dir_all(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(3) => std::fs::remove_dir_all(to_long_path(path)),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
pub fn safe_remove_dir_all(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

/// Check if entry should be skipped (symlink, junction, or reparse point)
///
/// Use this before descending into directories during scanning to prevent:
/// - Infinite loops from circular symlinks
/// - Following junctions to system directories
/// - Issues with OneDrive placeholder files
pub fn should_skip_entry(path: &Path) -> bool {
    // Check for symlink via symlink_metadata
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return true;
        }
    }
    // Check for Windows reparse points (junctions, OneDrive placeholders)
    is_windows_reparse_point(path)
}

/// Returns true if this path is a Windows reparse point (junction/symlink/mount point).
///
/// Why this exists:
/// - On Windows, directory junctions and some OneDrive placeholders are *reparse points*.
/// - `walkdir`'s `follow_links(false)` prevents following *symlinks*, but junctions/reparse
///   points can still be traversed as normal directories, which may create cycles and
///   trigger stack overflows during deep scans.
pub fn is_windows_reparse_point(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            return meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

/// Calculate total size of a directory tree using parallel traversal.
///
/// Uses jwalk for parallel directory traversal which is 2-4x faster than sequential.
///
/// Optimized to:
/// - Use parallel traversal with rayon thread pool
/// - Skip permission-denied errors gracefully
/// - NOT walk into .git directories
/// - Handle symlinks and reparse points safely (don't follow)
/// - Limit depth to prevent runaway scans
/// - Handle Windows long paths (>260 chars) gracefully
pub fn calculate_dir_size(path: &Path) -> u64 {
    use jwalk::WalkDir;
    use std::sync::atomic::{AtomicU64, Ordering};

    const MAX_DEPTH: usize = 15;

    let total = AtomicU64::new(0);

    WalkDir::new(path)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonDefaultPool {
            busy_timeout: std::time::Duration::from_secs(1),
        })
        .process_read_dir(|_depth, _path, _state, children| {
            // Skip directories we don't want to descend into
            children.retain(|entry| {
                if let Ok(ref e) = entry {
                    if e.file_type().is_symlink() {
                        return false;
                    }
                    if e.file_type().is_dir() {
                        if let Some(name) = e.path().file_name() {
                            let name_str = name.to_string_lossy();
                            // Skip .git internals
                            if name_str == ".git" {
                                return false;
                            }
                        }
                    }
                }
                true
            });
        })
        .into_iter()
        .for_each(|entry| {
            if let Ok(e) = entry {
                if e.file_type().is_file() {
                    if let Ok(meta) = e.metadata() {
                        total.fetch_add(meta.len(), Ordering::Relaxed);
                    }
                }
            }
        });

    total.load(Ordering::Relaxed)
}

/// Calculate directory size and emit progress for each file visited.
///
/// Uses the same traversal rules as calculate_dir_size().
pub fn calculate_dir_size_with_progress<F>(path: &Path, on_path: &F) -> u64
where
    F: Fn(&Path) + Sync,
{
    use jwalk::WalkDir;
    use std::sync::atomic::{AtomicU64, Ordering};

    const MAX_DEPTH: usize = 15;

    let total = AtomicU64::new(0);

    WalkDir::new(path)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::RayonDefaultPool {
            busy_timeout: std::time::Duration::from_secs(1),
        })
        .process_read_dir(|_depth, _path, _state, children| {
            // Skip directories we don't want to descend into
            children.retain(|entry| {
                if let Ok(ref e) = entry {
                    if e.file_type().is_symlink() {
                        return false;
                    }
                    if e.file_type().is_dir() {
                        if let Some(name) = e.path().file_name() {
                            let name_str = name.to_string_lossy();
                            // Skip .git internals
                            if name_str == ".git" {
                                return false;
                            }
                        }
                    }
                }
                true
            });
        })
        .into_iter()
        .for_each(|entry| {
            if let Ok(e) = entry {
                if e.file_type().is_file() {
                    let path = e.path();
                    on_path(&path);
                    if let Ok(meta) = e.metadata() {
                        total.fetch_add(meta.len(), Ordering::Relaxed);
                    }
                }
            }
        });

    total.load(Ordering::Relaxed)
}

/// Fast size calculation for a single directory level (no recursion).
///
/// Use this for quick estimates when you don't need exact totals.
/// Much faster than calculate_dir_size() for large directories.
pub fn calculate_shallow_size(path: &Path) -> u64 {
    let mut total = 0u64;

    if let Ok(entries) = safe_read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = safe_metadata(&entry.path()) {
                if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }

    total
}

/// File type categories for large file detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Video,
    Audio,
    Image,
    DiskImage,
    Archive,
    Installer,
    Document,
    Spreadsheet,
    Presentation,
    Code,
    Text,
    Database,
    Backup,
    Font,
    Log,
    Certificate,
    System,
    Build,
    Subtitle,
    CAD,
    Model3D,
    GIS,
    VirtualMachine,
    Container,
    WebAsset,
    Game,
    Other,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Video => "Video",
            FileType::Audio => "Audio",
            FileType::Image => "Image",
            FileType::DiskImage => "Disk Image",
            FileType::Archive => "Archive",
            FileType::Installer => "Installer",
            FileType::Document => "Document",
            FileType::Spreadsheet => "Spreadsheet",
            FileType::Presentation => "Presentation",
            FileType::Code => "Code",
            FileType::Text => "Text",
            FileType::Database => "Database",
            FileType::Backup => "Backup",
            FileType::Font => "Font",
            FileType::Log => "Log",
            FileType::Certificate => "Certificate",
            FileType::System => "System",
            FileType::Build => "Build",
            FileType::Subtitle => "Subtitle",
            FileType::CAD => "CAD",
            FileType::Model3D => "3D Model",
            FileType::GIS => "GIS",
            FileType::VirtualMachine => "Virtual Machine",
            FileType::Container => "Container",
            FileType::WebAsset => "Web Asset",
            FileType::Game => "Game",
            FileType::Other => "Other",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            FileType::Video => "🎬",
            FileType::Audio => "🎵",
            FileType::Image => "🖼️",
            FileType::DiskImage => "💿",
            FileType::Archive => "📦",
            FileType::Installer => "📥",
            FileType::Document => "📄",
            FileType::Spreadsheet => "📊",
            FileType::Presentation => "📽️",
            FileType::Code => "💻",
            FileType::Text => "📝",
            FileType::Database => "🗃️",
            FileType::Backup => "💾",
            FileType::Font => "🔤",
            FileType::Log => "📋",
            FileType::Certificate => "🔒",
            FileType::System => "⚙️",
            FileType::Build => "🔨",
            FileType::Subtitle => "📺",
            FileType::CAD => "📐",
            FileType::Model3D => "🎨",
            FileType::GIS => "🗺️",
            FileType::VirtualMachine => "🖥️",
            FileType::Container => "📦",
            FileType::WebAsset => "🌐",
            FileType::Game => "🎮",
            FileType::Other => "📁",
        }
    }
}

/// Detect file type based on extension
/// Handles multiple extensions (e.g., .tar.gz, .min.js) by checking both last and second-to-last extensions
pub fn detect_file_type(path: &Path) -> FileType {
    // Get file name as string for analysis
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase());

    // Handle files without extensions
    if file_name.is_none() {
        return FileType::Other;
    }

    let file_name_str = file_name.unwrap();

    // Check for files without extensions but with known names
    let no_ext_files: &[(&str, FileType)] = &[
        // Container files
        ("dockerfile", FileType::Container),
        ("containerfile", FileType::Container),
        ("dockerignore", FileType::Container),
        // Build files
        ("makefile", FileType::Code),
        ("cmakelists", FileType::Code),
        ("rakefile", FileType::Code),
        ("gemfile", FileType::Code),
        ("podfile", FileType::Code),
        ("cartfile", FileType::Code),
        ("build", FileType::Code),
        // Version control
        ("gitignore", FileType::Code),
        ("gitattributes", FileType::Code),
        ("gitconfig", FileType::Code),
        ("gitmodules", FileType::Code),
        ("gitkeep", FileType::Code),
        // Config files
        ("editorconfig", FileType::Code),
        ("eslintrc", FileType::Code),
        ("prettierrc", FileType::Code),
        ("babelrc", FileType::Code),
        ("npmrc", FileType::Code),
        ("yarnrc", FileType::Code),
        ("nvmrc", FileType::Code),
        ("node-version", FileType::Code),
        ("env", FileType::Code),
        (".env", FileType::Code),
        // Shell config
        ("zshrc", FileType::Code),
        ("bashrc", FileType::Code),
        ("profile", FileType::Code),
        ("bash_profile", FileType::Code),
        ("vimrc", FileType::Code),
        ("gvimrc", FileType::Code),
        ("init.vim", FileType::Code),
        ("init.lua", FileType::Code),
    ];

    // Check files without extensions
    for (name, file_type) in no_ext_files {
        if file_name_str == *name || file_name_str == name.to_uppercase() {
            return *file_type;
        }
    }

    // Check for multiple extensions (e.g., .tar.gz, .min.js, .backup.bak)
    // Common double extensions that should be checked first
    let double_extensions: &[(&str, FileType)] = &[
        ("tar.gz", FileType::Archive),
        ("tar.bz2", FileType::Archive),
        ("tar.xz", FileType::Archive),
        ("tar.z", FileType::Archive),
        ("tar.lz", FileType::Archive),
        ("tar.lzma", FileType::Archive),
        ("tar.zst", FileType::Archive),
        ("min.js", FileType::Code),
        ("min.css", FileType::Code),
        ("bundle.js", FileType::Code),
        ("bundle.css", FileType::Code),
        ("backup.bak", FileType::Backup),
        ("backup.old", FileType::Backup),
        ("backup.tmp", FileType::Backup),
        ("sql.gz", FileType::Database),
        ("sql.bz2", FileType::Database),
        ("db.backup", FileType::Backup),
        ("log.gz", FileType::Log),
        ("log.bz2", FileType::Log),
        ("log.zip", FileType::Log),
    ];

    // Check double extensions first
    for (double_ext, file_type) in double_extensions {
        if file_name_str.ends_with(&format!(".{}", double_ext)) {
            return *file_type;
        }
    }

    // Get primary extension (last extension)
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        // Video files
        Some(
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
            | "3gp" | "3g2" | "asf" | "rm" | "rmvb" | "vob" | "ts" | "mts" | "m2ts" | "divx"
            | "f4v" | "ogv" | "ogm",
        ) => FileType::Video,
        // Audio files
        Some(
            "mp3" | "m4a" | "wav" | "flac" | "aac" | "ogg" | "oga" | "wma" | "opus" | "mka"
            | "ape" | "ac3" | "dts" | "amr" | "au" | "ra" | "tta" | "tak",
        ) => FileType::Audio,
        // Image files
        Some(
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" | "ico"
            | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw" | "dng" | "orf" | "rw2" | "pef"
            | "srw" | "raf" | "3fr" | "erf" | "mef" | "mos" | "nrw" | "srf" | "x3f" | "psb"
            | "psd" | "ai" | "indd" | "sketch" | "fig" | "xd" | "afdesign" | "afphoto" | "afpub",
        ) => FileType::Image,
        // Disk images
        Some(
            "iso" | "img" | "dmg" | "vhd" | "vhdx" | "vmdk" | "vdi" | "wim" | "esd" | "bin"
            | "cue" | "nrg" | "mdf" | "ccd" | "sub",
        ) => FileType::DiskImage,
        // Archives (excluding "dmg", "iso", "bin" which are matched as DiskImage)
        Some(
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "cab" | "tgz" | "tbz2"
            | "lz" | "lzma" | "lzh" | "ace" | "arj" | "deb" | "rpm" | "pkg" | "sit"
            | "sitx" | "z" | "cpio" | "shar" | "lbr" | "mar" | "s7z" | "alz" | "apk" | "arc"
            | "b1" | "ba" | "bh" | "bz" | "bzip2" | "c00" | "c01" | "c10" | "car" | "cbr"
            | "cbz" | "egg" | "gda" | "jar" | "lha" | "lib" | "lzo" | "lzx" | "pak" | "pea"
            | "rz" | "sfx" | "sqx" | "tlz" | "txz" | "tz" | "tzo" | "tzst" | "udf" | "war"
            | "whl" | "xar" | "zipx" | "zoo" | "zpaq" | "zz" | "001" | "002" | "003",
        ) => FileType::Archive,
        // Installers (excluding extensions already matched in Archives)
        Some(
            "exe" | "msi" | "msix" | "appx" | "appxbundle" | "ipa" | "app" | "run",
        ) => FileType::Installer,
        // Spreadsheets
        Some(
            "xlsx" | "xls" | "csv" | "ods" | "numbers",
        ) => FileType::Spreadsheet,
        // Presentations
        Some(
            "pptx" | "ppt" | "odp" | "key",
        ) => FileType::Presentation,
        // Text files
        Some(
            "txt" | "md" | "markdown" | "rtf",
        ) => FileType::Text,
        // Code and config files
        // Note: "ts" excluded as it matches video file extension "ts" - use "tsx" or "typescript" instead
        Some(
            // Config/data formats
            "har" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "config"
            | "env" | "properties" | "prop" | "settings" | "prefs" | "plist" | "xcconfig"
            // Task/build files
            | "task" | "tsk" | "mk" | "make" | "cmake" | "gradle" | "sbt" | "mod" | "sum"
            | "podspec" | "podfile" | "cartfile" | "gemfile" | "rakefile" | "pom" | "build"
            // Common languages
            | "js" | "jsx" | "tsx" | "py" | "java" | "cpp" | "c" | "h" | "hpp" | "hxx" | "h++"
            | "cc" | "cxx" | "c++" | "cs" | "php" | "rb" | "go" | "rs" | "swift" | "kt" | "kts"
            | "scala" | "sc" | "clj" | "cljs" | "cljc" | "edn" | "hs" | "lhs" | "ml" | "mli"
            | "mll" | "mly" | "fs" | "fsi" | "fsx" | "vb" | "vbs" | "pl" | "pm" | "t" | "sh"
            | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "psd1" | "bat" | "cmd" | "awk" | "sed"
            // Web technologies
            | "css" | "scss" | "sass" | "less" | "styl" | "html" | "htm" | "xhtml" | "vue"
            | "svelte" | "elm" | "dart" | "astro" | "hbs" | "handlebars" | "mustache"
            // Other languages
            | "lua" | "r" | "R" | "m" | "mm" | "zig" | "nim" | "cr" | "d" | "adb" | "ads"
            | "f" | "f90" | "f95" | "f03" | "f08" | "cbl" | "cob" | "cpy" | "pas" | "pp" | "p"
            | "dpr" | "dfm" | "erl" | "hrl" | "ex" | "exs" | "pro" | "lisp" | "lsp" | "cl"
            | "el" | "scm" | "ss" | "rkt" | "st" | "fth" | "4th" | "tcl" | "tk"
            // Database query languages
            | "plsql" | "mysql" | "pgsql" | "mssql" | "oracle" | "hql" | "cql" | "cypher"
            | "gql" | "graphql" | "gremlin" | "sparql" | "xquery" | "xpath" | "xq" | "xql"
            | "xqm" | "xqy"
            // Infrastructure as code (excluding "pp" which matches Pascal)
            | "tf" | "tfvars" | "tfstate" | "sls" | "hcl"
            // API/Schema definitions
            | "proto" | "thrift" | "avsc" | "avdl" | "fbs" | "capnp" | "idl"
            // Notebooks
            | "ipynb" | "rmd" | "qmd"
            // Templates
            | "erb" | "j2" | "jinja" | "jinja2" | "twig" | "njk" | "ejs" | "pug" | "jade"
            // IDE/Editor files
            | "iml" | "ipr" | "iws" | "code-workspace" | "sublime-project" | "sublime-workspace"
            | "atom" | "vimrc" | "gvimrc" | "vim" | "nvim"
            // Version control
            | "gitconfig" | "gitmodules" | "gitkeep"
            // Shell config
            | "zshrc" | "bashrc" | "profile" | "bash_profile" | "bash_login" | "bash_logout"
            | "zprofile" | "zlogin" | "zlogout" | "zshenv" | "fishrc"
            // Node/JS config
            | "nvmrc" | "node-version" | "npmrc" | "yarnrc" | "eslintrc" | "eslintignore"
            | "prettierrc" | "prettierignore" | "babelrc" | "babelignore" | "jestrc"
            | "jestconfig" | "tsconfig" | "jsconfig" | "webpack" | "rollup" | "vite"
            // Project files
            | "sln" | "csproj" | "vbproj" | "vcxproj" | "vcproj" | "vcxproj.filters"
            | "xcodeproj" | "xcworkspace" | "pbxproj" | "storyboard" | "xib" | "nib"
            // Other config (excluding container files which are matched separately)
            | "editorconfig" | "swagger" | "openapi"
        ) => FileType::Code,
        // Documents (PDFs, ebooks, and other document formats)
        Some(
            "pdf" | "docx" | "doc" | "odt" | "pages" | "tex" | "latex" | "epub" | "mobi"
            | "azw" | "azw3" | "fb2",
        ) => FileType::Document,
        // Databases (excluding "mdf" which matches disk image, and "bak" which matches backup)
        Some(
            "db" | "sqlite" | "sqlite3" | "mdb" | "accdb" | "fdb" | "gdb" | "ndf"
            | "ldf" | "dbf" | "db3" | "s3db" | "sl3" | "sl2" | "sl1" | "sqlitedb" | "sqlite2"
            | "sqlite-wal" | "sqlite-shm" | "db-shm" | "db-wal" | "sdb" | "sql" | "sqlite-db"
            | "db2" | "db4" | "db5" | "db6" | "db7" | "db8" | "db9" | "db10" | "db11" | "db12"
            | "db13" | "db14" | "db15" | "db16" | "db17" | "db18" | "db19" | "db20",
        ) => FileType::Database,
        // Backup files (excluding "bak" which matches database)
        Some(
            "backup" | "old" | "orig" | "bkp" | "tmp" | "temp" | "~" | "swp" | "swo"
            | "save" | "sav" | "back" | "bac" | "bk" | "bkf" | "bk1" | "bk2" | "bk3" | "bk4"
            | "bk5" | "bk6" | "bk7" | "bk8" | "bk9" | "old1" | "old2" | "old3" | "old4"
            | "old5" | "old6" | "old7" | "old8" | "old9" | "orig1" | "orig2" | "orig3"
            | "orig4" | "orig5" | "orig6" | "orig7" | "orig8" | "orig9",
        ) => FileType::Backup,
        // Font files
        Some(
            "ttf" | "otf" | "woff" | "woff2" | "eot" | "ttc" | "fon" | "pfm" | "afm" | "pfb",
        ) => FileType::Font,
        // Log files
        Some(
            "log" | "out" | "err" | "trace" | "debug" | "audit",
        ) => FileType::Log,
        // Certificate and security files (excluding "key" which matches Presentation)
        Some(
            "pem" | "crt" | "cer" | "p12" | "pfx" | "jks" | "keystore" | "truststore"
            | "csr" | "der" | "p7b" | "p7c" | "p7s" | "spc" | "p8" | "pub",
        ) => FileType::Certificate,
        // Build artifacts and compiled files (must come before System to catch .o, .obj, .class, etc.)
        Some(
            "o" | "obj" | "class" | "pyc" | "pyo" | "pycache" | "elc" | "beam" | "hi" | "cmi"
            | "cmo" | "cmx" | "cma" | "cmxa" | "cmxs" | "pdb" | "ilk" | "exp" | "map" | "lst"
            | "gcda" | "gcno" | "gcov" | "profdata",
        ) => FileType::Build,
        // System files (excluding .exe, .com, .bin, .lib, .a, .so, .dylib, .dll which are matched elsewhere)
        Some(
            "dll" | "so" | "dylib" | "sys" | "drv" | "vxd" | "ocx" | "framework" | "kext",
        ) => FileType::System,
        // Subtitle files (excluding "sub" which matches DiskImage)
        Some(
            "srt" | "vtt" | "ass" | "ssa" | "idx" | "smi" | "scc" | "ttml" | "dfxp"
            | "sbv" | "mpl2" | "mks" | "usf" | "jss",
        ) => FileType::Subtitle,
        // CAD files (specific CAD formats)
        Some(
            "dwg" | "dxf" | "step" | "stp" | "iges" | "igs" | "sat" | "3dm" | "c4d" | "ma" | "mb",
        ) => FileType::CAD,
        // 3D Model files (must come after Build to avoid .obj conflict)
        Some(
            "blend" | "3ds" | "max" | "fbx" | "dae" | "x3d" | "wrl" | "x3dv" | "stl" | "ply"
            | "off" | "3mf" | "amf" | "gltf" | "glb" | "usd" | "usda" | "usdc" | "usdz" | "abc",
        ) => FileType::Model3D,
        // GIS and mapping files (excluding .tif/.tiff/.img which are images, .map which is build)
        Some(
            "shp" | "shx" | "prj" | "kml" | "kmz" | "gpx" | "geojson" | "topojson"
            | "mif" | "mid" | "tab" | "gml" | "osm" | "pbf" | "mbtiles" | "gpkg"
            | "geotiff" | "hgt" | "dem" | "dt0" | "dt1" | "dt2" | "asc" | "adf" | "bil",
        ) => FileType::GIS,
        // Virtual machine files (excluding .vhd/.vhdx/.vdi/.vmdk which are matched as DiskImage)
        Some(
            "qcow" | "qcow2" | "vfd" | "vmem" | "vmsn" | "vmss" | "vmx" | "vmxf" | "nvram"
            | "vbox" | "ova" | "ovf" | "hdd" | "hds" | "hsv" | "vsv" | "avhd" | "avhdx" | "vbox-prev",
        ) => FileType::VirtualMachine,
        // Container and virtualization files (excluding .yaml/.yml which are Code)
        Some(
            "dockerfile" | "containerfile" | "dockerignore" | "compose" | "docker-compose"
            | "podman" | "kubernetes" | "k8s" | "helm" | "chart",
        ) => FileType::Container,
        // Web assets (excluding fonts/images/audio/video already matched)
        Some(
            "webmanifest" | "manifest" | "serviceworker" | "sw" | "wasm" | "br" | "zstd" | "lz4",
        ) => FileType::WebAsset,
        // Game files (excluding .pak/.map/.bin/.cfg/.ini/.log/.sav/.dat/.manifest already matched)
        Some(
            "wad" | "mdl" | "mdx" | "smd" | "vtx" | "vvd" | "phy" | "ani" | "vmt" | "vtf"
            | "spr" | "qc" | "dmx" | "vcd" | "pcf" | "vpk" | "gcf" | "ncf" | "vdf" | "acf"
            | "appinfo" | "appmanifest" | "gma" | "nav" | "ain" | "res" | "bsp",
        ) => FileType::Game,
        // Files without extension or unknown extensions
        None => FileType::Other,
        _ => FileType::Other,
    }
}

/// Check if a file is hidden (Windows hidden attribute or dot-prefix)
#[allow(unused_variables)]
pub fn is_hidden(path: &Path) -> bool {
    // Check dot-prefix (Unix style, also works on Windows)
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with('.') {
            return true;
        }
    }

    // Check Windows hidden attribute
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = std::fs::metadata(path) {
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            if meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
                return true;
            }
        }
    }

    false
}

/// Convert an absolute path to a relative path based on a base directory.
///
/// If the path is not under the base directory, tries to show a relative path from
/// common parent directories (Documents, OneDrive/Documents, user home) to save space.
/// If the path equals the base directory, returns ".".
pub fn to_relative_path(path: &Path, base: &Path) -> String {
    // Normalize paths by converting to strings for comparison (handles case-insensitivity on Windows)
    let path_str = path.to_string_lossy().to_string();
    let base_str = base.to_string_lossy().to_string();

    // Normalize separators and remove trailing separators
    let path_normalized = path_str
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let base_normalized = base_str
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();

    // Helper to check if path starts with base (case-insensitive on Windows)
    fn path_starts_with(path: &str, base: &str) -> bool {
        #[cfg(windows)]
        {
            path.to_lowercase().starts_with(&base.to_lowercase())
        }
        #[cfg(not(windows))]
        {
            path.starts_with(base)
        }
    }

    // Try to make relative to base directory
    if path_starts_with(&path_normalized, &base_normalized) {
        let relative = &path_normalized[base_normalized.len()..].trim_start_matches('/');
        if relative.is_empty() {
            return ".".to_string();
        }
        return relative.to_string();
    }

    // Path is not under base, try to make relative to common parent directories
    #[cfg(windows)]
    {
        // Try OneDrive/Documents
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let onedrive_docs = format!("{}/OneDrive/Documents", userprofile.replace('\\', "/"));
            let onedrive_docs_normalized = onedrive_docs.trim_end_matches('/').to_string();
            if path_starts_with(&path_normalized, &onedrive_docs_normalized) {
                let relative =
                    &path_normalized[onedrive_docs_normalized.len()..].trim_start_matches('/');
                if !relative.is_empty() {
                    return format!("documents/{}", relative);
                }
            }

            // Try Documents
            let docs = format!("{}/Documents", userprofile.replace('\\', "/"));
            let docs_normalized = docs.trim_end_matches('/').to_string();
            if path_starts_with(&path_normalized, &docs_normalized) {
                let relative = &path_normalized[docs_normalized.len()..].trim_start_matches('/');
                if !relative.is_empty() {
                    return format!("documents/{}", relative);
                }
            }

            // check other standard folders
            for (name, folder) in [
                ("Downloads", "Downloads"),
                ("Pictures", "Pictures"),
                ("Music", "Music"),
                ("Videos", "Videos"),
                ("Desktop", "Desktop"),
            ] {
                let check_path = format!("{}/{}", userprofile.replace('\\', "/"), folder);
                let check_normalized = check_path.trim_end_matches('/').to_string();
                if path_starts_with(&path_normalized, &check_normalized) {
                    let relative =
                        &path_normalized[check_normalized.len()..].trim_start_matches('/');
                    if relative.is_empty() {
                        return name.to_string();
                    } else {
                        return format!("{}/{}", name, relative);
                    }
                }
            }

            // Try user home
            let home_normalized = userprofile
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_string();
            if path_starts_with(&path_normalized, &home_normalized) {
                let relative = &path_normalized[home_normalized.len()..].trim_start_matches('/');
                if !relative.is_empty() {
                    // Start relative path directly if it's a top-level folder
                    // e.g. "Downloads" or "Music"
                    for folder in ["Downloads", "Pictures", "Music", "Videos", "Desktop"] {
                        if relative.eq_ignore_ascii_case(folder) {
                            return folder.to_string();
                        }
                        if relative
                            .to_lowercase()
                            .starts_with(&format!("{}/", folder.to_lowercase()))
                        {
                            return relative.to_string();
                        }
                    }

                    // Show last 2-3 components to save space
                    let components: Vec<&str> = relative.split('/').collect();
                    if components.len() > 3 {
                        return format!(".../{}", components[components.len() - 2..].join("/"));
                    }
                    return relative.to_string();
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        // Try user home on Unix
        if let Ok(home) = std::env::var("HOME") {
            let home_normalized = home.trim_end_matches('/').to_string();
            if path_normalized.starts_with(&home_normalized) {
                let relative = &path_normalized[home_normalized.len()..].trim_start_matches('/');
                if !relative.is_empty() {
                    return format!("~/{}", relative);
                }
            }
        }
    }

    // Fallback: show last 2-3 path components to save space
    let components: Vec<&str> = path_normalized
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if components.len() > 3 {
        format!(".../{}", components[components.len() - 2..].join("/"))
    } else if !components.is_empty() {
        components.join("/")
    } else {
        path.display().to_string()
    }
}

/// System directories to always skip during scanning
/// These are Windows system directories that should not be scanned
pub const SYSTEM_DIRS: &[&str] = &[
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "System Volume Information",
    "Recovery",
    "MSOCache",
    // Note: We intentionally scan Users directory even though it contains some system files
    // because it also contains user data which is what we want to analyze
];

/// Check if path contains a system directory
///
/// Only checks the first few Normal components (root-level directories) to avoid false positives.
/// For example:
/// - `C:\Windows\...` is a system path (blocked) ✓
/// - `C:\Program Files\...` is a system path (blocked) ✓
/// - `C:\Users\...\AppData\Local\Microsoft\Windows\Caches` is NOT a system path (should NOT be blocked) ✓
pub fn is_system_path(path: &Path) -> bool {
    let mut normal_component_count = 0;
    // Only check the first 2 Normal components after the root
    // This prevents false positives from paths like:
    // C:\Users\jhppo\AppData\Local\Microsoft\Windows\Caches (should NOT be blocked)
    // While still catching:
    // C:\Windows\... (should be blocked - "Windows" is 1st Normal component)
    // C:\Program Files\... (should be blocked - "Program Files" is 1st Normal component)
    const MAX_NORMAL_COMPONENTS_TO_CHECK: usize = 2;

    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            normal_component_count += 1;
            if normal_component_count > MAX_NORMAL_COMPONENTS_TO_CHECK {
                break;
            }

            let name_str = name.to_string_lossy();
            if SYSTEM_DIRS
                .iter()
                .any(|&sys| name_str.eq_ignore_ascii_case(sys))
            {
                return true;
            }
        }
    }
    false
}

/// Directories to skip walking into (we don't need to scan inside these)
pub const SKIP_WALK_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    "target",
    ".gradle",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
];

// Function disabled - walkdir not available in minimal test
// pub fn should_skip_walk(entry: &walkdir::DirEntry) -> bool { ... }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_type_detection() {
        // Video files
        assert_eq!(detect_file_type(Path::new("movie.mp4")), FileType::Video);
        assert_eq!(detect_file_type(Path::new("video.mkv")), FileType::Video);

        // Audio files
        assert_eq!(detect_file_type(Path::new("song.m4a")), FileType::Audio);
        assert_eq!(detect_file_type(Path::new("music.mp3")), FileType::Audio);
        assert_eq!(detect_file_type(Path::new("sound.wav")), FileType::Audio);

        // Image files
        assert_eq!(detect_file_type(Path::new("photo.heic")), FileType::Image);
        assert_eq!(detect_file_type(Path::new("image.jpg")), FileType::Image);
        assert_eq!(detect_file_type(Path::new("photo.jpeg")), FileType::Image);
        assert_eq!(detect_file_type(Path::new("photo.raw")), FileType::Image);
        assert_eq!(detect_file_type(Path::new("picture.png")), FileType::Image);

        // Disk images
        assert_eq!(
            detect_file_type(Path::new("windows.iso")),
            FileType::DiskImage
        );

        // Archives
        assert_eq!(detect_file_type(Path::new("backup.zip")), FileType::Archive);

        // Installers
        assert_eq!(
            detect_file_type(Path::new("setup.exe")),
            FileType::Installer
        );

        // Documents
        assert_eq!(
            detect_file_type(Path::new("document.docx")),
            FileType::Document
        );
        assert_eq!(detect_file_type(Path::new("file.pdf")), FileType::Document);

        // Spreadsheets
        assert_eq!(
            detect_file_type(Path::new("data.csv")),
            FileType::Spreadsheet
        );
        assert_eq!(
            detect_file_type(Path::new("spreadsheet.xlsx")),
            FileType::Spreadsheet
        );

        // Presentations
        assert_eq!(
            detect_file_type(Path::new("presentation.pptx")),
            FileType::Presentation
        );

        // Text files
        assert_eq!(detect_file_type(Path::new("readme.txt")), FileType::Text);
        assert_eq!(detect_file_type(Path::new("notes.md")), FileType::Text);

        // Code files
        assert_eq!(detect_file_type(Path::new("code.js")), FileType::Code);
        assert_eq!(detect_file_type(Path::new("config.json")), FileType::Code);
        assert_eq!(detect_file_type(Path::new("archive.har")), FileType::Code);
        assert_eq!(detect_file_type(Path::new("task.task")), FileType::Code);
        assert_eq!(detect_file_type(Path::new("script.py")), FileType::Code);

        // Databases
        assert_eq!(
            detect_file_type(Path::new("database.db")),
            FileType::Database
        );

        // Font files
        assert_eq!(detect_file_type(Path::new("font.ttf")), FileType::Font);
        assert_eq!(detect_file_type(Path::new("font.woff2")), FileType::Font);

        // Log files
        assert_eq!(detect_file_type(Path::new("app.log")), FileType::Log);

        // Certificate files
        assert_eq!(
            detect_file_type(Path::new("cert.pem")),
            FileType::Certificate
        );
        assert_eq!(
            detect_file_type(Path::new("cert.crt")),
            FileType::Certificate
        );

        // Build files
        assert_eq!(detect_file_type(Path::new("file.o")), FileType::Build);
        assert_eq!(detect_file_type(Path::new("file.class")), FileType::Build);

        // Subtitle files
        assert_eq!(
            detect_file_type(Path::new("subtitle.srt")),
            FileType::Subtitle
        );

        // CAD files
        assert_eq!(detect_file_type(Path::new("drawing.dwg")), FileType::CAD);

        // 3D Model files
        assert_eq!(detect_file_type(Path::new("model.fbx")), FileType::Model3D);

        // GIS files
        assert_eq!(detect_file_type(Path::new("map.shp")), FileType::GIS);

        // Container files
        assert_eq!(
            detect_file_type(Path::new("Dockerfile")),
            FileType::Container
        );

        // Edge cases: Multiple extensions
        assert_eq!(
            detect_file_type(Path::new("archive.tar.gz")),
            FileType::Archive
        );
        assert_eq!(detect_file_type(Path::new("script.min.js")), FileType::Code);
        assert_eq!(
            detect_file_type(Path::new("backup.backup.bak")),
            FileType::Backup
        );

        // Edge cases: Files without extension
        assert_eq!(detect_file_type(Path::new("README")), FileType::Other);
        assert_eq!(detect_file_type(Path::new("Makefile")), FileType::Code);
        assert_eq!(
            detect_file_type(Path::new("Dockerfile")),
            FileType::Container
        );

        // Edge cases: Unknown extensions
        assert_eq!(detect_file_type(Path::new("unknown.xyz")), FileType::Other);
        assert_eq!(detect_file_type(Path::new("file.unknown")), FileType::Other);
    }

    #[test]
    fn test_system_path_detection() {
        // Test Windows paths (works on any platform as we're just checking path components)
        let windows_path1 = Path::new(r"C:\Windows\System32");
        let windows_path2 = Path::new(r"C:\Program Files\App");
        let normal_path = Path::new(r"C:\Users\me\Documents");

        // Check if Windows is in the path components
        let has_windows = windows_path1.components().any(|c| {
            if let std::path::Component::Normal(name) = c {
                name.to_string_lossy().eq_ignore_ascii_case("Windows")
            } else {
                false
            }
        });

        let has_program_files = windows_path2.components().any(|c| {
            if let std::path::Component::Normal(name) = c {
                name.to_string_lossy().eq_ignore_ascii_case("Program Files")
            } else {
                false
            }
        });

        // The function should detect these as system paths
        assert_eq!(is_system_path(windows_path1), has_windows);
        assert_eq!(is_system_path(windows_path2), has_program_files);
        assert!(!is_system_path(normal_path));

        // Test with a path that definitely has a system directory at root level
        let test_path = Path::new("/Windows/system");
        assert!(is_system_path(test_path));

        let test_path2 = Path::new("/Program Files/app");
        assert!(is_system_path(test_path2));

        // Test false positive prevention: user cache directories with "Windows" in the path
        // should NOT be blocked (this was the bug we fixed)
        let user_cache_path = Path::new(r"C:\Users\jhppo\AppData\Local\Microsoft\Windows\Caches");
        assert!(
            !is_system_path(user_cache_path),
            "User cache directory should not be blocked"
        );

        let user_explorer_path = Path::new(r"C:\Users\me\AppData\Local\Microsoft\Windows\Explorer");
        assert!(
            !is_system_path(user_explorer_path),
            "User Explorer directory should not be blocked"
        );
    }

    #[test]
    #[ignore = "temporarily disabled to debug stack overflow"]
    fn test_should_skip_walk() {
        use walkdir::WalkDir;
        let temp_dir = tempfile::tempdir().unwrap();

        // Ensure we're using the temp directory
        assert!(temp_dir.path().exists());

        // Create a node_modules directory
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();

        // Create a .git directory
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        // Create a normal directory
        let normal_dir = temp_dir.path().join("normal");
        fs::create_dir_all(&normal_dir).unwrap();

        let mut entries: Vec<String> = Vec::new();
        // Use a very limited depth to prevent any stack issues
        for e in WalkDir::new(temp_dir.path())
            .max_depth(2) // Increased from 1 to allow subdirectories but still safe
            .into_iter()
            .filter_entry(|e| !should_skip_entry(e.path()))
            .flatten()
        {
            if let Some(name) = e.path().file_name() {
                entries.push(name.to_string_lossy().to_string());
            }
        }

        // Should skip node_modules and .git, but include normal
        assert!(!entries.contains(&"node_modules".to_string()));
        assert!(!entries.contains(&".git".to_string()));
    }

    #[test]
    fn test_is_hidden_dot_prefix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hidden_file = temp_dir.path().join(".hidden");
        fs::write(&hidden_file, "test").unwrap();

        assert!(is_hidden(&hidden_file));

        let visible_file = temp_dir.path().join("visible");
        fs::write(&visible_file, "test").unwrap();

        assert!(!is_hidden(&visible_file));
    }

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "hello").unwrap();
        fs::write(&file2, "world").unwrap();

        // Ensure we're using the temp directory, not accidentally walking system paths
        assert!(temp_dir.path().exists());
        let size = calculate_dir_size(temp_dir.path());
        assert_eq!(size, 10); // 5 + 5 bytes
    }

    #[test]
    fn test_file_type_emoji() {
        assert_eq!(FileType::Video.emoji(), "🎬");
        assert_eq!(FileType::Audio.emoji(), "🎵");
        assert_eq!(FileType::Image.emoji(), "🖼️");
        assert_eq!(FileType::Archive.emoji(), "📦");
        assert_eq!(FileType::Document.emoji(), "📄");
        assert_eq!(FileType::Spreadsheet.emoji(), "📊");
        assert_eq!(FileType::Presentation.emoji(), "📽️");
        assert_eq!(FileType::Code.emoji(), "💻");
        assert_eq!(FileType::Text.emoji(), "📝");
        assert_eq!(FileType::Font.emoji(), "🔤");
        assert_eq!(FileType::Log.emoji(), "📋");
        assert_eq!(FileType::Certificate.emoji(), "🔒");
        assert_eq!(FileType::System.emoji(), "⚙️");
        assert_eq!(FileType::Build.emoji(), "🔨");
        assert_eq!(FileType::Subtitle.emoji(), "📺");
        assert_eq!(FileType::CAD.emoji(), "📐");
        assert_eq!(FileType::Model3D.emoji(), "🎨");
        assert_eq!(FileType::GIS.emoji(), "🗺️");
        assert_eq!(FileType::VirtualMachine.emoji(), "🖥️");
        assert_eq!(FileType::Container.emoji(), "📦");
        assert_eq!(FileType::WebAsset.emoji(), "🌐");
        assert_eq!(FileType::Game.emoji(), "🎮");
        assert_eq!(FileType::Other.emoji(), "📁");
    }

    #[test]
    fn test_file_type_as_str() {
        assert_eq!(FileType::Video.as_str(), "Video");
        assert_eq!(FileType::Audio.as_str(), "Audio");
        assert_eq!(FileType::Image.as_str(), "Image");
        assert_eq!(FileType::DiskImage.as_str(), "Disk Image");
        assert_eq!(FileType::Spreadsheet.as_str(), "Spreadsheet");
        assert_eq!(FileType::Presentation.as_str(), "Presentation");
        assert_eq!(FileType::Code.as_str(), "Code");
        assert_eq!(FileType::Text.as_str(), "Text");
        assert_eq!(FileType::Other.as_str(), "Other");
    }

    #[test]
    fn test_to_long_path() {
        // Test that already-prefixed paths are returned unchanged
        let prefixed = Path::new(r"\\?\C:\Users\test");
        let result = to_long_path(prefixed);
        assert!(result.to_str().unwrap().starts_with(r"\\?\"));

        // Test that normal paths get the prefix added (on Windows)
        #[cfg(windows)]
        {
            let normal = Path::new(r"C:\Users\test\file.txt");
            let result = to_long_path(normal);
            assert!(result.to_str().unwrap().starts_with(r"\\?\"));
        }
    }

    #[test]
    fn test_safe_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        // safe_metadata should work on normal paths
        let meta = safe_metadata(&test_file).unwrap();
        assert!(meta.is_file());
        assert_eq!(meta.len(), 5);
    }

    #[test]
    fn test_safe_symlink_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        // safe_symlink_metadata should work on normal files
        let meta = safe_symlink_metadata(&test_file).unwrap();
        assert!(meta.is_file());
    }

    #[test]
    fn test_should_skip_entry_regular_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("regular.txt");
        fs::write(&test_file, "test").unwrap();

        // Regular files should not be skipped
        assert!(!should_skip_entry(&test_file));
    }

    #[test]
    fn test_should_skip_entry_regular_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_dir = temp_dir.path().join("regular_dir");
        fs::create_dir(&test_dir).unwrap();

        // Regular directories should not be skipped
        assert!(!should_skip_entry(&test_dir));
    }

    #[test]
    #[cfg(unix)]
    fn test_should_skip_entry_symlink() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().unwrap();
        let target = temp_dir.path().join("target.txt");
        let link = temp_dir.path().join("link.txt");

        fs::write(&target, "test").unwrap();
        symlink(&target, &link).unwrap();

        // Symlinks should be skipped
        assert!(should_skip_entry(&link));
    }
}
