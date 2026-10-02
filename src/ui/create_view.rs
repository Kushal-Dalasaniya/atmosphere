use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, Entry, Label, Orientation, ScrolledWindow, Switch,
};

use crate::theme::apply;
use crate::theme::extract;
use crate::theme::model::{
    self, ColorMode, GlassSettings, GlassStrength, Theme, default_name_from_wallpaper,
};
use crate::thumbs;

pub struct CreateView {
    pub widget: GtkBox,
    inner: Rc<CreateInner>,
}

struct CreateInner {
    name_entry: Entry,
    dark_switch: Switch,
    glass_switch: Switch,
    glass_subtle_btn: Button,
    glass_strong_btn: Button,
    status: Label,
    wallpapers_box: GtkBox,
    draft: RefCell<Option<Theme>>,
    selected_wallpaper: RefCell<Option<PathBuf>>,
    glass_strength: RefCell<GlassStrength>,
}

impl CreateView {
    pub fn new() -> Self {
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

        root.append(&Label::new(Some("Wallpapers")));

        let wallpapers_box = GtkBox::new(Orientation::Vertical, 6);
        let scroll = ScrolledWindow::builder()
            .height_request(200)
            .child(&wallpapers_box)
            .build();
        root.append(&scroll);

        let extract_btn = Button::with_label("Extract colors");
        root.append(&extract_btn);

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
            status,
            wallpapers_box,
            draft: RefCell::new(None),
            selected_wallpaper: RefCell::new(None),
            glass_strength: RefCell::new(GlassStrength::Subtle),
        });

        {
            let inner = inner.clone();
            glass_switch.connect_active_notify(move |sw| {
                let on = sw.is_active();
                inner.glass_subtle_btn.set_visible(on);
                inner.glass_strong_btn.set_visible(on);
            });
        }

        {
            let inner = inner.clone();
            glass_subtle_btn.connect_clicked(move |_| {
                *inner.glass_strength.borrow_mut() = GlassStrength::Subtle;
                inner.status.set_text("Glass strength: Subtle");
            });
        }
        {
            let inner = inner.clone();
            glass_strong_btn.connect_clicked(move |_| {
                *inner.glass_strength.borrow_mut() = GlassStrength::Strong;
                inner.status.set_text("Glass strength: Strong");
            });
        }

        {
            let inner = inner.clone();
            extract_btn.connect_clicked(move |_| run_extract(inner.clone()));
        }
        {
            let inner = inner.clone();
            save_btn.connect_clicked(move |_| {
                if let Err(e) = save_theme(&inner, false) {
                    inner.status.set_text(&format!("Save failed: {}", e));
                }
            });
        }
        {
            let inner = inner.clone();
            save_apply_btn.connect_clicked(move |_| {
                if let Err(e) = save_theme(&inner, true) {
                    inner.status.set_text(&format!("Save failed: {}", e));
                }
            });
        }

        let view = Self {
            widget: root,
            inner,
        };
        view.reload_wallpapers();
        view
    }

    pub fn reload_wallpapers(&self) {
        while let Some(child) = self.inner.wallpapers_box.first_child() {
            self.inner.wallpapers_box.remove(&child);
        }
        for path in thumbs::list_wallpapers() {
            let btn = Button::with_label(&path.file_name().unwrap_or_default().to_string_lossy());
            let path = path.clone();
            let inner = self.inner.clone();
            btn.connect_clicked(move |_| {
                *inner.selected_wallpaper.borrow_mut() = Some(path.clone());
                inner
                    .name_entry
                    .set_text(&default_name_from_wallpaper(&path));
                inner
                    .status
                    .set_text(&format!("Selected: {}", path.display()));
            });
            self.inner.wallpapers_box.append(&btn);
        }
    }
}

fn run_extract(inner: Rc<CreateInner>) {
    let path = inner.selected_wallpaper.borrow().clone();
    let Some(path) = path else {
        inner.status.set_text("Select a wallpaper first");
        return;
    };
    let mode = if inner.dark_switch.is_active() {
        ColorMode::Dark
    } else {
        ColorMode::Light
    };
    let inner2 = inner.clone();
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || {
            extract::extract_palette_from_wallpaper(&path, mode)
        })
        .await;
        match result {
            Ok(Ok(palette)) => {
                let theme = Theme {
                    name: default_name_from_wallpaper(&inner2.selected_wallpaper.borrow().as_ref().unwrap()),
                    mode,
                    wallpaper: inner2.selected_wallpaper.borrow().clone().unwrap(),
                    palette_edited: false,
                    glass: GlassSettings::default(),
                    palette,
                };
                *inner2.draft.borrow_mut() = Some(theme);
                inner2.status.set_text("Colors extracted. Adjust glass and save.");
            }
            Ok(Err(e)) => inner2.status.set_text(&format!("Extract failed: {}", e)),
            Err(e) => inner2.status.set_text(&format!("Extract failed: {}", e)),
        }
    });
}

fn save_theme(inner: &CreateInner, and_apply: bool) -> Result<(), String> {
    let wallpaper = inner
        .selected_wallpaper
        .borrow()
        .clone()
        .or_else(|| inner.draft.borrow().as_ref().map(|t| t.wallpaper.clone()))
        .ok_or_else(|| "Select a wallpaper first".to_string())?;

    let name = inner.name_entry.text().to_string();
    if name.trim().is_empty() {
        return Err("Enter a theme name".into());
    }

    let mode = if inner.dark_switch.is_active() {
        ColorMode::Dark
    } else {
        ColorMode::Light
    };

    let glass = GlassSettings {
        enabled: inner.glass_switch.is_active(),
        strength: *inner.glass_strength.borrow(),
    };

    let palette = inner
        .draft
        .borrow()
        .as_ref()
        .map(|t| t.palette.clone())
        .ok_or_else(|| "Run Extract colors first".to_string())?;

    let theme = Theme {
        name,
        mode,
        wallpaper,
        palette_edited: inner
            .draft
            .borrow()
            .as_ref()
            .map(|t| t.palette_edited)
            .unwrap_or(false),
        glass,
        palette,
    };

    let id = model::save_theme(&theme).map_err(|e| e.to_string())?;
    inner
        .status
        .set_text(&format!("Saved theme “{}” ({})", theme.name, id));

    if and_apply {
        apply::apply_theme(&theme).map_err(|e| e.to_string())?;
        inner
            .status
            .set_text(&format!("Saved and applied “{}”", theme.name));
    }
    Ok(())
}
