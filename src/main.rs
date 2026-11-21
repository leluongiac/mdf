use inquire::{Confirm, MultiSelect, Select, Text};
use std::env;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Default)]
struct DesktopEntry {
    // Cơ bản
    entry_type: String,
    name: String,
    comment: Option<String>,
    icon: Option<String>,

    // Dành riêng cho Application / Link
    exec: Option<String>,
    terminal: bool,
    categories: Vec<String>,
    url: Option<String>,

    // Nâng cao (theo thứ tự độ phổ biến)
    startup_wm_class: Option<String>,
    mime_type: Option<String>,
    keywords: Option<String>,
    generic_name: Option<String>,
    path: Option<String>,
    try_exec: Option<String>,
    no_display: Option<bool>,
    startup_notify: Option<bool>,
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
    // Chuyển toàn bộ dữ liệu trong struct thành chuỗi chuẩn file .desktop
    fn to_desktop_string(&self) -> String {
        let mut lines = vec!["[Desktop Entry]".to_string()];

        if let Some(v) = &self.version {
            lines.push(format!("Version={}", v));
        }
        lines.push(format!("Type={}", self.entry_type));
        lines.push(format!("Name={}", self.name));

        if let Some(v) = &self.generic_name {
            lines.push(format!("GenericName={}", v));
        }
        if let Some(v) = &self.comment {
            lines.push(format!("Comment={}", v));
        }
        if let Some(v) = &self.icon {
            lines.push(format!("Icon={}", v));
        }
        if let Some(v) = &self.exec {
            lines.push(format!("Exec={}", v));
        }
        if let Some(v) = &self.try_exec {
            lines.push(format!("TryExec={}", v));
        }
        if let Some(v) = &self.path {
            lines.push(format!("Path={}", v));
        }
        if let Some(v) = &self.url {
            lines.push(format!("URL={}", v));
        }
        if self.entry_type == "Application" {
            lines.push(format!("Terminal={}", self.terminal));
            if !self.categories.is_empty() {
                lines.push(format!("Categories={};", self.categories.join(";")));
            }
        }
        if let Some(v) = &self.mime_type {
            lines.push(format!("MimeType={}", v));
        }
        if let Some(v) = &self.keywords {
            lines.push(format!("Keywords={}", v));
        }
        if let Some(v) = &self.startup_wm_class {
            lines.push(format!("StartupWMClass={}", v));
        }
        if let Some(v) = self.no_display {
            lines.push(format!("NoDisplay={}", v));
        }
        if let Some(v) = self.hidden {
            lines.push(format!("Hidden={}", v));
        }
        if let Some(v) = &self.only_show_in {
            lines.push(format!("OnlyShowIn={}", v));
        }
        if let Some(v) = &self.not_show_in {
            lines.push(format!("NotShowIn={}", v));
        }
        if let Some(v) = self.startup_notify {
            lines.push(format!("StartupNotify={}", v));
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

        format!("{}\n", lines.join("\n"))
    }
}

fn prompt_required(label: &str) -> String {
    loop {
        let input = Text::new(label).prompt().expect("Lỗi khi nhập liệu");
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        println!("⚠️  Trường này bắt buộc, không được để trống!");
    }
}

fn prompt_optional(label: &str) -> Option<String> {
    let input = Text::new(label)
        .with_help_message("Bấm Enter để bỏ qua")
        .prompt()
        .expect("Lỗi khi nhập liệu");
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// Tự động chuẩn hóa chuỗi danh sách (vd: "code, editor" -> "code;editor;")
fn normalize_list(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw
        .split(|c| c == ';' || c == ',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(format!("{};", parts.join(";")))
    }
}

fn prompt_list(label: &str, help: &str) -> Option<String> {
    let input = Text::new(label)
        .with_help_message(help)
        .prompt()
        .expect("Lỗi khi nhập liệu");
    normalize_list(&input)
}

fn prompt_bool(label: &str, default: bool) -> bool {
    Confirm::new(label)
        .with_default(default)
        .prompt()
        .expect("Lỗi khi chọn true/false")
}

fn slugify_filename(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for c in name.chars() {
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
        "app.desktop".to_string()
    } else {
        format!("{}.desktop", slug)
    }
}

fn main() {
    println!("=== Tạo file .desktop với mdf ===\n");

    let types = vec!["Application", "Link", "Directory"];
    let entry_type = Select::new("Loại entry (Type):", types)
        .with_help_message("Enter ngay để chọn Application")
        .prompt()
        .expect("Lỗi khi chọn Type")
        .to_string();

    let name = prompt_required("Tên hiển thị (Name):");
    let comment = prompt_optional("Mô tả ngắn (Comment):");
    let icon = prompt_optional("Đường dẫn hoặc tên Icon (Icon):");

    let mut entry = DesktopEntry {
        entry_type: entry_type.clone(),
        name,
        comment,
        icon,
        ..Default::default()
    };

    match entry_type.as_str() {
        "Application" => {
            entry.exec = Some(prompt_required("Lệnh thực thi (Exec):"));
            entry.terminal = prompt_bool("Chạy trong Terminal?", false);

            let cat_options = vec![
                "Utility", "Development", "Network", "Office",
                "Graphics", "AudioVideo", "Game", "System", "Settings",
            ];
            entry.categories = MultiSelect::new("Danh mục (Categories):", cat_options)
                .with_help_message("Space để chọn/bỏ, Enter để tiếp tục")
                .prompt()
                .expect("Lỗi khi chọn Categories")
                .into_iter()
                .map(|s| s.to_string())
                .collect();
        }
        "Link" => {
            entry.url = Some(prompt_required("Đường dẫn URL:"));
        }
        _ => {}
    }

    // Hỏi có muốn cấu hình nâng cao không
    if prompt_bool("\nM có muốn cấu hình nâng cao không?", false) {
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
                "OnlyShowIn",
                "NotShowIn",
                "PrefersNonDefaultGPU",
                "SingleMainWindow",
                "Hidden",
                "DBusActivatable",
                "Implements",
                "Version",
            ]
        } else {
            vec![
                "Keywords",
                "GenericName",
                "NoDisplay",
                "OnlyShowIn",
                "NotShowIn",
                "Hidden",
                "Version",
            ]
        };

        let selected = MultiSelect::new("Chọn các tuỳ chọn nâng cao:", adv_options)
            .with_help_message("Space để tích chọn, Enter để nhập chi tiết")
            .prompt()
            .expect("Lỗi khi chọn menu nâng cao");

        for key in selected {
            match key {
                "StartupWMClass" => entry.startup_wm_class = prompt_optional("StartupWMClass (vd: code, obsidian):"),
                "MimeType" => entry.mime_type = prompt_list("MimeType:", "Ví dụ: text/plain, application/pdf"),
                "Keywords" => entry.keywords = prompt_list("Keywords:", "Ngăn cách bằng dấu phẩy (vd: editor, code)"),
                "GenericName" => entry.generic_name = prompt_optional("GenericName (vd: Web Browser):"),
                "Path" => entry.path = prompt_optional("Path (thư mục làm việc):"),
                "TryExec" => entry.try_exec = prompt_optional("TryExec (đường dẫn file kiểm tra tồn tại):"),
                "NoDisplay" => entry.no_display = Some(prompt_bool("NoDisplay (ẩn khỏi menu)?", true)),
                "StartupNotify" => entry.startup_notify = Some(prompt_bool("StartupNotify (hiện hiệu ứng khởi động)?", true)),
                "OnlyShowIn" => entry.only_show_in = prompt_list("OnlyShowIn:", "Ví dụ: GNOME, KDE, XFCE"),
                "NotShowIn" => entry.not_show_in = prompt_list("NotShowIn:", "Ví dụ: GNOME, KDE, XFCE"),
                "PrefersNonDefaultGPU" => entry.prefers_non_default_gpu = Some(prompt_bool("PrefersNonDefaultGPU (ưu tiên GPU rời)?", true)),
                "SingleMainWindow" => entry.single_main_window = Some(prompt_bool("SingleMainWindow (chỉ mở 1 cửa sổ chính)?", true)),
                "Hidden" => entry.hidden = Some(prompt_bool("Hidden (đánh dấu đã ẩn/xóa cấp user)?", true)),
                "DBusActivatable" => entry.dbus_activatable = Some(prompt_bool("DBusActivatable?", true)),
                "Implements" => entry.implements = prompt_list("Implements:", "Danh sách interface (ngăn cách bằng dấu phẩy)"),
                "Version" => entry.version = prompt_optional("Version của Desktop Entry spec (vd: 1.5):"),
                _ => {}
            }
        }
    }

    let content = entry.to_desktop_string();
    println!("\n📄 Xem trước nội dung file .desktop:\n----------------------------------------\n{}----------------------------------------", content);

    if !prompt_bool("M có muốn lưu file này không?", true) {
        println!("❌ Đã hủy lưu file.");
        return;
    }

    // 1. Gợi ý tên file và cho phép sửa
    let default_filename = slugify_filename(&entry.name);
    let raw_filename = Text::new("Tên file:")
        .with_default(&default_filename)
        .with_help_message("Enter để dùng tên gợi ý, hoặc gõ tên khác")
        .prompt()
        .expect("Lỗi khi nhập tên file");

    let raw_filename = raw_filename.trim();
    let filename = if raw_filename.is_empty() {
        default_filename
    } else if raw_filename.ends_with(".desktop") {
        raw_filename.to_string()
    } else {
        format!("{}.desktop", raw_filename)
    };

    // 2. Chọn nơi lưu
    let locations = vec![
        "User hiện tại (~/.local/share/applications/)",
        "Toàn hệ thống (/usr/share/applications/ - dùng sudo)",
    ];
    let choice = Select::new("Chọn vị trí lưu:", locations)
        .prompt()
        .expect("Lỗi khi chọn vị trí lưu");

    let is_system = choice.starts_with("Toàn hệ thống");
    let target_dir = if is_system {
        PathBuf::from("/usr/share/applications")
    } else {
        let home = env::var("HOME").expect("Không tìm thấy biến môi trường $HOME");
        PathBuf::from(home).join(".local/share/applications")
    };

    let full_path = target_dir.join(&filename);

    // 3. Kiểm tra trùng file
    if full_path.exists() {
        let msg = format!("File {:?} đã tồn tại. Ghi đè không?", full_path);
        if !prompt_bool(&msg, false) {
            println!("❌ Đã hủy để tránh ghi đè.");
            return;
        }
    }

    // 4. Ghi file
    if is_system {
        let mut child = Command::new("sudo")
            .arg("tee")
            .arg(&full_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("Không thể gọi lệnh sudo tee");

        // Dùng take() để đóng stdin sau khi ghi xong, tránh tee bị treo chờ EOF
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .expect("Lỗi khi truyền dữ liệu cho sudo tee");
        }

        let status = child.wait().expect("Lỗi khi chờ sudo tee");
        if !status.success() {
            eprintln!("❌ Lưu file vào hệ thống thất bại!");
            return;
        }
    } else {
        fs::create_dir_all(&target_dir).expect("Không thể tạo thư mục applications");
        fs::write(&full_path, &content).expect("Không thể ghi file");
    }

    println!("🎉 Đã lưu thành công tại: {:?}", full_path);

    // 5. Chạy desktop-file-validate (nếu máy có cài)
    match Command::new("desktop-file-validate").arg(&full_path).output() {
        Ok(output) => {
            if output.status.success() {
                println!("✅ Kiểm tra chuẩn (desktop-file-validate): Hợp lệ!");
            } else {
                let out = String::from_utf8_lossy(&output.stdout);
                let err = String::from_utf8_lossy(&output.stderr);
                println!("⚠️  Cảnh báo từ desktop-file-validate:\n{}{}", out, err);
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            println!("ℹ️  Máy chưa cài `desktop-file-validate`, bỏ qua bước kiểm tra.");
        }
        Err(e) => println!("⚠️  Lỗi khi chạy desktop-file-validate: {}", e),
    }

    // 6. Chạy update-desktop-database (nếu máy có cài)
    let db_res = if is_system {
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
            println!("🔄 Đã cập nhật menu ứng dụng (update-desktop-database)!");
        }
        Ok(_) => println!("⚠️  `update-desktop-database` chạy nhưng trả về mã lỗi."),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            println!("ℹ️  Máy chưa cài `update-desktop-database`, bỏ qua bước cập nhật.");
        }
        Err(e) => println!("⚠️  Lỗi khi chạy update-desktop-database: {}", e),
    }
}