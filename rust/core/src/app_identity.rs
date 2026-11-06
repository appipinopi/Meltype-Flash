// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Known native application identifiers, without globally stripping extensions.
#[derive(Clone, Copy)]
pub enum Platform {
    Windows,
    Linux,
    Mac,
}
impl Platform {
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }
}
pub fn legacy_rule(identity: &str, platform: Platform) -> Option<&'static str> {
    match (platform, identity) {
        (
            Platform::Linux,
            "code" | "code-oss" | "codium" | "com.visualstudio.code" | "com.vscodium.codium",
        )
        | (Platform::Mac, "com.microsoft.VSCode" | "com.vscodium") => Some("Code.exe"),
        (Platform::Linux, "code-insiders") | (Platform::Mac, "com.microsoft.VSCodeInsiders") => {
            Some("Code - Insiders.exe")
        }
        (Platform::Linux, "cursor") | (Platform::Mac, "com.todesktop.230313mzl4w4u92") => {
            Some("Cursor.exe")
        }
        (Platform::Linux, "windsurf") | (Platform::Mac, "com.exafunction.windsurf") => {
            Some("Windsurf.exe")
        }
        (Platform::Linux, "zed") | (Platform::Mac, "dev.zed.Zed") => Some("zed.exe"),
        (Platform::Linux, "sublime_text")
        | (Platform::Mac, "com.sublimetext.4" | "com.sublimetext.3") => Some("sublime_text.exe"),
        (
            Platform::Linux,
            "gnome-terminal"
            | "org.gnome.Terminal"
            | "org.kde.konsole"
            | "gnome-terminal-server"
            | "konsole"
            | "kitty"
            | "xterm"
            | "foot"
            | "wezterm"
            | "alacritty"
            | "rio",
        )
        | (
            Platform::Mac,
            "com.apple.Terminal"
            | "com.googlecode.iterm2"
            | "com.github.wez.wezterm"
            | "org.alacritty"
            | "net.kovidgoyal.kitty",
        ) => Some("WindowsTerminal.exe"),
        _ => None,
    }
}

/// IBus also sends generic toolkit names. Only GTK names carry application IDs.
pub fn ibus_application(client: &str) -> &str {
    ["gtk-im:", "gtk3-im:", "gtk4-im:"]
        .iter()
        .find_map(|prefix| client.strip_prefix(prefix))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linux_desktop_ids_resolve_to_existing_profiles() {
        assert_eq!(
            legacy_rule("com.visualstudio.code", Platform::Linux),
            Some("Code.exe")
        );
        assert_eq!(
            legacy_rule("org.gnome.Terminal", Platform::Linux),
            Some("WindowsTerminal.exe")
        );
        assert_eq!(
            legacy_rule("org.kde.konsole", Platform::Linux),
            Some("WindowsTerminal.exe")
        );
        assert_eq!(
            legacy_rule("com.visualstudio.code", Platform::Windows),
            None
        );
    }
}
