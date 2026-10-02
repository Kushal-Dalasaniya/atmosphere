use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, Orientation, Picture, ScrolledWindow, Spinner};
use libadwaita::{Toast, ToastOverlay};

use crate::theme::{apply, baseline, model};

const SUCCESS_TOAST: &str = "Applied. Apps you open from now on use this theme.";
const EMPTY_TEXT: &str = "No saved themes yet. Create one from a wallpaper, then apply it here.";

pub struct ThemesView {
    pub widget: GtkBox,
    inner: Rc<Inner>,
}

struct Inner {
    list_box: GtkBox,
    on_applied: Rc<dyn Fn()>,
    toast: ToastOverlay,
    applying: Cell<bool>,
}

impl Clone for ThemesView {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl ThemesView {
    pub fn new(open_create: Rc<dyn Fn()>, on_applied: Rc<dyn Fn()>, toast: ToastOverlay) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 12);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let title = Label::new(Some("Themes"));
        title.add_css_class("title-2");
        root.append(&title);

        let list_box = GtkBox::new(Orientation::Vertical, 8);
        let scroll = ScrolledWindow::builder()
            .vexpand(true)
            .child(&list_box)
            .build();
        root.append(&scroll);

        let empty_btn = Button::with_label("Create your first theme");
        {
            let open_create = open_create.clone();
            empty_btn.connect_clicked(move |_| open_create());
        }
        root.append(&empty_btn);

        let restore_btn = Button::with_label("Restore Ubuntu defaults");
        restore_btn.add_css_class("destructive-action");
        root.append(&restore_btn);

        let view = Self {
            widget: root,
            inner: Rc::new(Inner {
                list_box,
                on_applied,
                toast,
                applying: Cell::new(false),
            }),
        };
        {
            let this = view.clone();
            restore_btn.connect_clicked(move |_| this.restore_dialog());
        }
        view
    }

    fn show_toast(&self, message: &str) {
        self.inner.toast.add_toast(Toast::new(message));
    }

    /// First line only: toasts say what failed in one sentence, no traces.
    fn failure_message(err: &anyhow::Error) -> String {
        let first = err
            .to_string()
            .lines()
            .next()
            .unwrap_or("unknown error")
            .to_string();
        format!("Apply failed: {first}")
    }

    pub fn reload(&self) {
        while let Some(child) = self.inner.list_box.first_child() {
            self.inner.list_box.remove(&child);
        }

        let ids = model::list_theme_ids().unwrap_or_default();
        let applied = model::applied_theme_id();

        if ids.is_empty() {
            let label = Label::new(Some(EMPTY_TEXT));
            label.set_wrap(true);
            self.inner.list_box.append(&label);
            return;
        }

        for id in ids {
            match model::load_theme(&id) {
                Ok(theme) => self.append_card(&id, &theme, applied.as_deref() == Some(&id)),
                Err(_) => self.append_broken_row(&id),
            }
        }
    }

    fn append_card(&self, id: &str, theme: &model::Theme, is_applied: bool) {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        row.add_css_class("card");
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        row.set_margin_start(6);
        row.set_margin_end(6);

        // Wallpaper thumbnail, decoded off the main thread.
        let picture = Picture::builder()
            .width_request(96)
            .height_request(96)
            .can_shrink(true)
            .build();
        let wallpaper = theme.wallpaper.clone();
        let thumb_picture = picture.clone();
        glib::spawn_future_local(async move {
            let thumb =
                gio::spawn_blocking(move || crate::thumbs::ensure_thumbnail(&wallpaper)).await;
            if let Ok(Ok(path)) = thumb {
                thumb_picture.set_filename(Some(path));
            }
        });
        row.append(&picture);

        let info = GtkBox::new(Orientation::Vertical, 4);
        info.set_hexpand(true);

        let mut badges = vec![theme.name.clone()];
        if is_applied {
            badges.push("Applied".to_string());
        }
        badges.push(if theme.glass.enabled {
            "Glass on".to_string()
        } else {
            "Glass off".to_string()
        });
        let name = Label::new(Some(&badges.join("  ·  ")));
        name.set_halign(gtk::Align::Start);
        name.add_css_class("title-4");
        info.append(&name);

        let swatches = GtkBox::new(Orientation::Horizontal, 4);
        for hex in [
            &theme.palette.background,
            &theme.palette.surface,
            &theme.palette.foreground,
            &theme.palette.accent,
            &theme.palette.on_accent,
        ] {
            swatches.append(&swatch(hex));
        }
        info.append(&swatches);
        row.append(&info);

        let actions = GtkBox::new(Orientation::Vertical, 4);

        let apply_row = GtkBox::new(Orientation::Horizontal, 6);
        let apply_btn = Button::with_label("Apply");
        let spinner = Spinner::new();
        spinner.set_visible(false);
        apply_row.append(&apply_btn);
        apply_row.append(&spinner);
        actions.append(&apply_row);

        {
            let this = self.clone();
            let theme = theme.clone();
            apply_btn.connect_clicked(move |btn| {
                if this.inner.applying.get() {
                    return;
                }
                this.inner.applying.set(true);
                btn.set_sensitive(false);
                spinner.set_visible(true);
                spinner.set_spinning(true);
                let theme = theme.clone();
                let this = this.clone();
                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(move || apply::apply_theme(&theme)).await;
                    this.inner.applying.set(false);
                    match result {
                        Ok(Ok(())) => {
                            this.show_toast(SUCCESS_TOAST);
                            (this.inner.on_applied)();
                        }
                        Ok(Err(e)) => this.show_toast(&Self::failure_message(&e)),
                        Err(e) => this.show_toast(&format!("Apply failed: {e:?}")),
                    }
                });
            });
        }

        let manage_row = GtkBox::new(Orientation::Horizontal, 6);
        let rename_btn = Button::with_label("Rename");
        let delete_btn = Button::with_label("Delete");
        manage_row.append(&rename_btn);
        manage_row.append(&delete_btn);
        actions.append(&manage_row);

        {
            let this = self.clone();
            let id = id.to_string();
            let name = theme.name.clone();
            rename_btn.connect_clicked(move |_| this.rename_dialog(&id, &name));
        }
        {
            let this = self.clone();
            let id = id.to_string();
            let name = theme.name.clone();
            delete_btn.connect_clicked(move |_| this.delete_dialog(&id, &name));
        }
        row.append(&actions);

        self.inner.list_box.append(&row);
    }

    /// Row for a theme directory that fails to parse: deletable, never applied.
    fn append_broken_row(&self, id: &str) {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.add_css_class("card");
        let label = Label::new(Some(&format!(
            "{id}  ·  Could not load — delete it or fix theme.toml"
        )));
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Start);
        row.append(&label);
        let delete_btn = Button::with_label("Delete");
        {
            let this = self.clone();
            let id = id.to_string();
            delete_btn.connect_clicked(move |_| this.delete_dialog(&id, &id));
        }
        row.append(&delete_btn);
        self.inner.list_box.append(&row);
    }

    fn rename_dialog(&self, id: &str, current: &str) {
        let win = gtk::Window::builder()
            .title("Rename theme")
            .modal(true)
            .default_width(320)
            .build();
        let content = GtkBox::new(Orientation::Vertical, 8);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        let entry = Entry::builder().text(current).build();
        let save_btn = Button::with_label("Rename");
        content.append(&entry);
        content.append(&save_btn);
        win.set_child(Some(&content));

        let this = self.clone();
        let id = id.to_string();
        let win_handle = win.clone();
        save_btn.connect_clicked(move |_| {
            let new_name = entry.text().to_string();
            match model::rename_theme(&id, &new_name) {
                Ok(_) => {
                    this.reload();
                    this.show_toast(&format!("Renamed to “{new_name}”"));
                    win_handle.close();
                }
                Err(e) => this.show_toast(&format!(
                    "Rename failed: {}",
                    e.to_string().lines().next().unwrap_or("invalid name")
                )),
            }
        });
        win.present();
    }

    fn delete_dialog(&self, id: &str, name: &str) {
        let alert = gtk::AlertDialog::builder()
            .message(format!("Delete “{name}”?"))
            .detail(
                "The desktop stays as it is until another theme is applied. Your wallpaper file is kept.",
            )
            .buttons(["Cancel", "Delete"])
            .cancel_button(0)
            .default_button(0)
            .build();

        let this = self.clone();
        let id = id.to_string();
        let name = name.to_string();
        alert.choose(
            None::<&gtk::Window>,
            None::<&gio::Cancellable>,
            move |response| {
                if response != Ok(1) {
                    return;
                }
                match model::delete_theme(&id) {
                    Ok(()) => {
                        this.reload();
                        this.show_toast(&format!("Deleted “{name}”"));
                    }
                    Err(e) => this.show_toast(&format!("Delete failed: {e}")),
                }
            },
        );
    }

    /// Restore Ubuntu defaults behind a confirmation dialog (spec §3.5).
    /// Runs on a worker; the toast reports baseline vs fallback outcome.
    fn restore_dialog(&self) {
        let alert = gtk::AlertDialog::builder()
            .message("Restore Ubuntu defaults?")
            .detail("Returns wallpaper, GTK theme, color-scheme, and shell theme to how they were before Atmosphere's first Apply. Saved themes are kept.")
            .buttons(["Cancel", "Restore"])
            .cancel_button(0)
            .default_button(0)
            .build();

        let this = self.clone();
        alert.choose(
            None::<&gtk::Window>,
            None::<&gio::Cancellable>,
            move |response| {
                if response != Ok(1) {
                    return;
                }
                let this = this.clone();
                glib::spawn_future_local(async move {
                    let result =
                        gio::spawn_blocking(baseline::restore_baseline).await;
                    match result {
                        Ok(Ok(baseline::RestoreOutcome::Baseline)) => {
                            this.show_toast(
                                "Restored your desktop to how it was before Atmosphere.",
                            );
                            this.reload();
                        }
                        Ok(Ok(baseline::RestoreOutcome::Fallback)) => {
                            this.show_toast(
                                "No baseline found — restored Ubuntu 26 defaults, which may not match your exact pre-Atmosphere look.",
                            );
                            this.reload();
                        }
                        Ok(Err(e)) => this.show_toast(&format!(
                            "Restore failed: {}",
                            e.to_string().lines().next().unwrap_or("unknown error")
                        )),
                        Err(e) => this.show_toast(&format!("Restore failed: {e:?}")),
                    }
                });
            },
        );
    }
}

fn swatch(hex: &str) -> GtkBox {
    let b = GtkBox::new(Orientation::Horizontal, 0);
    b.set_size_request(20, 20);
    super::css::tint(&b, hex);
    b
}
