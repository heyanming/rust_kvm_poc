// client/src/main.rs
//! 监听 TCP，接收事件并在本机注入。（鼠标移动 + 左键）
use anyhow::{Context, Result};
use clap::Parser;
use kvm_core::{InputEvent, MouseButton, decode};
use tokio::{net::TcpListener, io::AsyncReadExt};
use enigo::{Enigo, MouseControllable, MouseButton as EnigoBtn};

#[derive(Parser, Debug)]
struct Args {
    /// 监听地址，例如 0.0.0.0:50051
    #[arg(long, default_value = "0.0.0.0:50051")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let listener = TcpListener::bind(&args.listen).await
        .with_context(|| format!("bind {}", args.listen))?;
    println!("👂 listening on {}", args.listen);

    loop {
        let (mut sock, peer) = listener.accept().await?;
        println!("🔗 connected from {}", peer);
        let mut enigo = Enigo::new();

        tokio::spawn(async move {
            let mut len_buf = [0u8; 4];
            let mut payload = vec![];
            loop {
                if let Err(e) = sock.read_exact(&mut len_buf).await {
                    eprintln!("read len error: {e}"); break;
                }
                let len = u32::from_le_bytes(len_buf) as usize;
                payload.resize(len, 0);
                if let Err(e) = sock.read_exact(&mut payload).await {
                    eprintln!("read payload error: {e}"); break;
                }
                if let Some(ev) = decode(&payload) {
                    handle_event(&mut enigo, ev);
                }
            }
            println!("❌ disconnected {}", peer);
        });
    }
}
fn handle_event(enigo: &mut Enigo, ev: InputEvent) {
    match ev {
        InputEvent::MouseMove { x, y } => enigo.mouse_move_to(x, y),
        InputEvent::MouseButton { button, down } => {
            if let Some(btn) = map_button(button) {
                if down { enigo.mouse_down(btn) } else { enigo.mouse_up(btn) }
            }
        }
    }
}
fn map_button(btn: MouseButton) -> Option<EnigoBtn> {
    Some(match btn {
        MouseButton::Left => EnigoBtn::Left,
        MouseButton::Right => EnigoBtn::Right,
        MouseButton::Middle => EnigoBtn::Middle,
        MouseButton::Other(_) => return None,
    })
}
