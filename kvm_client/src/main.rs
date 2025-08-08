use anyhow::{Context, Result};
use clap::Parser;
use kvm_core::{encode_env, now_millis, EventEnvelope, InputEvent, MouseButton};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[derive(Parser, Debug)]
#[command(about = "KVM Client (sender): capture local input and send to server")]
struct Args {
    /// 连接到 Server 的地址，例如 192.168.1.23:50051
    #[arg(long)]
    connect: String,

    /// 开启调试日志
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    eprintln!("🔌 Client connecting to {} ...", args.connect);

    let stream = TcpStream::connect(&args.connect)
        .await
        .with_context(|| format!("connect to {}", args.connect))?;
    eprintln!("✅ Client connected.");

    // 供 rdev 回调线程使用
    let stream = Arc::new(Mutex::new(stream));
    let seq = Arc::new(AtomicU64::new(1));
    let debug = Arc::new(args.debug);

    let stream_clone = stream.clone();
    let seq_clone = seq.clone();
    let debug_clone = debug.clone();

    // 捕获键鼠事件（同步回调）
    let callback = move |event: rdev::Event| {
        if let Some(ev) = map_event(event) {
            let env = EventEnvelope {
                ts_millis: now_millis(),
                seq: seq_clone.fetch_add(1, Ordering::Relaxed),
                event: ev,
            };

            // 帧：u32(LE) 长度 + payload
            let mut payload = encode_env(&env);
            let len = payload.len() as u32;
            let mut framed = len.to_le_bytes().to_vec();
            framed.append(&mut payload);

            if let Ok(mut guard) = stream_clone.lock() {
                // 用 block_on 在回调里同步写 tokio stream
                if let Err(e) = futures::executor::block_on(guard.write_all(&framed)) {
                    eprintln!("send error: {e}");
                    return;
                }
                let _ = futures::executor::block_on(guard.flush());
                if *debug_clone {
                    eprintln!(
                        "[CLIENT] sent seq={} ts={} len={}B event={:?}",
                        env.seq, env.ts_millis, len, env.event
                    );
                }
            }
        }
    };

    eprintln!("🖱️  Client capturing mouse/keyboard (rdev)...");
    if let Err(e) = rdev::listen(callback) {
        eprintln!("client capture error: {:?}", e);
    }
    Ok(())
}

// 将 rdev::Event 映射成我们自己的 InputEvent
fn map_event(ev: rdev::Event) -> Option<InputEvent> {
    use rdev::EventType;
    match ev.event_type {
        EventType::MouseMove { x, y } => Some(InputEvent::MouseMove {
            x: x as i32,
            y: y as i32,
        }),
        EventType::ButtonPress(btn) => map_button(btn).map(|b| InputEvent::MouseButton {
            button: b,
            down: true,
        }),
        EventType::ButtonRelease(btn) => map_button(btn).map(|b| InputEvent::MouseButton {
            button: b,
            down: false,
        }),
        _ => None,
    }
}

fn map_button(btn: rdev::Button) -> Option<MouseButton> {
    use rdev::Button::*;
    Some(match btn {
        Left => MouseButton::Left,
        Right => MouseButton::Right,
        Middle => MouseButton::Middle,
        Unknown(code) => MouseButton::Other(code as u8),
    })
}
