1. 用户如何使用插件? 直接通过 Rust Crate 使用? 在两种 WasmEdge 应用形式中解释这个问题.
   
    目前的理解是：~~用户将需要使用的插件的动态库安装到插件库中，然后在用户的程序中声明需要用的插件模块以及需要使用的方法，然后进行调用。~~  
    update: 有两个crate，crate a是用来实现插件的，通过**host function**?，同时会向外export 调用接口，crate b 会依赖 crate a 对外 export 的接口，进行再一次的封装供用户使用。所以用户用的就是 crate b。  
    不知道能否这样理解，是否能够把实现插件的部分视为一个 crate。  
    想到的另一种解释就是：用户只需要使用依赖了插件的那个 crate，然后将插件的动态库(linux 下的 .so 文件)放到 wasmedge 的插件库中，插件库指的是[这个例子](https://github.com/second-state/wasmedge-rustsdk-examples/tree/main/simple-plugin)里的`/usr/local/lib/wasmedge`
   
    关于实现 WASI 的插件：
   
   > Sam: wasmedge内部直接包含了一个wasi的标准实现，也就是cpp实现。这个实现不是以插件的方式提供的。在使用“cargo build --target wasm32-wasi”命令编译这个rust例子之后，生成的wasm文件就自带了wasi的一些“声明”。在使用 “wasmedge --dir .:. target/wasm32-wasi/debug/wasi.wasm hello” 命令执行的时候，会自动引入wasmedge wasi module，从而完成对标准接口的调用。这个调用与plugin的差别就在于，built-in wasi不用再指明所属module的name，而plugin的方式是需要有个过程指明plugin module的名字，这样wasmedge runtime才能通过module name 加载相应的module。  
   > 从实际实现上来说，采用plugin的方式之后，不要使用“wasmedge --dir .:. target/wasm32-wasi/debug/wasi.wasm hello” 这样的wasmedge CLI来进行测试，因为这个CLI命令会自动开启wasmedge built-in wasi implementation。  
   > 在wasmedge-sys的plugin.rs中，有test cases可以作为验证plugin的参考，比如plugin-wasmedge-process的[test case](https://github.com/apepkuss/WasmEdge/blob/master/bindings/rust/wasmedge-sys/src/plugin.rs#L463-L506) 。这个test case中，描述了验证plugin的一些步骤，其中需要注意的是：PluginManager::load_plugins_from_default_paths(); ，这行代码是从wasmedge plugin的默认路径加载plugins。只有执行了这一步，后续plugin中定义的相应模块才能找到

2. 用插件实现的 WASI 接口是通过导入 crate 调用的，之后就可以不用内置的 WASI 接口了吗？这两种实现对于用户来说会有什么区别吗？
   
    根据Sam的回答：
   
   > crate是给用户用的。原因是，从设计这个crate的动机来看，它北侧的接口（也就是用户看到的接口）是使用rust built-in types作为参数的接口，也就是说，可以是一些复杂的类型，比如struct等；而它的南向接口（也就是“对接”wasmedge plugin的接口）其参数类型通常是WebAssembly types （在wasmedge Rust SDK中就是WasmValue类型）支持的类型，比如i32, i64等等。南向接口的参数类型对于用户来说是不友好的，所以这个crate的设计动机实际上就是为了让你设计的plugin中所包含的host functions，对于不熟悉webassembly的用户来说，学习的成本降为0。
   
    简单来说就是让 rust 用户在使用 wasmedge 的时候不需要关心 webassembly spec 定义的类型，正常使用 rust built-in type 即可，降低了学习成本。
   
    但是在[Access OS services](https://wasmedge.org/book/en/write_wasm/rust/wasi.html)中，用户也可以使用 rust built-in types 调用函数， 然后使用`cargo build --target wasm32-wasi`编译，生成的.wasm 文件是自动带有 wasi 接口的声明的。那么在哪些情况下，不使用插件实现的 wasi 接口会使用户接触到 webassembly type 呢？
   
    而且，wasi 接口现在就需要用户通过 crate 来调用，而不是使用 `cargo build --target wasm32-wasi` 自动生成声明了？

3. VSCode 扩展 rust-analyzer 如何指定 workspace 目录? 由于 WasmEdge 根目录中没有 Cargo.toml, 其会认为不是有效的 Rust 工作空间. 
   
    创建一个Cargo.toml 然后在里面加上在 workspace，members 属性中填上属于 workspace 的 packages，如下所示:  
    ![](README.assets/rust-workspace.jpg)

4. 什么是 Module, 什么是 Instance? 有如此多种类的 Instance, 包括 Func Instance, Executor Instance

5. --target wasm32-wasi 的作用是什么，只是将目标编译成 wasm 模块吗
   
   > WASI provides a standardized interface for WebAssembly modules to interact with the host operating system in a secure and platform-independent manner. By targeting wasm32-wasi, you're specifying that the Wasm module should be built with the necessary interfaces and capabilities to interact with the underlying system through the WASI runtime.  
   > It's important to note that in order to execute a Wasm module compiled with wasm32-wasi, you will need a WASI-compliant runtime or environment that provides the necessary system interfaces and capabilities defined by the WASI specification. 
   
    意思是把编译目标设置为 wasm32-wasi，从而可以使用提供 WASI 接口的 runtime 进行执行？
   
    另一个问题是，如果以 wasm32-wasi 为编译目标，那么不同的提供了 WASI 接口的 runtime 都可以执行这个 wasm 文件吗

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

#### WASI 接口在 C++ 中是如何实现的

所有的 WASI 接口都统一在 WasiModule 中 (见wasimodule.cpp)

根据继承的含义, 一个 WasiModule 是一个 ModuleInstance (wasimodule.h) (问题在于, 什么是ModuleInstance ?)

一个 WasiArgsSizesGet 是一个 Wasi (见wasifunction.h)

一个 Wasi 是一个 HostFunction, 一个 Wasi 需要一个 Environ, 所有 Wasi 共享一个 Environ (见 wasibase.h 和 wasimodule.h)

如果已经有了 Environ, 则实现 args_sizes_get 非常简单. 问题在于, Environ 何时调用 init() 被初始化? WasiModule 何时被创建? 一个 ModuleInstance 是怎么样的存在

### 05-11

在 Wasm Spec 中, 一个VM是如何定义的? 一个VM都包括什么
