// core/src/lib.rs
//! 事件模型与编解码
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton { Left, Right, Middle, Other(u8) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove { x: i32, y: i32 },
    MouseButton { button: MouseButton, down: bool },
    // 预留：键盘
    // Key { code: u32, down: bool }
}

pub fn encode(event: &InputEvent) -> Vec<u8> { bincode::serialize(event).expect("serialize") }
pub fn decode(buf: &[u8]) -> Option<InputEvent> { bincode::deserialize(buf).ok() }
