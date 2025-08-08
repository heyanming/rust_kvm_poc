use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: MouseButton, down: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// 发送端时间戳（UNIX 毫秒）
    pub ts_millis: u128,
    /// 发送端事件自增序号
    pub seq: u64,
    /// 实际事件
    pub event: InputEvent,
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn encode_env(env: &EventEnvelope) -> Vec<u8> {
    bincode::serialize(env).expect("serialize env")
}

pub fn decode_env(buf: &[u8]) -> Option<EventEnvelope> {
    bincode::deserialize(buf).ok()
}
