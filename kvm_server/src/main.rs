//! `kvm_server` 二进制：在本地监听 TCP 连接，接收来自客户端的
//! 键鼠事件并通过 `Enigo` 注入到当前操作系统中。

use anyhow::{Context, Result};
use clap::Parser;
use enigo::Mouse; // bring trait into scope
use enigo::{Button as EnigoBtn, Coordinate, Direction, Enigo, Settings};
use kvm_core::{now_millis, InputEvent, MouseButton};
use std::sync::mpsc;
use tokio::{io::AsyncReadExt, net::TcpListener};

/// 命令行参数定义
#[derive(Parser, Debug)]
#[command(about = "KVM Server (receiver): listen, receive and inject events")]
struct Args {
    /// 监听的本地地址，例如 `0.0.0.0:50051`
    #[arg(long, default_value = "0.0.0.0:50051")]
    listen: String,

    /// 是否输出调试日志
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 解析命令行参数并绑定监听端口
    let args = Args::parse();

    let listener = TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("bind {}", args.listen))?;
    eprintln!("🖥️  Server listening on {}", args.listen);

    // 2. 主循环：接受新的客户端连接
    loop {
        let (mut sock, peer) = listener.accept().await?;
        eprintln!("🔗 Client connected from {}", peer);

        // 使用 mpsc 通道在网络读取任务与注入线程之间传递事件
        let (tx, rx) = mpsc::channel::<(u64, u128, InputEvent)>();
        let debug = args.debug;

        // 注入线程：创建并持有 `Enigo` 实例（它不是 `Send`，
        // 因此必须放在普通线程中）
        std::thread::spawn(move || {
            let settings = Settings::default();
            let mut enigo = match Enigo::new(&settings) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("create Enigo failed: {e:?}");
                    return;
                }
            };
            for (seq, ts, event) in rx {
                // 计算一下端到端延迟，便于调试
                let now = now_millis();
                let latency = now.saturating_sub(ts);
                if debug {
                    eprintln!(
                        "🖥️  [SERVER] recv seq={} ts={} now={} latency={}ms event={:?}",
                        seq, ts, now, latency, event
                    );
                }
                if let Err(e) = handle_event(&mut enigo, event) {
                    eprintln!("inject error: {e}");
                }
            }
            eprintln!("🧵 injector thread exit for {}", peer);
        });

        // 异步任务：读取网络数据 → 解帧 → 解码成事件
        let tx_task = tx.clone();
        tokio::spawn(async move {
            let mut len_buf = [0u8; 4];
            let mut payload = vec![];

            loop {
                // 先读 4 字节长度
                if let Err(e) = sock.read_exact(&mut len_buf).await {
                    eprintln!("read len error: {e}");
                    break;
                }
                let len = u32::from_le_bytes(len_buf) as usize;

                // 再读指定长度的 payload
                payload.resize(len, 0);
                if let Err(e) = sock.read_exact(&mut payload).await {
                    eprintln!("read payload error: {e}");
                    break;
                }

                if let Some(env) = kvm_core::decode_env(&payload) {
                    // 把 (seq, ts, event) 交给注入线程
                    if tx_task.send((env.seq, env.ts_millis, env.event)).is_err() {
                        break; // 注入线程退出
                    }
                }
            }
            eprintln!("❌ Client disconnected {}", peer);
        });
    }
}

/// 根据事件类型，调用 `Enigo` 执行实际的鼠标操作
fn handle_event(enigo: &mut Enigo, ev: InputEvent) -> Result<()> {
    match ev {
        InputEvent::MouseMove { x, y } => {
            // 绝对坐标移动鼠标
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

/// 将我们自定义的 `MouseButton` 转成 `Enigo` 使用的按键枚举
fn map_button(btn: MouseButton) -> Option<EnigoBtn> {
    Some(match btn {
        MouseButton::Left => EnigoBtn::Left,
        MouseButton::Right => EnigoBtn::Right,
        MouseButton::Middle => EnigoBtn::Middle,
        MouseButton::Other(_) => return None,
    })
}
