use std::process::Command;

use crate::paths::USER_THEME_EXTENSION;

pub fn user_themes_enabled() -> bool {
    let output = Command::new("gnome-extensions")
        .args(["info", USER_THEME_EXTENSION])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.contains("State: ENABLED")
        }
        _ => false,
    }
}
