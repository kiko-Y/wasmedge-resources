# WASI 相关基础知识

## 1. 简介

### 1.1. Why WASI, What is WASI

WASI(WebAssembly System Interface)

因为 WebAssembly 首先是在浏览器端被使用，因为其速度快、安全性好而且易于在不同架构的机器上运行，所以现在 WebAssembly 希望可以用在**非浏览器端**的应用场景中。

由于 wasm 是一种底层语言，类似汇编语言，可以看做是运行在一种概念上的物理机器(`conceptual machine`) 上的，如果需要供外部使用的话，我们就需要一个 概念上的操作系统(`conceptual operating system`)，然后向外界提供一组**系统调用接口**，各种 `wasm runtime` 就可以看做是操作系统，而这组系统调用接口就是我们所说的 **WASI**，不同的运行时可能会向外提供不同的接口，为了标准化 wasm runtime，就需要提供一组标准化的 wasm 运行时的接口来做规范，所以就有了 **WASI 标准**。

### 1.2. 用户如何使用 wasi 接口
