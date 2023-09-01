# Q&A

## 关于 wasi

wasmedge 是一个运行时，它也是按 `wasm spec` 提供了各种 sdk 接口，我们是需要在这个 sdk 接口之上，再做一层接口，使其符合 WASI 标准这样？  
其他 wasm runtime 也是一样做的吗？  
其实我们现在已经可以用 wasmedge 提供的 sdk 去开发 wasm 程序，在非浏览器端运行了，wasi 的作用只是用来统一 wasm 的使用规范吗?

用户是如何使用 wasi 接口的？为什么 rust 代码经过编译(wsm32-unknown-wasi为编译目标)之后就能够直接使用 wasmedge 的 wasi 接口了(包括用的 rust std 函数)？

## 关于 wasm 如何运行

以 rust 和 wasmedge 为例

method1:  
首先编写 rust 代码，然后将其编译成 wasm (编译目标为 wasm32-unknown-wasi)，之后使用 wasmedge 运行时运行 wasm 模块。

method2:  
编写 rust 代码，然后编译成 wasm，再在另一个 rust 代码中调用 VM 去 load wasm，然后运行。
