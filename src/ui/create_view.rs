use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, DropTarget, Entry, FileDialog, FileFilter, Label, Orientation, Overlay,
    Picture, ScrolledWindow, Switch,
};

use crate::theme::apply;
use crate::theme::extract;
use crate::theme::glass::panel_alpha;
use crate::theme::model::{
    self, ColorMode, GlassSettings, GlassStrength, IconsSettings, Palette, Theme,
    default_name_from_wallpaper,
};
use crate::theme::validate::{checked_hex, importable_regular_file};
use crate::{paths, thumbs};

/// Swatch roles in spec order: Background, Surface, Foreground, Accent, On accent.
const SWATCH_ROLES: [&str; 5] = ["Background", "Surface", "Foreground", "Accent", "On accent"];

fn role_value<'a>(palette: &'a Palette, role: &str) -> &'a str {
    match role {
        "Background" => &palette.background,
        "Surface" => &palette.surface,
        "Foreground" => &palette.foreground,
        "Accent" => &palette.accent,
        _ => &palette.on_accent,
    }
}

pub struct CreateView {
    pub widget: GtkBox,
    inner: Rc<CreateInner>,
}

struct SwatchRow {
    role: &'static str,
    entry: Entry,
    preview: GtkBox,
}

struct CreateInner {
    name_entry: Entry,
    dark_switch: Switch,
    glass_switch: Switch,
    glass_subtle_btn: Button,
    glass_strong_btn: Button,
    icons_switch: Switch,
    status: Label,
    note: Label,
    wallpapers_box: GtkBox,
    preview_picture: Picture,
    preview_tint: GtkBox,
    swatches: Vec<SwatchRow>,
    selected_wallpaper: RefCell<Option<PathBuf>>,
    extracted: RefCell<Option<Palette>>,
    edited: Cell<bool>,
    glass_strength: RefCell<GlassStrength>,
    open_themes: Rc<dyn Fn()>,
}

impl CreateView {
    pub fn new(open_themes: Rc<dyn Fn()>) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 12);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let title = Label::new(Some("Create"));
        title.add_css_class("title-2");
        root.append(&title);

        let name_entry = Entry::new();
        name_entry.set_placeholder_text(Some("Theme name"));
        root.append(&name_entry);

        let mode_row = GtkBox::new(Orientation::Horizontal, 8);
        mode_row.append(&Label::new(Some("Dark mode")));
        let dark_switch = Switch::new();
        dark_switch.set_active(true);
        mode_row.append(&dark_switch);
        root.append(&mode_row);

        let glass_row = GtkBox::new(Orientation::Horizontal, 8);
        glass_row.append(&Label::new(Some("Glass effect")));
        let glass_switch = Switch::new();
        glass_row.append(&glass_switch);
        root.append(&glass_row);

        let strength_row = GtkBox::new(Orientation::Horizontal, 8);
        let glass_subtle_btn = Button::with_label("Subtle");
        let glass_strong_btn = Button::with_label("Strong");
        glass_subtle_btn.set_visible(false);
        glass_strong_btn.set_visible(false);
        strength_row.append(&glass_subtle_btn);
        strength_row.append(&glass_strong_btn);
        root.append(&strength_row);

        // Two-tone Yaru folders, matched to the accent (spec §4.12).
        let icons_row = GtkBox::new(Orientation::Horizontal, 8);
        icons_row.append(&Label::new(Some("Match folder icons (Yaru)")));
        let icons_switch = Switch::new();
        icons_switch.set_active(true);
        icons_row.append(&icons_switch);
        root.append(&icons_row);
        let icons_help = Label::new(Some(
            "Keeps Ubuntu’s two-color folders; picks the Yaru set closest to your accent.",
        ));
        icons_help.set_wrap(true);
        icons_help.add_css_class("dim-label");
        root.append(&icons_help);

        root.append(&Label::new(Some("Wallpapers")));

        let wallpapers_box = GtkBox::new(Orientation::Vertical, 6);
        let scroll = ScrolledWindow::builder()
            .height_request(200)
            .child(&wallpapers_box)
            .build();
        root.append(&scroll);

        let note = Label::new(None);
        note.set_wrap(true);
        note.set_visible(false);
        root.append(&note);

        let add_btn = Button::with_label("Add wallpaper");
        root.append(&add_btn);

        // Live glass preview: selected wallpaper with a tint overlay while
        // the Glass switch is on, plain thumbnail while off.
        root.append(&Label::new(Some("Preview")));
        let preview_picture = Picture::builder()
            .height_request(160)
            .can_shrink(true)
            .content_fit(gtk::ContentFit::Cover)
            .build();
        let preview_tint = GtkBox::new(Orientation::Horizontal, 0);
        preview_tint.set_hexpand(true);
        preview_tint.set_vexpand(true);
        preview_tint.set_halign(gtk::Align::Fill);
        preview_tint.set_valign(gtk::Align::Fill);
        preview_tint.set_can_target(false);
        preview_tint.set_visible(false);
        let preview_overlay = Overlay::new();
        preview_overlay.set_child(Some(&preview_picture));
        preview_overlay.add_overlay(&preview_tint);
        preview_overlay.set_height_request(160);
        root.append(&preview_overlay);

        let swatch_box = GtkBox::new(Orientation::Vertical, 6);
        let mut swatches = Vec::new();
        for role in SWATCH_ROLES {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let label = Label::new(Some(role));
            label.set_width_chars(10);
            label.set_halign(gtk::Align::Start);
            let entry = Entry::new();
            entry.set_placeholder_text(Some("#rrggbb"));
            entry.set_hexpand(true);
            let preview = GtkBox::new(Orientation::Horizontal, 0);
            preview.set_size_request(28, 28);
            row.append(&label);
            row.append(&entry);
            row.append(&preview);
            swatch_box.append(&row);
            swatches.push(SwatchRow {
                role,
                entry,
                preview,
            });
        }
        root.append(&swatch_box);

        let extract_btn = Button::with_label("Extract colors");
        root.append(&extract_btn);

        let reset_btn = Button::with_label("Reset to last extraction");
        root.append(&reset_btn);

        let save_btn = Button::with_label("Save");
        let save_apply_btn = Button::with_label("Save and apply");
        let btn_row = GtkBox::new(Orientation::Horizontal, 8);
        btn_row.append(&save_btn);
        btn_row.append(&save_apply_btn);
        root.append(&btn_row);

        let status = Label::new(None);
        status.set_wrap(true);
        root.append(&status);

        let inner = Rc::new(CreateInner {
            name_entry,
            dark_switch,
            glass_switch,
            glass_subtle_btn,
            glass_strong_btn,
            icons_switch,
            status,
            note,
            wallpapers_box,
            preview_picture,
            preview_tint,
            swatches,
            selected_wallpaper: RefCell::new(None),
            extracted: RefCell::new(None),
            edited: Cell::new(false),
            glass_strength: RefCell::new(GlassStrength::Subtle),
            open_themes,
        });

        {
            let inner = inner.clone();
            let switch = inner.glass_switch.clone();
            switch.connect_active_notify(move |sw| {
                let on = sw.is_active();
                inner.glass_subtle_btn.set_visible(on);
                inner.glass_strong_btn.set_visible(on);
                refresh_glass_preview(&inner);
            });
        }
        {
            let inner = inner.clone();
            let btn = inner.glass_subtle_btn.clone();
            btn.connect_clicked(move |_| {
                *inner.glass_strength.borrow_mut() = GlassStrength::Subtle;
                inner.status.set_text("Glass strength: Subtle");
                refresh_glass_preview(&inner);
            });
        }
        {
            let inner = inner.clone();
            let btn = inner.glass_strong_btn.clone();
            btn.connect_clicked(move |_| {
                *inner.glass_strength.borrow_mut() = GlassStrength::Strong;
                inner.status.set_text("Glass strength: Strong");
                refresh_glass_preview(&inner);
            });
        }
        // Dark/light re-runs extraction for that mode; it never inverts colors.
        {
            let inner = inner.clone();
            let switch = inner.dark_switch.clone();
            switch.connect_active_notify(move |_| {
                if inner.selected_wallpaper.borrow().is_some() {
                    run_extract(inner.clone());
                }
            });
        }
        for row in &inner.swatches {
            let inner = inner.clone();
            let preview = row.preview.clone();
            row.entry.connect_changed(move |entry| {
                inner.edited.set(true);
                paint_preview(&preview, entry.text().as_str());
                refresh_glass_preview(&inner);
            });
        }
        {
            let inner = inner.clone();
            extract_btn.connect_clicked(move |_| run_extract(inner.clone()));
        }
        {
            let inner = inner.clone();
            reset_btn.connect_clicked(move |_| {
                if let Some(palette) = inner.extracted.borrow().clone() {
                    fill_swatches(&inner, &palette);
                    inner.edited.set(false);
                    inner.status.set_text("Restored the last extraction.");
                } else {
                    inner
                        .status
                        .set_text("Nothing to reset yet — run Extract first.");
                }
            });
        }
        {
            let inner = inner.clone();
            add_btn.connect_clicked(move |_| choose_wallpaper_file(inner.clone()));
        }
        {
            let inner = inner.clone();
            save_btn.connect_clicked(move |_| {
                if save_theme(&inner, false) {
                    (inner.open_themes)();
                }
            });
        }
        {
            let inner = inner.clone();
            save_apply_btn.connect_clicked(move |_| {
                save_theme(&inner, true);
            });
        }

        // Drag-and-drop of an image onto the view selects it as wallpaper.
        {
            let inner = inner.clone();
            let drop = DropTarget::new(gtk::glib::Type::OBJECT, gtk::gdk::DragAction::COPY);
            drop.connect_drop(move |_, value, _, _| {
                if let Ok(file) = value.get::<gio::File>() {
                    import_dropped_file(inner.clone(), file);
                    true
                } else {
                    false
                }
            });
            root.add_controller(drop);
        }

        let view = Self {
            widget: root,
            inner,
        };
        view.reload_wallpapers();
        view
    }

    pub fn reload_wallpapers(&self) {
        reload_wallpaper_strip(&self.inner);
    }
}

fn reload_wallpaper_strip(inner: &Rc<CreateInner>) {
    while let Some(child) = inner.wallpapers_box.first_child() {
        inner.wallpapers_box.remove(&child);
    }
    inner.note.set_text("");
    inner.note.set_visible(false);
    for path in thumbs::list_wallpapers() {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let btn = Button::with_label(&file_name);
        btn.set_halign(gtk::Align::Fill);
        let select_inner = inner.clone();
        let select_path = path.clone();
        btn.connect_clicked(move |_| select_wallpaper(&select_inner, &select_path));
        inner.wallpapers_box.append(&btn);

        // Probe decodability off the main thread; failures land in the
        // "could not read" note instead of crashing or stalling the UI.
        let note = inner.note.clone();
        let entry_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        glib::spawn_future_local(async move {
            let checked = gio::spawn_blocking(move || thumbs::ensure_thumbnail(&path)).await;
            if matches!(checked, Ok(Err(_))) {
                let current = note.text().to_string();
                if current.is_empty() {
                    note.set_text(&format!("Could not read: {entry_name}"));
                } else {
                    note.set_text(&format!("{current}, {entry_name}"));
                }
                note.set_visible(true);
            }
        });
    }
}

fn select_wallpaper(inner: &Rc<CreateInner>, path: &std::path::Path) {
    *inner.selected_wallpaper.borrow_mut() = Some(path.to_path_buf());
    inner
        .name_entry
        .set_text(&default_name_from_wallpaper(path));
    inner
        .status
        .set_text(&format!("Selected: {}", path.display()));
    // Thumbnail for the glass preview, decoded off the main thread.
    let picture = inner.preview_picture.clone();
    let probe = path.to_path_buf();
    glib::spawn_future_local(async move {
        let thumb = gio::spawn_blocking(move || thumbs::ensure_thumbnail(&probe)).await;
        if let Ok(Ok(dest)) = thumb {
            picture.set_filename(Some(dest));
        }
    });
    refresh_glass_preview(inner);
}

/// Glass preview overlay (spec §3.2 + contracts ui.md): a tinted overlay on
/// the wallpaper thumbnail while the switch is on, nothing while off.
/// The tint follows the live Surface swatch and the current strength.
fn refresh_glass_preview(inner: &Rc<CreateInner>) {
    if !inner.glass_switch.is_active() {
        inner.preview_tint.set_visible(false);
        return;
    }
    let surface = inner
        .swatches
        .iter()
        .find(|row| row.role == "Surface")
        .map(|row| row.entry.text().to_string());
    let alpha = panel_alpha(&GlassSettings {
        enabled: true,
        strength: *inner.glass_strength.borrow(),
    });
    match (surface, alpha) {
        (Some(hex), Some(a)) => {
            crate::ui::css::tint_rgba(&inner.preview_tint, &hex, a);
            inner.preview_tint.set_visible(true);
        }
        _ => inner.preview_tint.set_visible(false),
    }
}

fn fill_swatches(inner: &Rc<CreateInner>, palette: &Palette) {
    for row in &inner.swatches {
        let value = role_value(palette, row.role);
        row.entry.set_text(value);
        paint_preview(&row.preview, value);
    }
}

fn paint_preview(preview: &GtkBox, hex: &str) {
    crate::ui::css::tint(preview, hex);
}

fn current_mode(inner: &Rc<CreateInner>) -> ColorMode {
    if inner.dark_switch.is_active() {
        ColorMode::Dark
    } else {
        ColorMode::Light
    }
}

fn run_extract(inner: Rc<CreateInner>) {
    let path = inner.selected_wallpaper.borrow().clone();
    let Some(path) = path else {
        inner.status.set_text("Select a wallpaper first");
        return;
    };
    let mode = current_mode(&inner);
    inner.status.set_text("Extracting colors…");
    let inner2 = inner.clone();
    glib::spawn_future_local(async move {
        let result =
            gio::spawn_blocking(move || extract::extract_palette_from_wallpaper(&path, mode)).await;
        match result {
            Ok(Ok(palette)) => {
                fill_swatches(&inner2, &palette);
                *inner2.extracted.borrow_mut() = Some(palette);
                inner2.edited.set(false);
                refresh_glass_preview(&inner2);
                inner2
                    .status
                    .set_text("Colors extracted. Edit swatches, adjust glass, then save.");
            }
            Ok(Err(e)) => inner2
                .status
                .set_text(&format!("Extract failed: {}", first_line(&e.to_string()))),
            Err(e) => inner2.status.set_text(&format!("Extract failed: {e:?}")),
        }
    });
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

/// Copy `src` into the wallpaper library (numeric suffix on collision) and
/// select it. Regular files only; symlink escapes are refused.
fn import_dropped_file(inner: Rc<CreateInner>, src: gio::File) {
    inner.status.set_text("Importing wallpaper…");
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || import_wallpaper_file(&src)).await;
        match result {
            Ok(Ok(path)) => {
                reload_wallpaper_strip(&inner);
                select_wallpaper(&inner, &path);
            }
            Ok(Err(e)) => inner.status.set_text(&format!("Import failed: {e}")),
            Err(e) => inner.status.set_text(&format!("Import failed: {e:?}")),
        }
    });
}

fn import_wallpaper_file(src: &gio::File) -> Result<PathBuf, String> {
    use std::fs;
    let src_path = src
        .path()
        .ok_or_else(|| "Could not read the dropped file".to_string())?;
    let canonical =
        importable_regular_file(&src_path).map_err(|e| first_line(&e.to_string()).to_string())?;
    let dir = paths::wallpapers_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir_canon = std::fs::canonicalize(&dir).unwrap_or(dir.clone());
    if canonical.parent().map(|p| p == dir_canon).unwrap_or(false) {
        return Ok(canonical);
    }
    let stem = canonical
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("wallpaper");
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut n = 1u32;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    fs::copy(&canonical, &candidate).map_err(|e| e.to_string())?;
    Ok(candidate)
}

fn choose_wallpaper_file(inner: Rc<CreateInner>) {
    let dialog = FileDialog::builder()
        .title("Add wallpaper")
        .modal(true)
        .build();
    let images = FileFilter::new();
    images.set_name(Some("Images"));
    for pattern in ["*.jpg", "*.jpeg", "*.png", "*.webp"] {
        images.add_pattern(pattern);
    }
    dialog.set_default_filter(Some(&images));
    // FileDialog is async natively; the callback runs on the main loop.
    dialog.open(
        None::<&gtk::Window>,
        None::<&gio::Cancellable>,
        move |result: Result<gio::File, glib::Error>| match result {
            Ok(file) => import_dropped_file(inner, file),
            Err(e) => inner
                .status
                .set_text(&format!("Choose failed: {}", first_line(&e.to_string()))),
        },
    );
}

/// Save the theme (plus `preview.png`); optionally apply it on a worker.
/// Returns true when the theme was saved.
fn save_theme(inner: &Rc<CreateInner>, and_apply: bool) -> bool {
    let wallpaper = match inner.selected_wallpaper.borrow().clone() {
        Some(path) => path,
        None => {
            inner.status.set_text("Select a wallpaper first");
            return false;
        }
    };

    let name = inner.name_entry.text().to_string();
    if name.trim().is_empty() {
        inner.status.set_text("Enter a theme name");
        return false;
    }
    if inner.extracted.borrow().is_none() {
        inner.status.set_text("Run Extract colors first");
        return false;
    }

    let mut values = Vec::new();
    for row in &inner.swatches {
        match checked_hex(row.entry.text().as_str(), row.role) {
            Ok(hex) => values.push(hex),
            Err(_) => {
                inner
                    .status
                    .set_text(&format!("{} must be a #rrggbb color", row.role));
                return false;
            }
        }
    }
    let stored = inner.extracted.borrow().clone();
    let palette = Palette {
        background: values[0].clone(),
        surface: values[1].clone(),
        foreground: values[2].clone(),
        accent: values[3].clone(),
        on_accent: values[4].clone(),
        // Stored extended slots survive hand edits; a fresh Extract
        // re-derives them from the new extraction.
        error: stored.as_ref().and_then(|p| p.error.clone()),
        secondary: stored.as_ref().and_then(|p| p.secondary.clone()),
        tertiary: stored.as_ref().and_then(|p| p.tertiary.clone()),
    };

    let theme = Theme {
        name,
        mode: current_mode(inner),
        wallpaper: wallpaper.clone(),
        palette_edited: inner.edited.get(),
        glass: GlassSettings {
            enabled: inner.glass_switch.is_active(),
            strength: *inner.glass_strength.borrow(),
        },
        icons: IconsSettings {
            match_yaru: inner.icons_switch.is_active(),
        },
        palette,
    };

    let id = match model::save_theme(&theme) {
        Ok(id) => id,
        Err(e) => {
            inner
                .status
                .set_text(&format!("Save failed: {}", first_line(&e.to_string())));
            return false;
        }
    };
    if let Err(e) = thumbs::write_preview(&wallpaper, &model::theme_dir(&id)) {
        inner.status.set_text(&format!(
            "Saved “{}” ({id}), but preview failed: {}",
            theme.name,
            first_line(&e.to_string())
        ));
    } else {
        inner
            .status
            .set_text(&format!("Saved theme “{}” ({id})", theme.name));
    }

    if and_apply {
        inner
            .status
            .set_text(&format!("Saved. Applying “{}”…", theme.name));
        let inner = inner.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || apply::apply_theme(&theme)).await;
            match result {
                Ok(Ok(())) => inner
                    .status
                    .set_text(&format!("Saved and applied “{}”", inner.name_entry.text())),
                Ok(Err(e)) => inner.status.set_text(&format!(
                    "Saved, but apply failed: {}",
                    first_line(&e.to_string())
                )),
                Err(e) => inner
                    .status
                    .set_text(&format!("Saved, but apply failed: {e:?}")),
            }
        });
    }
    true
}
