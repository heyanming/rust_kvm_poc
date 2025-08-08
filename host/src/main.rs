// host/src/main.rs
//! 捕获本机鼠标事件，通过 TCP 发送到 client。（鼠标移动 + 左键）
use anyhow::{Context, Result};
use clap::Parser;
use kvm_core::{InputEvent, MouseButton, encode};
use tokio::{net::TcpStream, io::AsyncWriteExt};
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
struct Args {
    /// 连接到 client 的地址，例如 192.168.1.23:50051
    #[arg(long)]
    connect: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("🔌 connecting to {} ...", args.connect);
    let stream = TcpStream::connect(&args.connect).await
        .with_context(|| format!("connect to {}", args.connect))?;
    println!("✅ connected.");

    let stream = Arc::new(Mutex::new(stream));
    let stream_clone = stream.clone();
    let callback = move |event: rdev::Event| {
        if let Some(ev) = map_event(event) {
            let mut buf = encode(&ev);
            let len = buf.len() as u32;
            let mut framed = len.to_le_bytes().to_vec();
            framed.append(&mut buf);

            if let Ok(mut guard) = stream_clone.lock() {
                if let Err(e) = futures::executor::block_on(guard.write_all(&framed)) {
                    eprintln!("send error: {e}");
                }
                if let Err(e) = futures::executor::block_on(guard.flush()) {
                    eprintln!("flush error: {e}");
                }
            }
        }
    };

    println!("🖱  start listening mouse (rdev)...");
    if let Err(e) = rdev::listen(callback) { eprintln!("listen error: {:?}", e); }
    Ok(())
}

fn map_event(ev: rdev::Event) -> Option<InputEvent> {
    use rdev::{EventType, Button};
    match ev.event_type {
        EventType::MouseMove { x, y } => Some(InputEvent::MouseMove { x: x as i32, y: y as i32 }),
        EventType::ButtonPress(btn) => map_button(btn).map(|b| InputEvent::MouseButton { button: b, down: true }),
        EventType::ButtonRelease(btn) => map_button(btn).map(|b| InputEvent::MouseButton { button: b, down: false }),
        _ => None,
    }
}
fn map_button(btn: rdev::Button) -> Option<MouseButton> {
    use rdev::Button::*;
    Some(match btn {
        Left => MouseButton::Left, Right => MouseButton::Right, Middle => MouseButton::Middle,
        Unknown(code) => MouseButton::Other(code as u8), _ => return None,
    })
}
