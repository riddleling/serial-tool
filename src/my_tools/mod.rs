use gtk::prelude::*;
use chrono::prelude::*;

pub async fn show_alert_dialog<W: IsA<gtk::Window>>(window: &W, message: String) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .buttons(gtk::ButtonsType::Ok)
        .title("Alert")
        .text("Message")
        .secondary_text(&message)
        .window_position(gtk::WindowPosition::CenterOnParent)
        .build();
    dialog.run_future().await;
    dialog.close();
}

pub async fn show_question_dialog<W: IsA<gtk::Window>>(window: &W, message: String) -> gtk::ResponseType {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .buttons(gtk::ButtonsType::OkCancel)
        .title("Question")
        .text("Message")
        .secondary_text(&message)
        .window_position(gtk::WindowPosition::CenterOnParent)
        .build();
    let answer = dialog.run_future().await;
    dialog.close();
    answer
}

pub fn current_timestamp_string() -> String {
    let local: DateTime<Local> = Local::now();
    format!("{:02}:{:02}:{:02}.{}", 
            local.hour(),
            local.minute(),
            local.second(),
            cut_nanosecond(local.nanosecond()))
}

fn cut_nanosecond(nanosecond: u32) -> String {
    let s = nanosecond.to_string();
    String::from(&s[..3])
}
