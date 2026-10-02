use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow};

use crate::theme::apply;
use crate::theme::model;

pub struct ThemesView {
    pub widget: GtkBox,
    list_box: GtkBox,
    on_applied: Rc<dyn Fn()>,
}

impl ThemesView {
    pub fn new(open_create: Rc<dyn Fn()>, on_applied: Rc<dyn Fn()>) -> Self {
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

        Self {
            widget: root,
            list_box,
            on_applied,
        }
    }

    pub fn reload(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let ids = model::list_theme_ids().unwrap_or_default();
        let applied = model::applied_theme_id();

        if ids.is_empty() {
            let label = Label::new(Some("No saved themes yet."));
            self.list_box.append(&label);
            return;
        }

        for id in ids {
            let theme = model::load_theme(&id).ok();
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.add_css_class("card");

            let name = theme
                .as_ref()
                .map(|t| t.name.clone())
                .unwrap_or_else(|| id.clone());
            let glass = theme
                .as_ref()
                .map(|t| {
                    if t.glass.enabled {
                        "Glass on"
                    } else {
                        "Glass off"
                    }
                })
                .unwrap_or("");

            let label_text = if applied.as_deref() == Some(&id) {
                format!("{name}  ·  Applied  ·  {glass}")
            } else {
                format!("{name}  ·  {glass}")
            };
            let label = Label::new(Some(&label_text));
            label.set_hexpand(true);
            label.set_halign(gtk::Align::Start);
            row.append(&label);

            if let Some(theme) = theme {
                let apply_btn = Button::with_label("Apply");
                let on_applied = self.on_applied.clone();
                apply_btn.connect_clicked(move |_| {
                    let theme = theme.clone();
                    glib::spawn_future_local(async move {
                        let result = gio::spawn_blocking(move || apply::apply_theme(&theme)).await;
                        match result {
                            Ok(Ok(())) => on_applied(),
                            Ok(Err(e)) => glib::g_warning!("atmosphere", "Apply failed: {}", e),
                            Err(e) => glib::g_warning!("atmosphere", "Apply failed: {}", e),
                        }
                    });
                });
                row.append(&apply_btn);
            }
            self.list_box.append(&row);
        }
    }
}
