mod paths;
mod theme;
mod thumbs;
mod ui;

use gtk::glib;
use gtk::prelude::*;

const APP_ID: &str = "com.atmosphere.App";

fn main() -> glib::ExitCode {
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();
    app.connect_activate(|app| {
        let window = ui::window::build_window(app);
        window.present();
    });
    app.run()
}
