// #![windows_subsystem = "windows"]

pub mod main_window;
pub mod my_tools;
pub mod model;
pub mod port;
pub mod usb;

use main_window::MainWindow;
use gtk::prelude::*;
use gtk::gdk;


#[tokio::main]
async fn main() {
    let app = gtk::Application::builder()
        .application_id("site.riddleling.app.serial-tool")
        .build();

    app.connect_startup(|_| load_css());
    app.connect_activate(move |app| {
        build_ui(&app);
    });

    app.run();
}

fn build_ui(app: &gtk::Application) {
    let win = MainWindow::new(app);
    win.set_title("Serial Tool");
    win.set_border_width(0);
    win.set_window_position(gtk::WindowPosition::Center);
    win.show_all();
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_bytes!("style.css")).expect("Failed to load CSS");

    // Add the provider to the default screen
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing gtk css provider."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
