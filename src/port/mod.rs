use std::{io, str};
use std::sync::{Arc, Mutex};

use tokio_util::codec::{Decoder, Encoder};
use bytes::{BufMut, BytesMut};

use futures::channel::mpsc::UnboundedReceiver;
use futures_util::{future, pin_mut, StreamExt, SinkExt};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        eprintln!("In writer {:?}", &item);
        dst.reserve(item.len() + 1);
        dst.put(item.as_bytes());
        dst.put_u8(b'\n');
        Ok(())
    }
}

pub async fn open_port_async(
    port_name: String,
    baud_rate: u32,
    write_rx: UnboundedReceiver<String>,
    read_tx: glib::Sender<String>,
    state_tx: glib::Sender<String>,
    port_close_flag: Arc<Mutex<bool>>)
{
    let mut port = match tokio_serial::new(port_name, baud_rate).open_native_async() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            state_tx.send(String::from("[open_port](failed)")).expect("Could not send through channel");
            return;
        }
    };

    #[cfg(unix)]
    port.set_exclusive(false).expect("Unable to set serial port exclusive to false");

    #[cfg(windows)]
    if let Err(e) = port.write_data_terminal_ready(true) {
        eprintln!("Error: {}", e);
    }

    state_tx.send(String::from("[open_port](ok)")).expect("Could not send through channel");

    let (mut write, mut read) = LineCodec.framed(port).split();

    let mut write_rx_mut = write_rx;
    let write_to_port = async {
        while let Some(s) = write_rx_mut.next().await {
            let close_flag = *port_close_flag.lock().unwrap();
            eprintln!("(thread) write_to_port: port_close_flag => {}", close_flag);
            if close_flag {
                break;
            } else {
                let _ = write.send(s).await;
            }
        }
    };

    let read_from_port = async {
        while let Some(line_result) = read.next().await {
            match line_result {
                Ok(line) => read_tx.send(line).expect("Could not send through channel"),
                Err(e) => eprintln!("Failed to read line: {}", e)
            }
        }
        eprintln!("(thread) read_from_port: stop...");
    };

    pin_mut!(write_to_port, read_from_port);
    future::select(write_to_port, read_from_port).await;

    eprintln!("closing port...");
    state_tx.send(String::from("[close_port]()")).expect("Could not send through channel");
}
