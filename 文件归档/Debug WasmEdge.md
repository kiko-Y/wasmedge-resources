通过调试 WasmEdge 以更好的理解其组件

## Debug WasmEdge in Ubuntu Contanier

1. 配置好 Docker, 进入容器终端

2. ```shell
   cd /root/wasm
   git clone git@github.com:WasmEdge/WasmEdge.git
   cd WasmEdge
   apt install -y software-properties-common cmake libboost-all-dev
   apt install -y llvm-14-dev liblld-14-dev
   apt install -y gcc g++ gdb
   apt install -y clang-14
   mkdir build && cd build
   cmake -DCMAKE_BUILD_TYPE=Debug -DWASMEDGE_BUILD_TESTS=OFF .. 
   make -j8
   # 目标产物位于build目录中, 二进制文件位于 ./tools/wasmedge/wasmedge
   ```

3. 安装 Rust 和 Cargo, 使用 default 选项安装
   安装 Cargo 编译 wasm32-wasi 的目标格式

   ```shell
   curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
   rustup target add wasm32-wasi
   ```

4. 编写一个 Rust 的 HelloWorld 程序:

   + 新建并进入目录 `/root/wasm/examples`, 执行 `cargo new hello` 和 `cd hello`

   + 编辑 `src/main.rs` 文件, 如下

     ```rust
     fn main() {
         let s : &str = "Hello WasmEdge!";
         println!("{}", s);
     }
     ```

   + 在 hello 目录中执行 `cargo build --target wasm32-wasi --release`, 产物位于 `hello/target/wasm32-wasi/release/hello.wasm`

   + 将产物复制到 wasmedge 同级目录, 执行 `cp /root/wasm/examples/hello/target/wasm32-wasi/release/hello.wasm /root/wasm/WasmEdge/build/tools/wasmedge/hello.wasm`

   + 进入 wasmedge 所在目录, 执行 `cd /root/wasm/WasmEdge/build/tools/wasmedge`

   + 执行 `gdb --args ./wasmedge hello.wasm` 启动 GDB, 如下命令在 GDB 终端中执行

     + `b VM::unsafeValidate()`, 加一个断点
     + `run`, 执行, 会在断点处卡住. 通过 `next` 下一步, 通过 `continue` 继续执行
     + `continue`, 继续执行, 终端上输出 "Hello WasmEdge", 即为成功.

5. 使用 VSCode 图形界面调试: 太卡了, 4核8G都不够用, 遂放弃.

## Debug WasmEdge in Darwin Locally

1. ```shell
   cd WasmEdge
   brew install cmake ninja llvm
   export LLVM_DIR="$(brew --prefix)/opt/llvm/lib/cmake"
   export CC=clang
   export CXX=clang++
   cmake -Bbuild -GNinja -DCMAKE_BUILD_TYPE=Debug -DWASMEDGE_BUILD_TESTS=OFF .
   cmake --build build
   ```

2. 和上述同理, 执行 `lldb ./wasmedge hello.was` (据说 MAC 里都使用 lldb 而不是 gdb; MAC里使用gdb还要自己加证书, 太麻烦)

## 线索

wasmedge 和 wasmedgec 都是 tools, main函数分别在 WasmEdge/tools/wasmedge/wasmedge.cpp 和同级目录下的 wasmedgec.cpp 中.

执行 `wasmedge hello.wasm` 的调用栈:

1. tools/wasmedge/wasmedge.cpp 中进入主函数 main, 调用 lib/api/wasmedge.cpp 中的**WasmEdge_Driver_UniTool** 函数, 进一步调用到 lib/driver/uniTool.cpp 中的 WasmEdge::Driver::UniTool() 函数.

   > UniTool 的含义我猜是 Universal Tool

2. UniTool 函数中对输入的参数进行了解析, 存到了类 **DriverToolOptions** 的一个实例中, 以名字 Opt 传给了 lib/driver/runtimeTool.cpp 中的 Tool 函数, wasmedge 运行的流程在该函数中能看出个大概.

3. 在 Tool 函数中, 首先定义了 WasmEdge::Configure 的一个实例 Conf, 该 Conf 用于初始化 VM.

   ```C++
   Conf.addHostRegistration(HostRegistration::Wasi);	// 打一个标记
   const auto InputPath =
       std::filesystem::absolute(std::filesystem::u8path(Opt.SoName.value()));
   VM::VM VM(Conf);
   ```

   Configure 类中有定义, 很显然是用于标记宿主的xxx (这里应该是宿主函数 or 宿主模块?)
   ```
   std::bitset<static_cast<uint8_t>(HostRegistration::Max)> Hosts;
   ```

   然后使用 Conf 初始化 VM, 调用到

   ```C++
   VM::VM(const Configure &Conf)
       : Conf(Conf), Stage(VMStage::Inited),
         LoaderEngine(Conf, &Executor::Executor::Intrinsics),
         ValidatorEngine(Conf), ExecutorEngine(Conf, &Stat),
         Store(std::make_unique<Runtime::StoreManager>()), StoreRef(*Store.get()) {
     unsafeInitVM();
   }
   void VM::unsafeInitVM() {
     // Load the built-in modules and the plug-ins.
     unsafeLoadBuiltInHosts();
     unsafeLoadPlugInHosts();
   
     // Register all module instances.
     unsafeRegisterBuiltInHosts();
     unsafeRegisterPlugInHosts();
   }
   
   // 在该函数中, 构建了 WasiModule, 并存到了VM的成员变量里.
   // 该函数仅仅放入了 Wasi, 可见目前只有Wasi一个需要 registration 的 host
   void VM::unsafeLoadBuiltInHosts() {
     // Load the built-in host modules from configuration.
     // TODO: This will be extended for the versionlized WASI in the future.
     BuiltInModInsts.clear();
     if (Conf.hasHostRegistration(HostRegistration::Wasi)) {
       std::unique_ptr<Runtime::Instance::ModuleInstance> WasiMod =
           std::make_unique<Host::WasiModule>();
       BuiltInModInsts.insert({HostRegistration::Wasi, std::move(WasiMod)});
     }
   }
   void VM::unsafeRegisterBuiltInHosts() {
     // Register all created WASI host modules.
     for (auto &It : BuiltInModInsts) {
       ExecutorEngine.registerModule(StoreRef, *(It.second.get()));
     }
   }
   
   void VM::unsafeRegisterPlugInHosts() {
     // Register all created module instances from plugins.
     for (auto &It : PlugInModInsts) {
       ExecutorEngine.registerModule(StoreRef, *(It.get()));
     }
   ```

   初始化 WasiModule: 本质上 WasiModule 就是 ModuleInstance, 相比于 ModuleInstance, WasiModule初始化过程仅仅是多了一些 **addHostFunc** 调用. **addHostFunc** 把 func instance 加入到成员变量里 (Wasi的所有函数都已经声明且定义好了, 每一个 WasiFunction 都是一个 HostFunction)
   Wasi Function 靠什么工作? 通过传入的 **Runtime**::**CallingFrame** 实例以及 WasiModule 的成员 Env (一个 WasmEdge::Host::Wasi::Environ 实例)
   Load Module 之后就是 Register Module, 通过 Store Manager 做一些工作, 有待研究.
   从上述代码还可以发现, BuiltInHosts 和 PlugInHost 没有很大区别, 尤其是在 Register 的时候, 两者都被视为 ModuleInstance. 这也说明: 只要你继承 ModuleInstance 开发一个插件, 如果实现了和 Wasi 相同的功能, 则无需考虑其他任何问题, 只需要在 unsafeLoadPlugInHosts() 中加一行自己的插件, 便实现了 Wasi. (实际操作起来可能没那么简单.)

4. VM 初始化之后, 回到 Tool 函数中, 接下来需要经历几个过程, 分别是 loadWasm, validate, instantiate. 这几个过程都是针对输入的 .wasm 文件所做的工作. 值得注意的是, 输入的 .wasm 文件 (模块) 被加载成 AST::Module 类型, 对应的实例存储在 VM 的成员函数 Mod 中. validate 和 instantiate 都是对该 Mod 进行操作, 具体细节暂时先不管. 

5. 在运行 .wasm 文件之前, 还会执行 Wasi Env 的初始化工作 (其实就是从 ToolsOptions 中拿数据组装一下). 然后区分两种模式, 一种 Command Mode (未指定开始的模块名, 默认去寻找`_start`函数), 一种 Reactor Mode (指定了开始的模块名), 然后异步执行, 得到结果.

6. 在异步执行之前, 将断点加在 WasmEdge::Host::WasiFdWrite::body, 然后就能看到 Wasi Function 是如何被执行的, 以及执行时刻的调用栈.

需要继续探索的是: 插件实现 wasi 需要了解 Store Manager 的细节吗? 





Ref

1. 断点列表, 文件 breakpoint_list_wasmedge.lldb. 在 lldb 中执行 run 之前, 执行 `breakpoint read -f  breakpoint_list_wasmedge.lldb` 获取.