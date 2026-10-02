use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Orientation, Stack, StackSwitcher};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, Banner, HeaderBar, ToastOverlay};

use crate::paths::ensure_data_dirs;
use crate::theme::shell_check;
use crate::ui::create_view::CreateView;
use crate::ui::themes_view::ThemesView;

pub fn build_window(app: &libadwaita::Application) -> ApplicationWindow {
    ensure_data_dirs().ok();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Atmosphere")
        .default_width(720)
        .default_height(560)
        .build();

    let toast_overlay = ToastOverlay::new();
    window.set_content(Some(&toast_overlay));

    let root = GtkBox::new(Orientation::Vertical, 0);

    let header = HeaderBar::new();
    let stack = Stack::new();
    let switcher = StackSwitcher::new();
    switcher.set_stack(Some(&stack));
    header.set_title_widget(Some(&switcher));
    root.append(&header);

    stack.set_vexpand(true);

    let themes_holder = Rc::new(RefCell::new(None::<ThemesView>));
    let create_holder = Rc::new(RefCell::new(None::<CreateView>));

    let open_create = Rc::new({
        let stack = stack.clone();
        move || stack.set_visible_child_name("create")
    });

    let reload_themes = Rc::new({
        let themes_holder = themes_holder.clone();
        move || {
            if let Some(view) = themes_holder.borrow().as_ref() {
                view.reload();
            }
        }
    });

    let themes_view = ThemesView::new(open_create.clone(), reload_themes.clone());
    themes_view.reload();
    let themes_widget = themes_view.widget.clone();
    *themes_holder.borrow_mut() = Some(themes_view);

    let create_view = CreateView::new();
    let create_widget = create_view.widget.clone();
    *create_holder.borrow_mut() = Some(create_view);

    stack.add_named(&themes_widget, Some("themes"));
    stack.add_named(&create_widget, Some("create"));
    stack.set_visible_child_name("themes");

    stack.connect_visible_child_name_notify({
        let reload_themes = reload_themes.clone();
        let create_holder = create_holder.clone();
        move |s| {
            if s.visible_child_name().as_deref() == Some("themes") {
                reload_themes();
            }
            if s.visible_child_name().as_deref() == Some("create") {
                if let Some(view) = create_holder.borrow().as_ref() {
                    view.reload_wallpapers();
                }
            }
        }
    });

    if !shell_check::user_themes_enabled() {
        let banner = Banner::new(
            "To theme the top bar and notifications, enable the “User Themes” extension (Extensions app), then log out and back in. Wallpaper and app colors still work.",
        );
        banner.set_revealed(true);
        root.prepend(&banner);
    }

    root.append(&stack);
    toast_overlay.set_child(Some(&root));

    window
}
