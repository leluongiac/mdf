use inquire::{Confirm, InquireError, MultiSelect, Select, Text};
use std::env;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ============================================================================
// Data Model & Serialization
// ============================================================================

#[derive(Debug, Clone)]
struct DesktopAction {
    id: String,
    name: String,
    exec: String,
    icon: Option<String>,
}

#[derive(Debug, Default)]
struct DesktopEntry {
    // Core fields
    entry_type: String,
    name: String,
    comment: Option<String>,
    icon: Option<String>,

    // Application-specific fields
    exec: Option<String>,
    terminal: bool,
    categories: Vec<String>,

    // Link-specific fields
    url: Option<String>,

    // Advanced fields (ordered by practical frequency)
    startup_wm_class: Option<String>,
    mime_type: Option<String>,
    keywords: Option<String>,
    generic_name: Option<String>,
    path: Option<String>,
    try_exec: Option<String>,
    no_display: Option<bool>,
    startup_notify: Option<bool>,
    actions: Vec<DesktopAction>,
    only_show_in: Option<String>,
    not_show_in: Option<String>,
    prefers_non_default_gpu: Option<bool>,
    single_main_window: Option<bool>,
    hidden: Option<bool>,
    dbus_activatable: Option<bool>,
    implements: Option<String>,
    version: Option<String>,
}

impl DesktopEntry {
    /// Serializes the desktop entry into a Freedesktop-compliant string.
    fn to_desktop_string(&self) -> String {
        let mut lines = vec!["[Desktop Entry]".to_string()];

        // Common keys valid for all entry types (Application, Link, Directory)
        if let Some(v) = &self.version {
            lines.push(format!("Version={}", escape_desktop_string(v)));
        }
        lines.push(format!("Type={}", self.entry_type));
        lines.push(format!("Name={}", escape_desktop_string(&self.name)));

        if let Some(v) = &self.generic_name {
            lines.push(format!("GenericName={}", escape_desktop_string(v)));
        }
        if let Some(v) = &self.comment {
            lines.push(format!("Comment={}", escape_desktop_string(v)));
        }
        if let Some(v) = &self.icon {
            lines.push(format!("Icon={}", escape_desktop_string(v)));
        }
        if let Some(v) = self.no_display {
            lines.push(format!("NoDisplay={}", v));
        }
        if let Some(v) = self.hidden {
            lines.push(format!("Hidden={}", v));
        }
        if let Some(v) = &self.only_show_in {
            lines.push(format!("OnlyShowIn={}", v));
        } else if let Some(v) = &self.not_show_in {
            lines.push(format!("NotShowIn={}", v));
        }

        // Type-specific keys
        if self.entry_type == "Application" {
            if let Some(v) = &self.exec {
                lines.push(format!("Exec={}", v));
            }
            if let Some(v) = &self.try_exec {
                lines.push(format!("TryExec={}", escape_desktop_string(v)));
            }
            if let Some(v) = &self.path {
                lines.push(format!("Path={}", escape_desktop_string(v)));
            }
            lines.push(format!("Terminal={}", self.terminal));
            if !self.categories.is_empty() {
                lines.push(format!("Categories={};", self.categories.join(";")));
            }
            if let Some(v) = &self.mime_type {
                lines.push(format!("MimeType={}", v));
            }
            if let Some(v) = &self.keywords {
                lines.push(format!("Keywords={}", v));
            }
            if let Some(v) = &self.startup_wm_class {
                lines.push(format!("StartupWMClass={}", escape_desktop_string(v)));
            }
            if let Some(v) = self.startup_notify {
                lines.push(format!("StartupNotify={}", v));
            }
            if !self.actions.is_empty() {
                let ids: Vec<&str> = self.actions.iter().map(|a| a.id.as_str()).collect();
                lines.push(format!("Actions={};", ids.join(";")));
            }
            if let Some(v) = self.prefers_non_default_gpu {
                lines.push(format!("PrefersNonDefaultGPU={}", v));
            }
            if let Some(v) = self.single_main_window {
                lines.push(format!("SingleMainWindow={}", v));
            }
            if let Some(v) = self.dbus_activatable {
                lines.push(format!("DBusActivatable={}", v));
            }
            if let Some(v) = &self.implements {
                lines.push(format!("Implements={}", v));
            }

            for action in &self.actions {
                lines.push(String::new());
                lines.push(format!("[Desktop Action {}]", action.id));
                lines.push(format!("Name={}", escape_desktop_string(&action.name)));
                lines.push(format!("Exec={}", action.exec));
                if let Some(icon) = &action.icon {
                    lines.push(format!("Icon={}", escape_desktop_string(icon)));
                }
            }
        } else if self.entry_type == "Link"
            && let Some(v) = &self.url
        {
            lines.push(format!("URL={}", escape_desktop_string(v)));
        }

        format!("{}\n", lines.join("\n"))
    }
}

// ============================================================================
// String Escaping, Formatting & Path Utilities
// ============================================================================

/// Escapes special characters for Freedesktop `string` and `localestring` values.
fn escape_desktop_string(val: &str) -> String {
    let mut out = String::with_capacity(val.len());
    for c in val.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

fn is_exec_reserved(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '"'
            | '\''
            | '\\'
            | '>'
            | '<'
            | '~'
            | '|'
            | '&'
            | ';'
            | '$'
            | '*'
            | '?'
            | '#'
            | '('
            | ')'
            | '`'
    )
}

fn escape_exec_quoted(val: &str) -> String {
    let mut out = String::with_capacity(val.len());
    for c in val.chars() {
        match c {
            '\\' => out.push_str("\\\\\\\\"),
            '"' => out.push_str("\\\\\""),
            '$' => out.push_str("\\\\$"),
            '`' => out.push_str("\\\\`"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

/// Smartly formats the `Exec` value:
/// - If the input points to an existing file on disk containing spaces or reserved chars,
///   it wraps and escapes the path in double quotes.
/// - Otherwise (e.g., command with flags), it preserves the user's command intact.
fn format_exec(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed);

    if Path::new(unquoted).exists() {
        if unquoted.chars().any(is_exec_reserved) {
            return format!("\"{}\"", escape_exec_quoted(unquoted));
        }
        return unquoted.to_string();
    }

    trimmed.to_string()
}

/// Checks whether the executable name/path itself contains an equals sign (`=`).
fn has_equals_in_executable(raw: &str) -> bool {
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed);

    if Path::new(unquoted).exists() {
        return unquoted.contains('=');
    }

    if let Some(first_token) = trimmed.split_whitespace().next()
        && first_token != "env"
    {
        return first_token.contains('=');
    }
    false
}

/// Normalizes semicolon-separated `string(s)` values while preserving escaped `\;`.
fn normalize_list(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let mut items = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for c in raw.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            current.push(c);
            escaped = true;
        } else if c == ';' {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                items.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        items.push(trimmed.to_string());
    }

    if items.is_empty() {
        None
    } else {
        Some(format!("{};", items.join(";")))
    }
}

fn slugify_ascii(text: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }

    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug.to_string()
    }
}

/// Generates a clean filename with the spec-compliant extension (`.desktop` or `.directory`).
fn slugify_filename(name: &str, entry_type: &str) -> String {
    let base = slugify_ascii(name, "entry");
    let ext = if entry_type == "Directory" {
        "directory"
    } else {
        "desktop"
    };
    format!("{}.{}", base, ext)
}

/// Generates a valid Freedesktop Action ID (`[A-Za-z0-9-]`) from an action display name.
fn slugify_action_id(name: &str) -> String {
    slugify_ascii(name, "action")
}

/// Validates that an Action ID is non-empty and contains only `[A-Za-z0-9-]`.
fn is_valid_action_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Ensures the user-provided filename ends with the correct extension for its `Type`.
fn normalize_filename(raw: &str, default_name: &str, entry_type: &str) -> String {
    let ext = if entry_type == "Directory" {
        ".directory"
    } else {
        ".desktop"
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return default_name.to_string();
    }
    if trimmed.ends_with(ext) {
        return trimmed.to_string();
    }
    let stem = trimmed
        .strip_suffix(".desktop")
        .or_else(|| trimmed.strip_suffix(".directory"))
        .unwrap_or(trimmed);
    format!("{}{}", stem, ext)
}

/// Checks if a filename follows reverse-DNS naming (e.g., `org.example.App.desktop`).
fn is_reverse_dns_filename(filename: &str) -> bool {
    let stem = filename
        .strip_suffix(".desktop")
        .or_else(|| filename.strip_suffix(".directory"))
        .unwrap_or(filename);
    stem.split('.').filter(|s| !s.is_empty()).count() >= 3
}

// ============================================================================
// Interactive Prompt Helpers (Clean Exit on Esc / Ctrl+C)
// ============================================================================

fn handle_prompt<T>(result: Result<T, InquireError>) -> T {
    match result {
        Ok(val) => val,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            println!("\nOperation canceled. Goodbye!");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Input error: {}", e);
            std::process::exit(1);
        }
    }
}

fn prompt_required(label: &str) -> String {
    loop {
        let input = handle_prompt(Text::new(label).prompt());
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        println!("This field is required and cannot be empty.");
    }
}

fn prompt_optional(label: &str) -> Option<String> {
    let input = handle_prompt(
        Text::new(label)
            .with_help_message("Press Enter to skip")
            .prompt(),
    );
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn prompt_list(label: &str, help: &str) -> Option<String> {
    let input = handle_prompt(Text::new(label).with_help_message(help).prompt());
    normalize_list(&input)
}

fn prompt_bool(label: &str, default: bool) -> bool {
    handle_prompt(Confirm::new(label).with_default(default).prompt())
}

fn prompt_exec(label: &str) -> String {
    let raw_exec = prompt_required(label);
    if has_equals_in_executable(&raw_exec) {
        println!(
            "Warning: The executable name or path contains '='. \
            Per the Freedesktop specification, executable names must not contain '='."
        );
    }
    format_exec(&raw_exec)
}

fn prompt_actions() -> Vec<DesktopAction> {
    let mut actions: Vec<DesktopAction> = Vec::new();
    println!("\n--- Configuring Desktop Actions ---");

    loop {
        let action_name = prompt_required("Action display name (Name):");

        let base_id = slugify_action_id(&action_name);
        let mut suggested_id = base_id.clone();
        let mut suffix = 2;
        while actions.iter().any(|a| a.id == suggested_id) {
            suggested_id = format!("{}-{}", base_id, suffix);
            suffix += 1;
        }

        let action_id = loop {
            let input = handle_prompt(
                Text::new("Action identifier (ID):")
                    .with_default(&suggested_id)
                    .with_help_message("Allowed characters: ASCII letters, digits, and hyphens (-)")
                    .prompt(),
            );
            let candidate = input.trim();
            if !is_valid_action_id(candidate) {
                println!(
                    "Invalid Action ID. Only ASCII letters, digits, and hyphens (-) are allowed."
                );
                continue;
            }
            if actions.iter().any(|a| a.id == candidate) {
                println!(
                    "Action ID '{}' is already used. Please choose a unique identifier.",
                    candidate
                );
                continue;
            }
            break candidate.to_string();
        };

        let action_exec = prompt_exec("Action executable command or path (Exec):");
        let action_icon = prompt_optional("Action icon name or path (Icon):");

        actions.push(DesktopAction {
            id: action_id,
            name: action_name,
            exec: action_exec,
            icon: action_icon,
        });

        if !prompt_bool("Add another desktop action?", false) {
            break;
        }
    }

    actions
}

// ============================================================================
// System Integration Helpers (XDG Paths, Root Check, Saving, Validation)
// ============================================================================

fn resolve_user_data_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
        let path = PathBuf::from(xdg);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    let home = env::var("HOME").expect("HOME environment variable is not set");
    PathBuf::from(home).join(".local/share")
}

fn target_subdir(entry_type: &str) -> &'static str {
    if entry_type == "Directory" {
        "desktop-directories"
    } else {
        "applications"
    }
}

fn is_running_as_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

fn write_file_with_sudo(full_path: &Path, target_dir: &Path, content: &str) -> bool {
    if !target_dir.exists() {
        let mkdir_status = Command::new("sudo")
            .arg("mkdir")
            .arg("-p")
            .arg(target_dir)
            .status();
        if !matches!(mkdir_status, Ok(s) if s.success()) {
            eprintln!("Failed to create system directory {:?}.", target_dir);
            return false;
        }
    }

    let mut child = match Command::new("sudo")
        .arg("tee")
        .arg(full_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to run 'sudo tee': {}", e);
            return false;
        }
    };

    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(content.as_bytes())
    {
        eprintln!("Failed to write content to 'sudo tee': {}", e);
        return false;
    }

    matches!(child.wait(), Ok(s) if s.success())
}

// ============================================================================
// Main Application Flow
// ============================================================================

fn main() {
    println!("=== mdf - Desktop Entry Generator ===\n");

    // 1. Entry type selection (Application is default on Enter)
    let types = vec!["Application", "Link", "Directory"];
    let entry_type = handle_prompt(
        Select::new("Entry type (Type):", types)
            .with_help_message("Press Enter to select Application")
            .prompt(),
    )
    .to_string();

    // 2. Basic metadata
    let name = prompt_required("Display name (Name):");
    let comment = prompt_optional("Short description (Comment):");
    let icon = prompt_optional("Icon name or path (Icon):");

    let mut entry = DesktopEntry {
        entry_type: entry_type.clone(),
        name,
        comment,
        icon,
        ..Default::default()
    };

    // 3. Type-specific basic fields
    match entry_type.as_str() {
        "Application" => {
            entry.exec = Some(prompt_exec("Executable command or path (Exec):"));
            entry.terminal = prompt_bool("Run in a terminal window (Terminal)?", false);

            let cat_options = vec![
                "Utility",
                "Development",
                "Network",
                "Office",
                "Graphics",
                "AudioVideo",
                "Game",
                "System",
                "Settings",
            ];
            entry.categories = handle_prompt(
                MultiSelect::new("Categories:", cat_options)
                    .with_help_message("Space to toggle, Enter to confirm")
                    .prompt(),
            )
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        }
        "Link" => {
            entry.url = Some(prompt_required("Target URL (URL):"));
        }
        _ => {}
    }

    // 4. Optional advanced configuration
    if prompt_bool("\nWould you like to configure advanced options?", false) {
        let adv_options = if entry.entry_type == "Application" {
            vec![
                "StartupWMClass",
                "MimeType",
                "Keywords",
                "GenericName",
                "Path",
                "TryExec",
                "NoDisplay",
                "StartupNotify",
                "Actions",
                "OnlyShowIn / NotShowIn",
                "PrefersNonDefaultGPU",
                "SingleMainWindow",
                "Hidden",
                "DBusActivatable",
                "Implements",
                "Version",
            ]
        } else {
            vec![
                "GenericName",
                "NoDisplay",
                "OnlyShowIn / NotShowIn",
                "Hidden",
                "Version",
            ]
        };

        let selected = handle_prompt(
            MultiSelect::new("Select advanced options to configure:", adv_options)
                .with_help_message("Space to toggle, Enter to confirm")
                .prompt(),
        );

        for key in selected {
            match key {
                "StartupWMClass" => {
                    entry.startup_wm_class =
                        prompt_optional("StartupWMClass (e.g., code, obsidian):");
                }
                "MimeType" => {
                    entry.mime_type = prompt_list(
                        "MimeType:",
                        "Separate values with semicolons (e.g., text/plain;application/pdf)",
                    );
                }
                "Keywords" => {
                    entry.keywords = prompt_list(
                        "Keywords:",
                        "Separate values with semicolons (e.g., editor;code;ide)",
                    );
                }
                "GenericName" => {
                    entry.generic_name =
                        prompt_optional("GenericName (e.g., Web Browser, Text Editor):");
                }
                "Path" => {
                    entry.path = prompt_optional("Working directory (Path):");
                }
                "TryExec" => {
                    entry.try_exec =
                        prompt_optional("TryExec (executable path to verify installation):");
                }
                "NoDisplay" => {
                    entry.no_display =
                        Some(prompt_bool("NoDisplay (hide from application menus)?", true));
                }
                "StartupNotify" => {
                    entry.startup_notify = Some(prompt_bool(
                        "StartupNotify (enable startup notification)?",
                        true,
                    ));
                }
                "Actions" => {
                    entry.actions = prompt_actions();
                }
                "OnlyShowIn / NotShowIn" => {
                    let modes = vec![
                        "OnlyShowIn (show only in specified desktop environments)",
                        "NotShowIn (hide in specified desktop environments)",
                    ];
                    let mode = handle_prompt(
                        Select::new("Desktop environment visibility rule:", modes).prompt(),
                    );
                    if mode.starts_with("OnlyShowIn") {
                        entry.only_show_in = prompt_list(
                            "OnlyShowIn:",
                            "Separate values with semicolons (e.g., GNOME;KDE;XFCE)",
                        );
                        entry.not_show_in = None;
                    } else {
                        entry.not_show_in = prompt_list(
                            "NotShowIn:",
                            "Separate values with semicolons (e.g., GNOME;KDE;XFCE)",
                        );
                        entry.only_show_in = None;
                    }
                }
                "PrefersNonDefaultGPU" => {
                    entry.prefers_non_default_gpu = Some(prompt_bool(
                        "PrefersNonDefaultGPU (prefer discrete GPU if available)?",
                        true,
                    ));
                }
                "SingleMainWindow" => {
                    entry.single_main_window = Some(prompt_bool(
                        "SingleMainWindow (application uses a single main window)?",
                        true,
                    ));
                }
                "Hidden" => {
                    entry.hidden = Some(prompt_bool(
                        "Hidden (mark entry as deleted/hidden for current user)?",
                        true,
                    ));
                }
                "DBusActivatable" => {
                    entry.dbus_activatable =
                        Some(prompt_bool("DBusActivatable (launch via D-Bus)?", true));
                }
                "Implements" => {
                    entry.implements = prompt_list(
                        "Implements:",
                        "Separate interface names with semicolons (;)",
                    );
                }
                "Version" => {
                    let ver = handle_prompt(
                        Text::new("Desktop Entry specification version:")
                            .with_default("1.5")
                            .prompt(),
                    );
                    let trimmed = ver.trim();
                    if !trimmed.is_empty() {
                        entry.version = Some(trimmed.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // 5. Preview generated content
    let content = entry.to_desktop_string();
    println!("\n--- Preview ---\n{}---------------", content);

    if !prompt_bool("Save this file?", true) {
        println!("Save canceled.");
        return;
    }

    // 6. Filename prompt (with reverse-DNS check if DBusActivatable=true)
    let default_filename = slugify_filename(&entry.name, &entry.entry_type);
    let filename = loop {
        let raw_filename = handle_prompt(
            Text::new("File name:")
                .with_default(&default_filename)
                .with_help_message("Press Enter to keep the suggested name, or type a custom one")
                .prompt(),
        );
        let candidate = normalize_filename(&raw_filename, &default_filename, &entry.entry_type);

        if entry.dbus_activatable == Some(true) && !is_reverse_dns_filename(&candidate) {
            println!(
                "Warning: DBusActivatable=true requires a reverse-DNS filename \
                (e.g., org.example.App.desktop) for D-Bus activation to work."
            );
            if !prompt_bool("Keep this filename anyway?", false) {
                continue;
            }
        }
        break candidate;
    };

    // 7. Target directory selection
    let subdir = target_subdir(&entry.entry_type);
    let user_dir = resolve_user_data_dir().join(subdir);
    let system_dir = PathBuf::from("/usr/share").join(subdir);

    let locations = vec![
        format!("Current user ({})", user_dir.display()),
        format!("System-wide ({} - requires privileges)", system_dir.display()),
    ];
    let choice = handle_prompt(Select::new("Save location:", locations).prompt());

    let is_system = choice.starts_with("System-wide");
    let target_dir = if is_system { system_dir } else { user_dir };
    let full_path = target_dir.join(&filename);

    // 8. Overwrite check
    if full_path.exists() {
        let msg = format!(
            "File '{}' already exists. Overwrite it?",
            full_path.display()
        );
        if !prompt_bool(&msg, false) {
            println!("Save canceled to avoid overwriting the existing file.");
            return;
        }
    }

    // 9. Write file (avoid sudo if already running as root)
    let needs_sudo = is_system && !is_running_as_root();
    if needs_sudo {
        if !write_file_with_sudo(&full_path, &target_dir, &content) {
            eprintln!("Failed to save file with sudo.");
            return;
        }
    } else {
        if let Err(e) = fs::create_dir_all(&target_dir) {
            eprintln!(
                "Failed to create directory '{}': {}",
                target_dir.display(),
                e
            );
            return;
        }
        if let Err(e) = fs::write(&full_path, &content) {
            eprintln!("Failed to write file '{}': {}", full_path.display(), e);
            return;
        }
    }

    println!("Saved successfully to: {}", full_path.display());

    // 10. Validate with desktop-file-validate (if available)
    match Command::new("desktop-file-validate").arg(&full_path).output() {
        Ok(output) => {
            if output.status.success() {
                println!("Validation (desktop-file-validate): Passed.");
            } else {
                let out = String::from_utf8_lossy(&output.stdout);
                let err = String::from_utf8_lossy(&output.stderr);
                println!("Validation warnings/errors:\n{}{}", out, err);
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            println!("Note: 'desktop-file-validate' was not found; skipping validation.");
        }
        Err(e) => println!("Could not run 'desktop-file-validate': {}", e),
    }

    // 11. Refresh desktop database for applications/ (if available)
    if entry.entry_type != "Directory" {
        let db_res = if needs_sudo {
            Command::new("sudo")
                .arg("update-desktop-database")
                .arg(&target_dir)
                .status()
        } else {
            Command::new("update-desktop-database")
                .arg(&target_dir)
                .status()
        };

        match db_res {
            Ok(status) if status.success() => {
                println!("Desktop database updated (update-desktop-database).");
            }
            Ok(_) => println!("Note: 'update-desktop-database' exited with a non-zero status."),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                println!("Note: 'update-desktop-database' was not found; skipping database update.");
            }
            Err(e) => println!("Could not run 'update-desktop-database': {}", e),
        }
    }
}

#[cfg(test)]
mod tests;