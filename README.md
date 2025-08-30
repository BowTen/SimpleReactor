# Simple Reactor ✨

参考 Muduo 库，使用 Rust 实现的 Reactor 模式网络库。

## 设计想法

 - 由于Rust在并发访问上非常保守，不好实现C++那样灵活的访问。我让每个Reactor绑定在一个线程中，每个Socket绑定一个Reactor，仅能在该线程直接访问。
 - 每个Reactor有一个线程安全的信号队列，可向其发送不同类型的信号从而在外部线程控制Reactor。ReactorRemote和SocketRemote都是对SignalSender的封装

## 示例程序

位于 `src/bin/` 目录：

- `echo_server.rs` - TCP & UDP echo服务器
- `echo_tcp_client.rs` - TCP 客户端
- `echo_udp_client.rs` - UDP 客户端

## 项目结构

```
src/
├── lib.rs              # 库入口
├── reactor.rs          # Reactor 核心实现
├── server.rs           # TCP & UDP 服务器
├── client.rs           # TCP & UDP 客户端
├── tcp_connection.rs   # TCP 连接封装
├── udp_socket.rs       # UDP 套接字
├── reactor_remote.rs   # 线程安全的 Reactor 控制器
├── socket_remote.rs    # 线程安全的 Socket  控制器
├── event_loop_thread.rs    # 事件循环线程
├── event_loop_thread_pool.rs # 线程池
├── buffer.rs           # 缓冲区实现
├── callbacks.rs        # 回调函数定义
└── bin/                # 示例程序
    ├── echo_server.rs
    ├── echo_tcp_client.rs
    ├── echo_udp_client.rs
    └── simple_server.rs
```

## 模块架构

### reactor 结构图
![reactor 结构图](https://github.com/BowTen/SimpleReactor/raw/main/resources/reactor_arc.png)

### server 结构图
![server 结构图](https://github.com/BowTen/SimpleReactor/raw/main/resources/server_arc.png)


## TODO

目前只实现了 TCP,UDP 基础的 Server 和 Client 功能，只处理了读、写事件。
错误处理没有认真考虑，接口、模块设计也还可以优化

 - UdpSocket 处理事件时遇到一些错误可能导致系统关闭socket，库中也应该shutdown
 - 处理更多事件类型
 - 绘制模块图


参考了 [Muduo](https://github.com/chenshuo/muduo) 😘