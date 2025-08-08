//! `kvm_client` 二进制：负责在本地捕获键鼠输入并通过 TCP 发送到远端。
//! 整个流程大致为：解析命令行参数 → 建立 TCP 连接 → 使用 `rdev`
//! 监听事件 → 封装并发送到服务端。

use anyhow::{Context, Result};
use clap::Parser;
use kvm_core::{encode_env, now_millis, EventEnvelope, InputEvent, MouseButton};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::{io::AsyncWriteExt, net::TcpStream};

/// 命令行参数定义
#[derive(Parser, Debug)]
#[command(about = "KVM Client (sender): capture local input and send to server")]
struct Args {
    /// 需要连接的服务器地址，例如 `192.168.1.23:50051`
    #[arg(long)]
    connect: String,

    /// 是否输出调试日志（默认关闭）
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 解析命令行参数
    let args = Args::parse();
    eprintln!("🔌 Client connecting to {} ...", args.connect);

    // 2. 建立 TCP 连接，准备发送数据
    let stream = TcpStream::connect(&args.connect)
        .await
        .with_context(|| format!("connect to {}", args.connect))?;
    eprintln!("✅ Client connected.");

    // 3. 一些共享状态：
    //    - `stream` 供回调线程写入
    //    - `seq` 自增序号，方便在服务端调试
    //    - `debug` 标记是否输出更多日志
    let stream = Arc::new(Mutex::new(stream));
    let seq = Arc::new(AtomicU64::new(1));
    let debug = Arc::new(args.debug);

    // 复制到回调闭包中
    let stream_clone = stream.clone();
    let seq_clone = seq.clone();
    let debug_clone = debug.clone();

    // 4. 捕获键鼠事件（`rdev` 需要传入一个同步回调）
    let callback = move |event: rdev::Event| {
        // 将 `rdev` 的事件映射成我们自己的 `InputEvent`
        if let Some(ev) = map_event(event) {
            let env = EventEnvelope {
                ts_millis: now_millis(),
                // 递增序号并返回旧值
                seq: seq_clone.fetch_add(1, Ordering::Relaxed),
                event: ev,
            };

            // 根据约定：帧格式 = [u32 little-endian 长度] + payload
            let mut payload = encode_env(&env);
            let len = payload.len() as u32;
            let mut framed = len.to_le_bytes().to_vec();
            framed.append(&mut payload);

            // 上锁后写入异步 TCP 流。由于回调是同步的，
            // 这里使用 `block_on` 将异步写操作阻塞执行。
            if let Ok(mut guard) = stream_clone.lock() {
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

    // 5. 调用 `rdev::listen` 开始监听键鼠事件。
    //    如果监听过程中发生错误，直接打印出来。
    eprintln!("🖱️  Client capturing mouse/keyboard (rdev)...");
    if let Err(e) = rdev::listen(callback) {
        eprintln!("client capture error: {:?}", e);
    }
    Ok(())
}

/// 将 `rdev::Event` 转换为 `InputEvent`
fn map_event(ev: rdev::Event) -> Option<InputEvent> {
    use rdev::EventType;
    match ev.event_type {
        // 鼠标移动事件，注意 `rdev` 返回的是 `f64`，这里转换为 `i32`
        EventType::MouseMove { x, y } => Some(InputEvent::MouseMove {
            x: x as i32,
            y: y as i32,
        }),
        // 鼠标按下，`down: true`
        EventType::ButtonPress(btn) => map_button(btn).map(|b| InputEvent::MouseButton {
            button: b,
            down: true,
        }),
        // 鼠标释放，`down: false`
        EventType::ButtonRelease(btn) => map_button(btn).map(|b| InputEvent::MouseButton {
            button: b,
            down: false,
        }),
        _ => None,
    }
}

/// 将 `rdev` 的按键枚举映射为我们定义的 `MouseButton`
fn map_button(btn: rdev::Button) -> Option<MouseButton> {
    use rdev::Button::*;
    Some(match btn {
        Left => MouseButton::Left,
        Right => MouseButton::Right,
        Middle => MouseButton::Middle,
        // 其它按键使用原始 code 填充
        Unknown(code) => MouseButton::Other(code as u8),
    })
}
