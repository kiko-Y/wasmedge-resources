1. 用户如何使用插件? 直接通过 Rust Crate 使用? 在两种 WasmEdge 应用形式中解释这个问题.

2. VSCode 扩展 rust-analyzer 如何指定 workspace 目录? 由于 WasmEdge 根目录中没有 Cargo.toml, 其会认为不是有效的 Rust 工作空间. 

3. 什么是 Module, 什么是 Instance? 有如此多种类的 Instance, 包括 Func Instance, Executor Instance

先选定一个WASI接口, 看其C++是如何实现的, 然后再看Sam的Rust是怎么实现的.

综合这两者, 用Rust SDK实现, 然后再考虑用户如何使用.

这个WASI接口就选择 args_size_get

直接实现 args_size_get 有点困难, 把这个接口变得更简单.

实现一个打印 Hello, World 的接口, 然后编译成插件.

然后让用户通过两种方式使用插件, 第一种是编译型, 第二种是应用型.

在 [Access OS Services - WasmEdge Runtime](https://wasmedge.org/book/en/write_wasm/rust/wasi.html) 中, 用户使用 Rust 标准库编写 src.rs 程序, 编译成 src.wasm, 然后用命令 wasmedge src.wasm 执行, 使用了 WASI 接口, 但整个过程都感觉不到 WASI 接口的存在. 

如果我们用插件的形式实现 WASI 接口, 用户能以上述方式使用 WASI 接口而不需要知道实现细节吗? 

还是说用户必须先写一个 host app, 在 host app 中用 PluginManager, Plugin 等API访问插件进而访问WASI接口



## 05-10

目前无法让 Hello, World 通过编译型使用, 通过应用型使用已经[实现](../wasm/wasmedge-rustsdk-example/simple-plugin/naive-math-host-app/src/main.rs)

直接从 args_size_get 开始看 C++ 和 Rust 实现.
