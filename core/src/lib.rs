//! `core` 模块：定义了客户端与服务端之间共享的基础数据结构
//! 以及简单的序列化/反序列化函数。为了方便学习，下面的代码
//! 都配有较为详细的中文注释。

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 鼠标按键的枚举
///
/// `Other(u8)` 用于保存无法识别或未列出的按键编号，
/// 以便在两端传递时不丢失信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    /// 左键
    Left,
    /// 右键
    Right,
    /// 中键（滚轮按下）
    Middle,
    /// 其它按键，保存原始的按键代码
    Other(u8),
}

/// 我们自定义的输入事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    /// 鼠标移动事件，使用绝对坐标表示
    MouseMove { x: i32, y: i32 },
    /// 鼠标按键事件，`down = true` 表示按下，`false` 表示释放
    MouseButton { button: MouseButton, down: bool },
}

/// 网络上传输的“事件封包”，在 `InputEvent` 外再包一层元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// 发送端时间戳（UNIX 毫秒）
    pub ts_millis: u128,
    /// 发送端事件自增序号，便于调试/排查丢包
    pub seq: u64,
    /// 实际的输入事件
    pub event: InputEvent,
}

/// 获取当前的 UNIX 时间戳（毫秒）
pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// 使用 `bincode` 将事件封包序列化为字节数组
pub fn encode_env(env: &EventEnvelope) -> Vec<u8> {
    bincode::serialize(env).expect("serialize env")
}

/// 尝试从字节切片反序列化出事件封包
///
/// 反序列化失败时返回 `None`
pub fn decode_env(buf: &[u8]) -> Option<EventEnvelope> {
    bincode::deserialize(buf).ok()
}
