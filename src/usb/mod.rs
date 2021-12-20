use rusb::{Context, Device, HotplugBuilder, UsbContext, Registration};
use std::{thread, time::Duration, collections::HashSet};
use std::sync::{Arc, Mutex};

use crate::my_tools::*;

struct HotPlugHandler;

impl<T: UsbContext> rusb::Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, device: Device<T>) {
        eprintln!("usb device arrived {:?}", device);
    }

    fn device_left(&mut self, device: Device<T>) {
        eprintln!("usb device left {:?}", device);
    }
}

impl Drop for HotPlugHandler {
    fn drop(&mut self) {
        eprintln!("HotPlugHandler dropped");
    }
}

pub fn hotplug_runloop_startup(event_tx: glib::Sender<String>, detect_pause_flag: Arc<Mutex<bool>>) {
    if rusb::has_hotplug() {
        let context = match Context::new() {
            Ok(c) => c,
            Err(e) => { 
                eprintln!("usb hotplug failed: {}", e);
                return;
            }
        };

        let _reg: Option<Registration<Context>> = Some(
            match HotplugBuilder::new()
                    .enumerate(true)
                    .register(&context, Box::new(HotPlugHandler {}))
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("usb hotplug failed: {}", e);
                    return;
                }
            }
        );

        loop {
            context.handle_events(None).unwrap();
            let sec = Duration::from_secs_f32(0.5);
            thread::sleep(sec);
            event_tx.send(String::from("[usb_hotplug](changed)")).expect("Could not send through channel");
        }
    } else {
        eprintln!("libusb hotplug api unsupported");
        let mut list: HashSet<String> = HashSet::new();
        let mut is_changed = false;

        loop {
            let pause_flag = *detect_pause_flag.lock().unwrap();
            if !pause_flag {
                let mut current_list: Vec<String> = Vec::new();
                for device in rusb::devices().unwrap().iter() {
                    let device_desc = device.device_descriptor().unwrap();
                    let s = format!("{:03}_{:03}_{:04x}_{:04x}",
                                        device.bus_number(),
                                        device.address(),
                                        device_desc.vendor_id(),
                                        device_desc.product_id());
                    current_list.push(s);
                }

                for item in &current_list {
                    if !list.contains(item) {
                        list.insert(item.clone());
                        is_changed = true;
                    }
                }

                let mut remove_list: Vec<String> = Vec::new();
                for item in &list {
                    if !current_list.contains(item) {
                        remove_list.push(item.clone());
                    }
                }
                for item in remove_list {
                    list.remove(&item);
                    is_changed = true;
                }

                if is_changed {
                    eprintln!("{} >>> usb changed", current_timestamp_string());
                    event_tx.send(String::from("[usb_hotplug](changed)")).expect("Could not send through channel");
                    is_changed = false;
                }
            }

            let sec = Duration::from_secs_f32(1.0);
            thread::sleep(sec);
        }
    }
}
