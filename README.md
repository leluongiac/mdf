 <div align="center">

# mdf

**Make Desktop File** — An interactive Linux CLI for generating Freedesktop-compliant `.desktop` and `.directory` entries without memorizing spec keys.

<p>
  <img src="https://img.shields.io/badge/Rust-2024%20Edition-fab387?style=for-the-badge&logo=rust&logoColor=fab387&labelColor=1e1e2e" alt="Rust 2024 Edition" />
  <img src="https://img.shields.io/badge/Platform-Linux-fab387?style=for-the-badge&logo=linux&logoColor=fab387&labelColor=1e1e2e" alt="Linux" />
  <img src="https://img.shields.io/badge/Freedesktop-Spec%20v1.5-fab387?style=for-the-badge&labelColor=1e1e2e" alt="Freedesktop Spec v1.5" />
  <img src="https://img.shields.io/badge/License-GPL--2.0-fab387?style=for-the-badge&labelColor=1e1e2e" alt="License GPL-2.0" />
</p>

</div>

---

## Features

- **Interactive TUI:** Step-by-step prompts for `Application`, `Link`, and `Directory` entries (`Enter` defaults to `Application`; `Esc` or `Ctrl+C` exits cleanly).
- **Full Standard Key Support:** Covers essential keys (`Name`, `Exec`, `Icon`, `Comment`, `Terminal`, `Categories`) plus an optional advanced menu (`StartupWMClass`, `MimeType`, `Keywords`, `Actions`, `OnlyShowIn` / `NotShowIn`, `DBusActivatable`, `PrefersNonDefaultGPU`, and more).
- **Desktop Actions (`[Desktop Action ...]`):** Interactively add right-click context menu actions with auto-generated, validated action IDs.
- **Spec-Compliant Formatting:** Automatically handles `.desktop` vs `.directory` extensions, smartly quotes `Exec` paths containing spaces, normalizes semicolon lists, and escapes special characters.
- **Safe System Integration:** Respects `$XDG_DATA_HOME`, avoids unnecessary `sudo` when running as root, writes system-wide entries via `sudo tee`, and integrates with `desktop-file-validate` and `update-desktop-database`.

---

## Installation

### Quick Install (`install.sh`)

Run the installer script directly:

```bash
curl -fsSL https://raw.githubusercontent.com/leluongiac/mdf/main/install.sh | bash
```

Or download and inspect it first:

```bash
curl -fsSLO https://raw.githubusercontent.com/leluongiac/mdf/main/install.sh
chmod +x install.sh
./install.sh
```

### Via Cargo

If you already have Rust and Cargo installed:

```bash
cargo install --git https://github.com/leluongiac/mdf.git
```

---

## Usage

Launch the interactive prompt from anywhere in your terminal:

```bash
mdf
```

1. Select the entry type (Application, Link, or Directory).
2. Fill in basic fields (Name, Exec/URL, Icon, Comment, Terminal, Categories).
3. Optionally configure advanced keys and `[Desktop Action ...]` entries.
4. Preview the generated file, adjust the suggested filename if needed, and save to user (`~/.local/share/...`) or system-wide (`/usr/share/...`) directories.

### Example Output

```ini
[Desktop Entry]
Type=Application
Name=My App
Comment=A fast desktop utility
Icon=my-app
Exec="/home/user/My Apps/my-app.AppImage"
Terminal=false
Categories=Utility;
StartupWMClass=my-app
Actions=open-logs;

[Desktop Action open-logs]
Name=Open Logs
Exec=sh -c "less ~/.local/state/my-app.log"
```

---

## Optional Dependencies

- **desktop-file-utils** (`desktop-file-validate` and `update-desktop-database`) — automatically used when available to validate generated entries and refresh your application menu cache.

---

## License

Licensed under the GNU General Public License v2.0 (GPL-2.0).
