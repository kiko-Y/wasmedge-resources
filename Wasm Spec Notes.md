Wasm Spec 分几个部分讲解了 Wasm 的语法和语义.

某个特性的语法和语义分散在不同的部分中. 以 Module 为例, Structure 章节中讲解了 Module 的定义, Validation 章节中讲解了 Module 什么时候是有效的, Execution 章节中讲解了 Module 是如何被实例化的. 这一点看说明书的时候要注意, 要综合来看.

## Runtime Structure

### Store

在WebAssembly中，模块实例是一个独立的执行单元，包含了函数、表、内存、全局变量等元素的定义和实例化。这意味着只有在相应模块实例的上下文中，才能访问和操作模块实例中的元素和数据。(这在WasmEdge中是如何体现的? 一个VM包括什么? 一个Module呢? Store对应WasmEdge中的什么单元?)

### Module Instance

一个 Module Instance 是一个 Module 的运行时表示, 包括导入的实体(Import, 从其他模块导入的), 模块本身定义的实体, 导出的实体(Exported, 供其他模块使用的)


