# rust_kvm_poc
最小可运行 PoC：在一台机器（host）捕获鼠标/键盘事件，通过 TCP 传到另一台（client）并在那边注入。
> 目标：先跑通“鼠标移动 + 左键点击”的单向传输。键盘与更多功能逐步迭代。

## 架构
- `core/`：事件模型 + 编解码（serde + bincode）
- `host/`：捕获（rdev）+ 发送（tokio TCP 客户端）
- `client/`：接收（tokio TCP 服务端）+ 注入（enigo）

## 使用
### 启动 client（Server（被控制端））
cargo run -p kvm_server -- --listen 0.0.0.0:50051
### 启动 host（Client（主控端））
cargo run -p kvm_client -- --connect 192.168.1.79:50051

> macOS 需授予“输入监控/辅助功能”权限；Windows 可能需管理员权限。
> 目前用绝对坐标，未处理多屏/DPI，后续迭代。
