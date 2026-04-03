// Language detection by extension
pub fn detect_language(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("Rust"), "py" => Some("Python"), "js" => Some("JavaScript"),
        "ts" => Some("TypeScript"), "go" => Some("Go"), "java" => Some("Java"),
        "rb" => Some("Ruby"), "cpp"|"cc"|"cxx" => Some("C++"), "c"|"h" => Some("C"),
        "swift" => Some("Swift"), "kt" => Some("Kotlin"), "cs" => Some("C#"),
        "php" => Some("PHP"), "html" | "htm" => Some("HTML"), "css" | "scss" | "less" => Some("CSS"),
        "sh"|"bash"|"zsh" => Some("Shell"), "sql" => Some("SQL"), "md"|"mdx" => Some("Markdown"),
        "json" => Some("JSON"), "yaml"|"yml" => Some("YAML"), "toml" => Some("TOML"),
        "xml" => Some("XML"), "jsx" => Some("JSX"), "tsx" => Some("TSX"),
        "vue" => Some("Vue"), "svelte" => Some("Svelte"), "dart" => Some("Dart"),
        "r" => Some("R"), "scala" => Some("Scala"), "zig" => Some("Zig"),
        "lua" => Some("Lua"), "ex"|"exs" => Some("Elixir"), "clj" => Some("Clojure"),
        "tf" => Some("Terraform"), "proto" => Some("Protobuf"),
        _ => None,
    }
}

// Dirs to skip when analyzing
pub const IGNORED_DIRS: &[&str] = &[
    "node_modules", ".git", "dist", "build", "out", "target",
    ".next", ".nuxt", "__pycache__", ".cache", "vendor",
    "coverage", ".idea", ".vscode", ".gradle", "eggs",
    ".tox", "bower_components", ".parcel-cache", ".svn",
    ".nyc_output", ".eggs",
];

// Extensions to skip
pub const IGNORED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "ico", "webp", "bmp",
    "mp4", "mp3", "wav", "avi", "mov",
    "woff", "woff2", "ttf", "eot", "otf",
    "zip", "tar", "gz", "rar", "7z",
    "exe", "dll", "so", "dylib", "bin",
    "pyc", "pyo", "class", "o", "obj",
    "lock",  // but we handle lock files by name too
];

// Lock/generated files to skip
pub const IGNORED_FILES: &[&str] = &[
    "package-lock.json", "yarn.lock", "pnpm-lock.yaml", "Cargo.lock",
    "Gemfile.lock", "poetry.lock", "composer.lock", "Pipfile.lock",
    ".DS_Store", "Thumbs.db",
];

// Entry point files
pub const ENTRY_POINTS: &[&str] = &[
    "main.rs", "lib.rs", "main.py", "app.py", "manage.py",
    "index.js", "index.ts", "server.js", "server.ts",
    "main.go", "main.java", "App.tsx", "App.jsx",
    "index.html", "main.dart",
];

// Config files worth noting
pub const CONFIG_FILES: &[&str] = &[
    "Dockerfile", "docker-compose.yml", "docker-compose.yaml",
    "Makefile", "CMakeLists.txt", ".env.example",
    "tsconfig.json", "webpack.config.js", "vite.config.ts",
    "jest.config.js", "pytest.ini", "setup.py", "setup.cfg",
    ".eslintrc.js", ".prettierrc", "babel.config.js",
];

// Doc files
pub const DOC_FILES: &[&str] = &[
    "README.md", "README.rst", "README.txt", "README",
    "CHANGELOG.md", "CHANGES.md", "LICENSE", "LICENSE.md",
    "CONTRIBUTING.md", "CODE_OF_CONDUCT.md",
];

pub fn should_skip_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    // Skip if any directory segment is ignored
    for part in &parts[..parts.len().saturating_sub(1)] {
        if IGNORED_DIRS.contains(part) {
            return true;
        }
    }
    // Skip by filename
    let filename = parts.last().unwrap_or(&"");
    if IGNORED_FILES.contains(filename) {
        return true;
    }
    // Skip by extension
    if let Some(ext) = std::path::Path::new(filename).extension().and_then(|e| e.to_str()) {
        if IGNORED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
    }
    false
}

// Detect manifest file type
pub fn detect_manifest(filename: &str) -> Option<&'static str> {
    match filename {
        "package.json" => Some("npm"),
        "Cargo.toml" => Some("cargo"),
        "requirements.txt" => Some("pip"),
        "pyproject.toml" => Some("pip"),
        "go.mod" => Some("go"),
        "Gemfile" => Some("ruby"),
        "pom.xml" => Some("maven"),
        "build.gradle" | "build.gradle.kts" => Some("gradle"),
        "composer.json" => Some("composer"),
        "mix.exs" => Some("hex"),
        _ => None,
    }
}

// Count lines in a byte buffer (for blobs)
pub fn count_lines(content: &[u8]) -> usize {
    content.iter().filter(|&&b| b == b'\n').count() + if content.last() != Some(&b'\n') && !content.is_empty() { 1 } else { 0 }
}
