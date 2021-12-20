use glib::clone;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};
use std::thread;
use once_cell::unsync::OnceCell;
use futures::channel::mpsc::unbounded;

use tokio_serial::available_ports;
use serialport::SerialPortType::*;
use regex::Regex;

use crate::model;
use crate::port::open_port_async;
use crate::my_tools::*;
use crate::usb::hotplug_runloop_startup;

enum PortState {
    Opening,
    Opened,
    Closed,
}

#[derive(Debug, Default)]
pub struct MainWindow {
    port_model: OnceCell<gtk::ListStore>,
    port_combo_box: OnceCell<gtk::ComboBoxText>,
    port_refresh_button: OnceCell<gtk::Button>,
    selected_port_name: RefCell<String>,

    write_entry: OnceCell<gtk::Entry>,
    write_button: OnceCell<gtk::Button>,
    write_button_handler_id: RefCell<Option<glib::SignalHandlerId>>,

    read_text_view: OnceCell<gtk::TextView>,
    scrolled_window: OnceCell<gtk::ScrolledWindow>,

    timestamp_check_button: OnceCell<gtk::CheckButton>,
    auto_scroll_check_button: OnceCell<gtk::CheckButton>,

    baud_rate_combo_box: OnceCell<gtk::ComboBoxText>,
    open_close_button: OnceCell<gtk::Button>,

    port_close_flag: Arc<Mutex<bool>>,
    is_port_opened: Cell<bool>,

    usb_detect_pause_flag: Arc<Mutex<bool>>,  // use only when hotplug is not supported
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindow {
    const NAME: &'static str = "MainWindow";
    type Type = super::MainWindow;
    type ParentType = gtk::ApplicationWindow;
}

impl ObjectImpl for MainWindow {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        // main_box
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .homogeneous(false)
            .spacing(0)
            .build();

        // box1
        let box1 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin(5)
            .spacing(5)
            .build();

        let port_label = gtk::Label::builder()
            .label("Serial Port:")
            .margin_start(5)
            .margin_end(5)
            .build();

        let port_model = model::create_port_model();
        let port_combo_box= gtk::ComboBoxText::builder()
            .model(&port_model)
            .build();

        port_combo_box.connect_changed(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_port_combo_box_changed();
        }));
        
        let port_refresh_button = gtk::Button::builder()
            .label("Refresh")
            .margin_start(5)
            .margin_end(5)
            .build();

        port_refresh_button.connect_clicked(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_port_refresh_button_clicked();
        }));

        box1.pack_start(&port_label, false, false, 0);
        box1.pack_start(&port_combo_box, true, true, 0);
        box1.pack_start(&port_refresh_button, false, false, 0);

        
        // box2
        let box2 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin_start(5)
            .margin_end(5)
            .margin_bottom(5)
            .spacing(5)
            .build();

        let write_entry = gtk::Entry::builder()
            .margin_start(5)
            .sensitive(false)
            .build();
        let write_button = gtk::Button::builder()
            .label("Send")
            .margin_start(5)
            .margin_end(5)
            .sensitive(false)
            .build();

        // write_entry press `Enter` key:
        write_entry.connect_activate(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_write_entry_activate();
        }));

        box2.pack_start(&write_entry, true, true, 0);
        box2.pack_start(&write_button, false, false, 0);


        // read_text_view
        let read_text_view = gtk::TextView::builder()
            .editable(false)
            .build();
            
        read_text_view.set_widget_name("read_text_view");
        read_text_view.connect_size_allocate(clone!(@weak obj => move |_,_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_read_text_view_size_allocate();
        }));

        // scrolled_window
        let scrolled_window = gtk::ScrolledWindow::builder()
            .child(&read_text_view)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .margin_start(10)
            .margin_end(10)
            .build();


        // box3
        let box3 = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(false)
            .margin(5)
            .spacing(5)
            .build();

        let clear_output_button = gtk::Button::builder()
            .label("Clear output")
            .margin_start(5)
            .build();

        clear_output_button.connect_clicked(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_clear_output_button_clicked();
        }));

        let auto_scroll_check_button = gtk::CheckButton::builder()
            .label("Autoscroll")
            .margin_start(5)
            .active(true)
            .build();

        let timestamp_check_button = gtk::CheckButton::builder()
            .label("Timestamp")
            .margin_start(5)
            .active(true)
            .build();

        let baud_rate_label = gtk::Label::builder()
            .label("Baud Rate:")
            .margin_start(55)
            .margin_end(5)
            .build();

        let baud_rate_model = model::create_baud_rate_model();
        let baud_rate_combo_box = gtk::ComboBoxText::builder()
            .model(&baud_rate_model)
            .build();

        model::set_baud_rate_combo_box_items(&baud_rate_model);
        if let Some(index) = model::get_baud_rate_vec().iter().position(|s| s == "115200") {
            baud_rate_combo_box.set_active(Some(index as u32));
        }

        let open_close_button = gtk::Button::builder()
            .label("Open Port")
            .margin_start(5)
            .margin_end(5)
            .build();

        open_close_button.connect_clicked(clone!(@weak obj => move |_| {
            let priv_ = MainWindow::from_instance(&obj);
            priv_.on_open_close_button_clicked();
        }));

        box3.pack_start(&clear_output_button, false, false, 0);
        box3.pack_start(&auto_scroll_check_button, false, false, 0);
        box3.pack_start(&timestamp_check_button, false, false, 0);
        box3.pack_end(&open_close_button, false, false, 0);
        box3.pack_end(&baud_rate_combo_box, false, false, 0);
        box3.pack_end(&baud_rate_label, false, false, 0);
        

        // add components to main_box
        main_box.pack_start(&box1, false, false, 0);
        main_box.pack_start(&box2, false, false, 0);
        main_box.pack_start(&scrolled_window, true, true, 0);
        main_box.pack_start(&box3, false, false, 0);
        
        // set window
        obj.add(&main_box);
        obj.set_default_size(750, 450);


        self.port_model.set(port_model).expect("Failed to initialize window state: port_model");
        self.port_combo_box.set(port_combo_box).expect("Failed to initialize window state: port_combo_box");
        self.port_refresh_button.set(port_refresh_button).expect("Failed to initialize window state: port_refresh_button");

        self.write_entry.set(write_entry).expect("Failed to initialize window state: write_entry");
        self.write_button.set(write_button).expect("Failed to initialize window state: write_button");

        self.read_text_view.set(read_text_view).expect("Failed to initialize window state: read_text_view");
        self.scrolled_window.set(scrolled_window).expect("Failed to initialize window state: scrolled_window");
        
        self.timestamp_check_button.set(timestamp_check_button).expect("Failed to initialize window state: timestamp_check_button");
        self.auto_scroll_check_button.set(auto_scroll_check_button).expect("Failed to initialize window state: auto_scroll_check_button");
        
        self.baud_rate_combo_box.set(baud_rate_combo_box).expect("Failed to initialize window state: baud_rate_combo_box");
        self.open_close_button.set(open_close_button).expect("Failed to initialize window state: open_port_button");
    

        // click port_refresh_button
        self.port_refresh_button.get().unwrap().clicked();

        // usb hotplug detection
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        rx.attach(
            None,  
            clone!(@weak obj => @default-return glib::Continue(false),
            move |msg| {
                let priv_ = MainWindow::from_instance(&obj);
                priv_.on_state_changed(msg);
                glib::Continue(true)
            }
        ));

        let detect_pause_flag = self.usb_detect_pause_flag.clone();
        thread::spawn(move || {
            hotplug_runloop_startup(tx, detect_pause_flag);
        });
    }
}

impl MainWindow {
    fn on_port_combo_box_changed(&self) {
        let port_name = self.get_selected_port_name();
        if port_name != "" && (port_name != *self.selected_port_name.borrow()) {
            eprintln!("port: {}", port_name);
            self.selected_port_name.replace(port_name);
        }
    }

    fn set_usb_detect_pause_flag(&self, flag: bool) {
        let mut detect_pause_flag = self.usb_detect_pause_flag.lock().unwrap();
        *detect_pause_flag = flag;
    }

    fn set_port_close_flag(&self, flag: bool) {
        let mut port_close_flag = self.port_close_flag.lock().unwrap();
        *port_close_flag = flag;
    }

    fn on_write_entry_activate(&self) {
        self.write_button.get().unwrap().clicked();
    }

    fn on_read_text_view_size_allocate(&self) {
        let is_auto_scroll = self.auto_scroll_check_button.get().unwrap().is_active();
        if is_auto_scroll {        
            let vadjustment = self.scrolled_window.get().unwrap().vadjustment();
            vadjustment.set_value(vadjustment.upper() - vadjustment.page_size());
        }
    }

    fn on_clear_output_button_clicked(&self) {
        let text_view = self.read_text_view.get().unwrap();
        if let Some(buffer) = text_view.buffer() {
            buffer.set_text("");
        }
    }

    fn on_port_refresh_button_clicked(&self) {
        let model = self.port_model.get().unwrap();
        model.clear();

        match available_ports() {
            Ok(ports) => {
                let mut selected_index = None;
                let mut i: u32 = 0;
                for p in ports {
                    let port_name = p.port_name;
                    let port_type = match p.port_type {
                        UsbPort(_) => "USB".to_string(),
                        PciPort => "PCI".to_string(),
                        BluetoothPort => "Bluetooth".to_string(),
                        Unknown => "Unknown".to_string()
                    };
                    eprintln!("- {} ({})", port_name, port_type);
                    if *self.selected_port_name.borrow() == port_name {
                        selected_index = Some(i);
                    }
                    model::add_port_item(&model, port_name, port_type);
                    i += 1;
                }
                eprintln!("----------");

                if let Some(index) = selected_index {
                    self.port_combo_box.get().unwrap().set_active(Some(index));
                }
            }
            Err(e) => eprintln!("No ports found: {}", e)
        }
    }

    fn question_close_port(&self) {
        let obj = MainWindow::instance(self);
        glib::MainContext::default().spawn_local(clone!(@weak obj => async move {
            let priv_ = MainWindow::from_instance(&obj);
            let answer = show_question_dialog(&obj, String::from("Close this port?")).await;
            if let gtk::ResponseType::Ok = answer {
                priv_.set_port_close_flag(true);
                priv_.write_button.get().unwrap().clicked();
            }
        })); 
    }

    fn on_open_close_button_clicked(&self) {
        if self.is_port_opened.get() {
            self.question_close_port();
            return;
        }

        // opening a port
        let port_name = self.get_selected_port_name();
        let baud_rate = self.get_selected_baud_rate();
        eprintln!("port_name: {} / baud_rate: {}", port_name, baud_rate);

        if port_name != "" && baud_rate != "" {
            if let Ok(baud_rate) = baud_rate.parse::<u32>() {
                self.open_port(port_name, baud_rate);
                return;
            }
        }

        let dialog_text: String;
        if port_name == "" {
            dialog_text = String::from("Please choose a port!");
        } else if baud_rate == "" {
            dialog_text = String::from("Please choose a baud rate!");
        } else {
            dialog_text = String::from("Failed to open the port!");
        }

        // display a dialog
        let obj = MainWindow::instance(self);
        glib::MainContext::default().spawn_local(clone!(@weak obj => async move {
            show_alert_dialog(&obj, dialog_text).await;
        }));
    }

    fn get_selected_port_name(&self) -> String {
        let combo = self.port_combo_box.get().unwrap();
        if let Some(tree_iter) = combo.active_iter() {
            if let Some(model) = combo.model() {
                if let Ok(port_name) = model.value(&tree_iter, 1).get::<String>() {
                    return port_name;
                }
            }
        }
        String::from("")
    }

    fn get_selected_baud_rate(&self) -> String {
        let combo = self.baud_rate_combo_box.get().unwrap();
        if let Some(tree_iter) = combo.active_iter() {
            if let Some(model) = combo.model() {
                if let Ok(baud_rate) = model.value(&tree_iter, 0).get::<String>() {
                    return baud_rate; 
                }   
            }
        }
        String::from("")
    }

    fn open_port(&self, port_name: String, baud_rate: u32) {
        let obj = MainWindow::instance(self);

        // receive port string, display to text_view
        let (read_tx, read_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        read_rx.attach(
            None,
            clone!(@weak obj => @default-return Continue(false),
                move |line| {
                    let priv_ = MainWindow::from_instance(&obj);
                    priv_.handle_output(line);
                    glib::Continue(true)
                }
            )
        );

        // send string to port
        let (write_tx, write_rx) = unbounded();
        let write_entry = self.write_entry.get().unwrap();
        let write_button = self.write_button.get().unwrap();
        let write_button_handler_id = write_button.connect_clicked(
            clone!(@weak write_entry, @strong write_tx => move |_| {
                let line = write_entry.text().to_string();
                write_tx.unbounded_send(line).expect("Could not send through channel");
                write_entry.set_text("");
            })
        );
        self.write_button_handler_id.replace(Some(write_button_handler_id));

        let (state_tx, state_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        
        state_rx.attach(
            None,
            clone!(@weak obj => @default-return Continue(false),
                move |msg| {
                    let priv_ = MainWindow::from_instance(&obj);
                    priv_.on_state_changed(msg);
                    glib::Continue(true)
                }
            )
        );

        self.set_open_close_button(PortState::Opening);
        self.port_widgets_enable(false);
        self.set_port_close_flag(false);

        let port_close_flag = self.port_close_flag.clone();
        tokio::task::spawn(async move {
            open_port_async(port_name, baud_rate, write_rx, read_tx, state_tx, port_close_flag).await;
        });
    }

    fn handle_output(&self, line: String) {
        let text_view = self.read_text_view.get().unwrap();
        if let Some(buffer) = text_view.buffer() {
            let mut end_iter = buffer.end_iter();
            let is_show_timestamp = self.timestamp_check_button.get().unwrap().is_active();
            let s: String;
            if is_show_timestamp {
                s = format!("{} -> {}", current_timestamp_string(), line);
            } else {
                s = line;
            }
            buffer.insert(&mut end_iter, &s);
        }
    }

    fn port_widgets_enable(&self, enable: bool) {
        self.port_combo_box.get().unwrap().set_sensitive(enable);
        self.port_refresh_button.get().unwrap().set_sensitive(enable);
        self.baud_rate_combo_box.get().unwrap().set_sensitive(enable);
    }

    fn write_widgets_enable(&self, enable: bool) {
        self.write_entry.get().unwrap().set_sensitive(enable);
        self.write_button.get().unwrap().set_sensitive(enable);
    }

    fn set_open_close_button(&self, state: PortState) {
        match state {
            PortState::Opening => {
                self.open_close_button.get().unwrap().set_sensitive(false);
            }
            PortState::Opened => {
                self.open_close_button.get().unwrap().set_label("Close Port");
                self.open_close_button.get().unwrap().set_sensitive(true);
            }
            PortState::Closed => {
                self.open_close_button.get().unwrap().set_label("Open Port");
                self.open_close_button.get().unwrap().set_sensitive(true);
            }
        }
    }

    fn get_state_event_and_value(&self, msg: String) -> (String, String) {
        //
        // msg format: [event_name](event_value)
        //
        let re = Regex::new(r"\[(?P<event>.+?)\]\((?P<value>.*?)\)").unwrap();
        match re.captures(&msg) {
            Some(caps) => {
                let event = &caps["event"];
                let value = &caps["value"];
                (event.to_string(), value.to_string())
            }
            None => ("".to_string(), "".to_string())
        }
    }

    fn on_state_changed(&self, state_msg: String) {
        let (event, value) = self.get_state_event_and_value(state_msg);
        if event == "open_port" && value == "ok" {
            self.is_port_opened.set(true);
            self.set_open_close_button(PortState::Opened);
            self.write_widgets_enable(true);
            self.set_usb_detect_pause_flag(true);
        } else if (event == "open_port" && value == "failed") || event == "close_port" {
            self.is_port_opened.set(false);
            self.set_open_close_button(PortState::Closed);
            let dialog_text: String;
            if event == "open_port" {
                dialog_text = String::from("Failed to open the port!");
            } else {
                dialog_text = String::from("Port closed.");
            }
            self.handle_close(dialog_text);
            self.set_usb_detect_pause_flag(false);
        } else if event == "usb_hotplug" && value == "changed" {
            if !self.is_port_opened.get() {
                self.port_refresh_button.get().unwrap().clicked();
            }
        }
    }

    fn handle_close(&self, dialog_text: String) {
        if let Some(id) = self.write_button_handler_id.borrow_mut().take() {
            self.write_button.get().unwrap().disconnect(id)
        }
        self.write_widgets_enable(false);
        self.port_widgets_enable(true);
        self.open_close_button.get().unwrap().set_label("Open Port");
        self.port_refresh_button.get().unwrap().clicked();

        // display a dialog
        let obj = MainWindow::instance(self);
        glib::MainContext::default().spawn_local(clone!(@weak obj => async move {
            show_alert_dialog(&obj, dialog_text).await;
        }));
    }    
}

impl WidgetImpl for MainWindow {}
impl ContainerImpl for MainWindow {}
impl BinImpl for MainWindow {}
impl WindowImpl for MainWindow {}
impl ApplicationWindowImpl for MainWindow {}
