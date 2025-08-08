// client/src/main.rs
//! Server：监听 TCP，接收事件并在本机注入。
//! Server：监听 TCP，接收事件并在本机注入。

use anyhow::{Context, Result};
use clap::Parser;
use enigo::Mouse; // 让 move_mouse / button 可用
use enigo::{Button as EnigoBtn, Coordinate, Direction, Enigo, Settings};
use kvm_core::{decode, InputEvent, MouseButton};
use std::sync::mpsc;
use tokio::{io::AsyncReadExt, net::TcpListener};

#[derive(Parser, Debug)]
struct Args {
    /// 监听地址，例如 0.0.0.0:50051
    #[arg(long, default_value = "0.0.0.0:50051")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let listener = TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("bind {}", args.listen))?;
    println!("🖥️  Server listening on {}", args.listen);

    loop {
        let (mut sock, peer) = listener.accept().await?;
        println!("🔗 Client connected from {}", peer);

        // 用 mpsc 把 InputEvent 发送给注入线程（拥有 Enigo）
        let (tx, rx) = mpsc::channel::<InputEvent>();

        // 注入线程（阻塞线程）——持有 Enigo，循环处理事件
        std::thread::spawn(move || {
            let settings = Settings::default();
            let mut enigo = match Enigo::new(&settings) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("create Enigo failed: {e:?}");
                    return;
                }
            };
            for ev in rx {
                if let Err(e) = handle_event(&mut enigo, ev) {
                    eprintln!("inject error: {e}");
                }
            }
            println!("🧵 injector thread exit for {}", peer);
        });

        // 异步读取 socket，解码后把事件发给注入线程
        tokio::spawn(async move {
            let mut len_buf = [0u8; 4];
            let mut payload = vec![];

            loop {
                if let Err(e) = sock.read_exact(&mut len_buf).await {
                    eprintln!("read len error: {e}");
                    break;
                }
                let len = u32::from_le_bytes(len_buf) as usize;

                payload.resize(len, 0);
                if let Err(e) = sock.read_exact(&mut payload).await {
                    eprintln!("read payload error: {e}");
                    break;
                }
                if let Some(ev) = decode(&payload) {
                    if tx.send(ev).is_err() {
                        // 注入线程已退出
                        break;
                    }
                }
            }
            println!("❌ Client disconnected {}", peer);
        });
    }
}

fn handle_event(enigo: &mut Enigo, ev: InputEvent) -> Result<()> {
    match ev {
        InputEvent::MouseMove { x, y } => {
            enigo
                .move_mouse(x, y, Coordinate::Abs)
                .map_err(|e| anyhow::anyhow!("move_mouse: {e:?}"))?;
        }
        InputEvent::MouseButton { button, down } => {
            if let Some(btn) = map_button(button) {
                let dir = if down {
                    Direction::Press
                } else {
                    Direction::Release
                };
                enigo
                    .button(btn, dir)
                    .map_err(|e| anyhow::anyhow!("button: {e:?}"))?;
            }
        }
    }
    Ok(())
}

fn map_button(btn: MouseButton) -> Option<EnigoBtn> {
    Some(match btn {
        MouseButton::Left => EnigoBtn::Left,
        MouseButton::Right => EnigoBtn::Right,
        MouseButton::Middle => EnigoBtn::Middle,
        MouseButton::Other(_) => return None,
    })
}
