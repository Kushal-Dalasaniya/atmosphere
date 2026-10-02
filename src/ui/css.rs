use std::sync::atomic::{AtomicU32, Ordering};

use gtk::prelude::*;

static TINT_SEQ: AtomicU32 = AtomicU32::new(0);

/// Tint a widget with an arbitrary hex color.
///
/// Uses a display-wide provider plus a unique class per call, avoiding the
/// per-widget `style_context().add_provider()` API deprecated since GTK 4.10.
/// Invalid hex is ignored (never interpolated), so theme-file content cannot
/// break out of the `background-color` value context.
pub fn tint(widget: &impl IsA<gtk::Widget>, hex: &str) {
    if !crate::theme::validate::is_valid_hex(hex) {
        return;
    }
    paint(
        widget,
        &format!("background-color: {hex}; border-radius: 10px;"),
    );
}

/// Paint a translucent glass tint over a widget (Create-view wallpaper
/// preview). Invalid hex or out-of-range alpha leaves the widget untouched.
pub fn tint_rgba(widget: &impl IsA<gtk::Widget>, hex: &str, alpha: f32) {
    if !crate::theme::validate::is_valid_hex(hex) || !(0.0..=1.0).contains(&alpha) {
        return;
    }
    let rgba = crate::theme::glass::hex_to_rgba(hex, alpha);
    paint(widget, &format!("background-color: {rgba};"));
}

fn paint(widget: &impl IsA<gtk::Widget>, declarations: &str) {
    // Drop previous tints so repeated refreshes don't stack providers.
    for class in widget.css_classes() {
        if class.starts_with("atmo-tint-") {
            widget.remove_css_class(&class);
        }
    }
    let class = format!("atmo-tint-{}", TINT_SEQ.fetch_add(1, Ordering::Relaxed));
    let css = gtk::CssProvider::new();
    css.load_from_string(&format!(".{class} {{ {declarations} }}"));
    gtk::style_context_add_provider_for_display(
        &widget.display(),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    widget.add_css_class(&class);
}
