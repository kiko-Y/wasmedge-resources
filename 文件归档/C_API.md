# WASMEDGE C API

wasmedge c 的 api 概览，对照着[官方文档](https://wasmedge.org/docs/embed/c/reference/latest#version)看吧

## Part 1 WasmEdge Basics

### Version

提供了获取 **wasmedge 共享库版本**的一系列接口。

### Logging Settings

提供了**设置日志级别**或者**禁止日志**的一系列接口。

### Value Types

在 WasmEdge 中，开发者要通过 API 把所有的 values 转换成 `WasmEdge_Value` 对象再进行使用。

提供了一系列的**类型转换**接口，主要是将 C 中的类型和 `WasmEdge_Value` 类型相互转换。

包括数值类型和引用类型的类型转换，数值类型包括 `i32`, `i64`, `f32`, `f64`, `v128`。

引用类型包括方法引用 `funcref`，外部引用 `externref`。

还有判断引用是否为空等接口。

### Strings

提供了 C 的字符串转换和 `WasmEdge_String` 之间转换的一系列接口。
还有字符串比较的接口。

### Results

`WasmEdge_Result` 对象表示执行的结果。
提供一系列接口，包括判断结果是否成功，结果的 **code** 和 **message**等信息。

### Contexts

提供了创建包括 `VM`, `Store`, `Function` 在内的一系列对象的接口。

### WASM Data Structures

`Limit`: 用于 `Memory`、`Table` 等的创建，指定容量，结构如下:

```c
typedef struct WasmEdge_Limit {
  /// Boolean to describe has max value or not.
  bool HasMax;
  /// Boolean to describe is shared memory or not.
  bool Shared;
  /// Minimum value.
  uint32_t Min;
  /// Maximum value. Will be ignored if the `HasMax` is false.
  uint32_t Max;
} WasmEdge_Limit;
```

`Function type context`: 用于创建 WasmEdge 中的 `Function`、获取方法的信息等功能，包含入参出参类型。

`Table type context`: 用于创建 WasmEdge 中的 `Table`，由 `WasmEdge_RefType` 和 `WasmEdge_Limit` 构造。

`Memory type context`: 用于构造 WasmEdge 中的 `Memory`，指定 `WasmEdge_Limit` 构造。

`Global type context`: 用于构造 WasmEdge 中的 `Global`。

`Import type context`: 用于获取 `AST Module` 中的导入信息。

`Export type context`: 用于获取 `AST Module` 中的导出信息。

### Async

todo

### Configurations

`WasmEdge_ConfigureContext` 管理了 `Loader`, `Validator`, `Executor`, `VM` 和 `Compiler` 的配置。

配置包括:

`Proposals`: 可以开启或者关闭 WebAssembly proposals。

`Host registrations`: 仅用于 `VM`，是否开启 `WASI` 支持。

`Maximum memory pages`: 用于 `Exector` 和 `VM`，管理内存页大小。

`Forcibly interpreter mode`: 执行 `.wasm` 的时候强制开启解释模式。

`AOT compiler options`: 配置 AOT 的优化等级，以及编译结果的形式。

`Statistics options`: 作用于 `Compiler`, `VM` 和 `Executor`，作用todo。

### Statistics

`WasmEdge_StatisticsContext` 提供了一系列的运行时数据统计，包括指令计数器、耗时统计。

## Part 2 WasmEdge VM

主要介绍 `WasmEdge_VMContext` 对象。
