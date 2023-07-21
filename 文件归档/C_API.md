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

主要介绍 `WasmEdge_VMContext` 对象，VM 用来加载注册 `wasm module`，并调用各种 `function`。

### VM 如何加载 wasm 文件并调用对应的方法

下面是 VM 加载 wasm 文件并执行相应方法的整个步骤流。
<img src="../README.assets/WasmEdge-VM-work-flow.png" width=500>

1. Initiate: 初始化 VM
2. Load: 加载 wasm 文件到 VM 中
3. Validate: 验证加载的 wasm module
4. Instantiate: 实例化 wasm module
5. Execute: 执行 wasm function

### VM Creations [🔗](https://wasmedge.org/docs/embed/c/reference/latest/#vm-creations)

VM 的构建需要传入 `WasmEdge_ConfigureContext` 和 `WasmEdge_StoreContext`，如果用默认的配置，就传空即可。

### Built-in Host Modules and Plug-in Preregistrations [🔗](https://wasmedge.org/docs/embed/c/reference/latest/#built-in-host-modules-and-plug-in-preregistrations)

WasmEdge 提供了以下的内置 `host modules` 和 `plug-in`

1. Wasi
可以在配置中打开 WASI 支持  
也可以创建 WASI 的 module instance
2. plug-ins
默认路径下有若干插件可供使用(首先需要下载 WasmEdge plug-ins)  
使用插件之前需要先**加载**插件

`VM Context`会在创建的时候自动创建和注册已经加载的插件模块

### Host Module Registrations [🔗](https://wasmedge.org/docs/embed/c/reference/latest/#host-module-registrations)

`Host Funciton` 是 wasm 外部的方法，通过导入到 `wasm module` 使用。在 WasmEdge 中， `Host Function` 组合进 `Host Module` 当中，作为一个 `WasmEdge_ModuleInstanceContext` 对象，并拥有一个模块名，注册到 VM 中使用。

### WASM Registrations And Executions [🔗](https://wasmedge.org/docs/embed/c/reference/latest/#wasm-registrations-and-executions)

在 WebAssembly 中，`wasm module` 中的 instance 可以被导出或者被其他 wasm 模块导入。WasmEdge VM 提供了一系列的 API 来注册和导出 `wasm module`，并且可以执行注册了的 `wasm module` 的 `host function` 或者 `function`(function 是在 wasm module 中的，host function 是在 host module 中的)。

### Asynchronous Execution [🔗](https://wasmedge.org/docs/embed/c/reference/latest/#asynchronous-execution)

提供了异步执行的方法

### Instance Tracing

用于获取 VM 中的实例

1. Store
   可以给 `VM` 初始化一个 `Store`，如果没有的话，`VM` 会自动分配一个 `Store`
   提供了获取 `Store` 的接口
2. List exported functions
   提供了接口来获取**方法名**以及**方法参数**列表
3. Get function types
   提供了接口来获取方法类型
4. Get the active module
   当 wasm 模块初始化之后，`VM` 会实例化一个 `anonymous module instance`
   提供了接口来获取 `anonymous module instance`
5. List and get the registered modules
   提供了接口来获取以及注册的 `module instance`
6. Get the components
   获取 `VM` 中的组件，包括 `Loader`, `Validator` 和 `Executor`。
