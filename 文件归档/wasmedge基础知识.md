# wasmedge 基础知识

## WebAssembly的概念和定义

### 1. 依赖
WebAssembly依赖于两个现存标准
1. IEEE754，用于浮点数的表示以及算数运算符的语义
2. Unicode，用于模块导入和导出的命名以及文本类型的格式

### 2. WebAssembly的定位
WebAssembly(wasm)是一种低级的，类似于汇编的语言。最初是为了提高浏览器端渲染运算的速度而提出的一种方案，在JavaScript中导入 wasm 模块来提高运行速度，因为 wasm 和机器语言更接近，而且已经经过了 AOT 编译，优化了性能。之后在服务端也用到了 wasm，包括云原生、边缘计算和去中心化应用，也用在了微服务和 serverless 应用上。

### 3. 概念
#### Values
WebAssembly 只提供了四种数值类型：**i32、i64、f32、f64**，其中 i32 同时用于布尔值和内存地址。
除此之外，还有 128 位的 **vector** 类型，用于表示组合数据。比如 4 个 32-bit、 2个 64-bit的 IEEE754 数值，或者 2 个 64-bit integer、4 个 32-bit integer、8 个 16-bit integer 或 16 个 8-bit integer。
最后，values 还可以由一些引用组成，作为指针指向不同的实体。

#### Instructions
WebAssembly 是基于 stack machine 的，指令顺序执行，有一个隐含的操作数栈，在这个栈上对 values 进行操作。指令分为两种类型：**Simple instructions** 和 **Control instructions**。其中 Simple instructions 负责对数据的基础操作，从栈顶 pop 出数据，操作完之后将数据存入栈顶。Control instructions 负责改变控制流，控制流包括 blocks、loops 和 conditions。

#### Traps
在一些情况下，一些指令会造成 **trap**，会立即终止执行。Traps 不能被 WebAssembly 代码处理，但是可以报告给外部环境，由外部环境捕获处理。

#### Functions
WebAssembly 代码被组织成不同的 **functions**，每个 function 可以接收多个 values 作为入参，然后返回若干个 values 作为结果。Function 之间可以互相调用，也可以递归调用，会形成递归调用栈。Functions 也可以声明可变的本地变量，作为*虚拟的寄存器*。

#### Tables
table 是一系列不透明的 values，由特定的一些 **element type** 组成。允许程序通过索引来选取 table 中的元素。目前可用的 element type 只有**无类型的方法引用(untyped function reference)**以及**外部宿主 value 的引用(reference to an external host value)**。


#### Linear Memory
Linear Memory 是一个连续可变的字节数组。创建的时候会有一个初始内存，并且可以动态增长。程序可以从 Linear Memory 的任何字节地址中 load 或者 store values。数值类型在load 和 store 过程中可以选取一个比自身大小小的 storage size。如果使用的地址超过了边界，则会产生 [**trap**](#traps)。


#### Modules
WebAssembly binary 以 module 的形式组织，WebAssembly modules 包含 functions、tables、linear memories 和可变的或者不可变的 global variables 的 **definition**。**definition** 可以从外部导入(**import**)。也可以通过一个或多个的名称进行导出(**exported**)。
除了 definitions，modules 还可以通过复制指定偏移量位置的 **segments** 为其 memories 和 tables 初始化数据。也可以定义一个 **start function**，这个函数会自动地去执行。

#### Embedder
一个 WebAssembly 的实现通常会嵌入(**embedded**)到宿主环境(**host environment**)中去。环境定义了加载的模块如何初始化，提供哪些导入(**imports**)，定义了导出(**exports**)如何被获取。具体细节和环境有关。

### 4. Semanic Phases
主要分三块
#### Decoding


#### Validation
验证解码后模块，保证有意义且是安全的。会对方法的类型和指令序列进行检查

#### Execution
执行可以细分两块
##### 1. Instantiation
把模块实例化为 **module instance**，类似于程序和进程的关系，module instance 是 module 的动态表示，有自己的状态和执行栈。初始化会执行 module 本身，导入所有的 imports，并初始化 global variables、memories 和 tables。并调用 **start function**。会返回 module 的导出实例。
##### 2. Invocation
实例化后可以调用 WebAssembly 的导出函数，给定需要的入参，执行相应的函数，返回结果。

Instantiation 和 Invocation 都是在宿主环境中执行的。


## 5. 定义
包括了值的范围定义，元数据的定义以及名称的定义
这里的定义使用了文本描述，而非抽象语法(abstract syntax)描述
### 5.1. Values
#### 5.1.1 Bytes
**byte** 可以表示为`0x00`到到`0xFF`之间的值
#### 5.1.2 Integers
**uN** 表示$0...2^N-1$
**sN** 表示$-2^{N-1}...2^{N-1}-1$
**iN** 同uN
#### 5.1.3 Floating-Point
符合IEEEE754标准的浮点数
#### 5.1.4 Vectors
表示 128-bit values，用 i128 表示

#### 5.1.5 Names
**name** 可以表示为若干个 char
**char** 可以由Unicode表示 U+00 - U+D7FF $\cup$ U+E000 - U+10FFFF


### 5.2 Types
这部分具体看 [spec](https://webassembly.github.io/spec/core/_download/WebAssembly.pdf)
#### 5.2.1 Number Types
$numtype ::= i32 \mid i64 \mid f32 \mid f64$

#### 5.2.2 Vector Types
$vectype ::= v128$

#### 5.2.3 Reference Types
$reftype ::= funcref \mid externref$
其中 funcref 表示为各种方法的引用，externref 表示所有宿主机中可以传入到 WebAssembly 的对象引用
这些引用都保存在[tables](#tables)中

#### 5.2.4 Value Types
$valtype ::= numtype \mid vectype \mid reftype$

#### 5.2.5 Result Types
$resulttype ::= [vec(valtype)]$
value的组合

#### 5.2.6 Function Types
$functype :: = resultype \rightarrow resulttype$

#### 5.2.7 Limits
$limits ::= \{\min u32, \max u32^?\}$
memory 和 table 的size范围，可以没有最大限制

#### 5.2.8 Memory Types
$memtype ::= limits$

#### 5.2.9 Table Types
$tabletype ::= limits\space reftype$

#### 5.2.10 Global Types
$globaltype ::= mut\space valtype$
$mut ::= const \mid var$

#### 5.2.11 External Types
$externtype ::= func functype \mid table tabletype \mid mem memtype \mid global globaltype$



## WasmEdge 中的概念和定义
### 1. HostFunction
什么是 HostFunction？
引用自[ref](https://www.secondstate.io/articles/extend-webassembly/)

> &emsp;&emsp;WebAssembly was developed for the browser. It gradually gain popularity on the server-side, but a significant disadvantage is its incomplete functionality and capability. The WASI proposal was initiated to solve these problems. But the forming and implementation of a standard is usually slow.  
> &emsp;&emsp;What if you want to use a function urgently? The answer is to use the Host Function to customize your WebAssembly Runtime.  
> &emsp;&emsp;As the name suggests, a Host Function is a function defined in the Host program. For Wasm, the Host Function can be used as an `import` segment to be registered in a `module`, and then it can be called when Wasm is running.  
> &emsp;&emsp;Wasm has limited capability, but those can't be achieved with Wasm itself can be resolved with Host Function, which **expanded Wasm functionality to a large extent**.  
> &emsp;&emsp;WasmEdge‘s other extensions apart from standards are majorly based on Host Function, for example, WasmEdge‘s Tensorflow API is implemented with Host Function and thus achieving the goal of running AI inference with the native speed.  
> &emsp;&emsp;Networking socket is implemented with host function as well. Thus we can run asynchronous HTTP client and server in WasmEdge which compensate for the WebAssembly's disadvantage in networking.  
> &emsp;&emsp;Another example. Fastly uses Host Function to add HTTP Request and Key-value store APIs to Wasm which added the extension functions.

简单来说就是因为 wasm 能够提供的功能有限，有些无法用 wasm 实现的功能可以使用 `host function` 进行实现。而 `host function` 则是定义在 **host program** 的方法，通过 **import module** 导入到 wasm，然后进行使用。
[C实现 Host Function 的例子](https://wasmedge.org/docs/embed/c/host_function)