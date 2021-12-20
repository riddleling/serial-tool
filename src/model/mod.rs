use gtk::prelude::*;

pub fn create_port_model() -> gtk::ListStore {
    let types = [
        glib::Type::STRING, 
        glib::Type::STRING
    ];
    let model = gtk::ListStore::new(&types);
    model
}

pub fn add_port_item(model: &gtk::ListStore, port_name: String, port_type: String) {
    let port_title = format!("{} ({})", port_name, port_type);
    let values: [(u32, &dyn ToValue); 2] = [
        (0, &port_title),
        (1, &port_name)
    ];
    model.set(&model.append(), &values);
}

pub fn create_baud_rate_model() -> gtk::ListStore {
    let types = [
        glib::Type::STRING,
    ];
    let model = gtk::ListStore::new(&types);
    model
}


pub fn get_baud_rate_vec() -> Vec<String> {
    vec![
        String::from("300"),
        String::from("1200"),
        String::from("2400"),
        String::from("4800"),
        String::from("9600"),
        String::from("14400"),
        String::from("19200"),
        String::from("28800"),
        String::from("38400"),
        String::from("57600"),
        String::from("74880"),
        String::from("115200"),
        String::from("230400"),
        String::from("250000"),
        String::from("460800"),
        String::from("500000"),
        String::from("576000"),
        String::from("921600"),
        String::from("1000000"),
        String::from("2000000")
    ]
}

pub fn set_baud_rate_combo_box_items(model: &gtk::ListStore) {
    let baud_rate_vec = get_baud_rate_vec();

    for s in &baud_rate_vec {
        let values: [(u32, &dyn ToValue); 1] = [
            (0, s)
        ];
        model.set(&model.append(), &values); 
    }
}
