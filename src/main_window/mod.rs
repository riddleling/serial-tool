mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::Widget, gtk::Container, gtk::Bin, gtk::Window, gtk::ApplicationWindow,
        @implements gtk::Buildable;          
}

impl MainWindow {
    pub fn new(app: &gtk::Application) -> Self {
        glib::Object::new(&[("application", app)]).expect("Failed to create MainWindow")
    }
}
