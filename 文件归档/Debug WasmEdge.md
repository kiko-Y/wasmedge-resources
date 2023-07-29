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
   cmake -Bbuild -GNinja -DCMAKE_BUILD_TYPE=Debug -DWASMEDGE_BUILD_TESTS=OFF -DWASMEDGE_PLUGIN_WASI_CRYPTO=On -DWASMEDGE_PLUGIN_WASI_LOGGING=ON -DWASMEDGE_PLUGIN_TENSORFLOW=On .
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



加载 Built-in Module 和 Plugin-in Module 一样吗? 答案是不一样, 如下是一些线索

1. 在 vm.cpp 的 unsafeInit() 中调用了 unsafeLoadPlugInHosts(), 在 unsafeLoadPlugInHosts() 中, 处理每一个插件的代码形如
   ```C++
   void VM::unsafeLoadPlugInHosts() {
     // Load the plugins and mock them if not found.
     PlugInModInsts.push_back(createPluginModule<Host::WasiNNModuleMock>("wasi_nn"sv, "wasi_nn"sv));
     PlugInModInsts.push_back(createPluginModule<Host::WasiCryptoCommonModuleMock>(
         "wasi_crypto"sv, "wasi_crypto_common"sv));
   }
   ```

2. createPluginModule 如下:
   该函数被套在一个匿名namespace中, 很显然是不想让外部文件访问
   该函数有两个返回路径, 一个是寻找插件->寻找模块->构建模块并返回, 一个是用模版类型创建一个实例返回
   很显然前者是在你已经安装的插件库中寻找, 而后者使用 Mock Module 来代替, Mock Module 的功能就是打印一条插件没安装日志

   ```cpp
   namespace {
   template <typename T>
   std::unique_ptr<Runtime::Instance::ModuleInstance>
   createPluginModule(std::string_view PName, std::string_view MName) {
     using namespace std::literals::string_view_literals;
     if (const auto *Plugin = Plugin::Plugin::find(PName)) {
       if (const auto *Module = Plugin->findModule(MName)) {
         return Module->create();
       }
     }
     spdlog::debug("Plugin: {} , module name: {} not found. Mock instead."sv,
                   PName, MName);
     return std::make_unique<T>();
   }
   } // namespace
   
   // function impl in mock module
   inline void printPluginMock(std::string_view PluginName) {
     spdlog::error("{} plugin not installed. Please install the plugin and "
                   "restart WasmEdge.",
                   PluginName);
   }
   ```

3. 插件的最终形式? 暂时不用管, 只要插件开发出来了, 就能立刻使用 (至少在 Command Line Tool 中是如此, SDK 暂时还没搞明白)!!
   为什么?
   因为 `cargo build --target wasm32-wasi --release` 得到的 .wasm 文件, 已经把输入输出函数转换成了 `fd_read()` 和 `fd_write()` 函数, 在执行 `wasmedge hello.wasm` 时, 只要 VM 的某个模块有这两个函数就行, 而这两个函数完全可以来自 Plugin-In Module, 如果该 Plugin-In 是自动安装的, 那么对命令行用户来说甚至不知道我们做了这种改变, 是完全透明的操作.

4. 为了研究编译后的插件是如何被加载到 VM 的, 重新编译 wasmedge, 安装插件.
   ```shell
   cmake -Bbuild -GNinja -DCMAKE_BUILD_TYPE=Debug -DWASMEDGE_BUILD_TESTS=OFF -DWASMEDGE_PLUGIN_WASI_CRYPTO=On -DWASMEDGE_PLUGIN_WASI_LOGGING=ON -DWASMEDGE_PLUGIN_TENSORFLOW=On .
   ```

   插件会被编译成动态库, 比如`build/plugins/wasi_crypto/libwasmedgePluginWasiCrypto.dylib` (能不能改成静态库? 这样方便调试)
   到目前为止, 生成的文件都在 build 目录中, 系统路径(包括 `/usr/local`) 中没有任何关于 wasmedge 的信息 (我也没安装 wasmedge)
   安装 wasmedge, 执行 `cmake --install build`, 以后调试时, 使用 `lldb /usr/local/bin/wasmedge hello.wasm` 即可加载插件

5. 编译后的插件被加载到 VM:
   调用栈 unsafeInit -> unsafeLoadPlugInHosts -> createPluginModule
   在 createPluginModule() 中, 调用了类 Plugin 的静态成员函数 find, 该类中还有两个静态成员变量, 分别是

   ```cpp
   static std::vector<Plugin> &PluginRegistory;
   static std::unordered_map<std::string_view, std::size_t> &PluginNameLookup;
   ```

   每一个插件, 都会在进入 uniTool 之前被构造一个类 Plugin 的实例, 放在 PluginRegistory 中. 而 PluginNameLookup 用于查找某个插件在 PluginRegistory 中的下标, 下面两段代码块解释了他们如何工作的:

   ```cpp
   (lldb) p PluginNameLookup
   (std::unordered_map<std::basic_string_view<char, std::char_traits<char> >, unsigned long, std::hash<std::basic_string_view<char, std::char_traits<char> > >, std::equal_to<std::basic_string_view<char, std::char_traits<char> > >, std::allocator<std::pair<const std::basic_string_view<char, std::char_traits<char> >, unsigned long> > >) $0 = size=2 {
     [0] = {
       __cc = (first = "wasi_crypto", second = 1)
     }
     [1] = {
       __cc = (first = "wasi_logging", second = 0)
     }
   }
   ```

   如下是一个可能的 PluginResigtory, 包括了两个插件. Plugin 中 ModuleRegistory 最重要, 其声明为 `  std::vector<PluginModule> ModuleRegistory;`, 表明其是存储了该插件相关的所有 Module.

   ```cpp
   (std::vector<WasmEdge::Plugin::Plugin, std::allocator<WasmEdge::Plugin::Plugin> >) $1 = size=2 {
     [0] = {
       Path = (__pn_ = "/usr/local/lib/wasmedge/libwasmedgePluginWasiLogging.dylib")
       Desc = 0x00000001058ac030
       Lib = std::__1::shared_ptr<WasmEdge::Loader::SharedLibrary>::element_type @ 0x0000600002c04098 strong=1 weak=2 {
         __ptr_ = 0x0000600002c04098
       }
       ModuleRegistory = size=1 {
         [0] = {
           Desc = 0x00000001058ac070
         }
       }
       ModuleNameLookup = size=1 {
         [0] = {
           __cc = (first = "wasi:logging/logging", second = 0)
         }
       }
     }
     [1] = {
       Path = (__pn_ = "/usr/local/lib/wasmedge/libwasmedgePluginWasiCrypto.dylib")
       Desc = 0x00000001156ae000
       Lib = std::__1::shared_ptr<WasmEdge::Loader::SharedLibrary>::element_type @ 0x0000600002c04118 strong=1 weak=2 {
         __ptr_ = 0x0000600002c04118
       }
       ModuleRegistory = size=5 {
         [0] = {
           Desc = 0x00000001156ae040
         }
         [1] = {
           Desc = 0x00000001156ae058
         }
         [2] = {
           Desc = 0x00000001156ae070
         }
         [3] = {
           Desc = 0x00000001156ae088
         }
         [4] = {
           Desc = 0x00000001156ae0a0
         }
       }
       ModuleNameLookup = size=5 {
         [0] = {
           __cc = (first = "wasi_crypto_symmetric", second = 4)
         }
         [1] = {
           __cc = (first = "wasi_crypto_kx", second = 2)
         }
         [2] = {
           __cc = (first = "wasi_crypto_signatures", second = 3)
         }
         [3] = {
           __cc = (first = "wasi_crypto_common", second = 1)
         }
         [4] = {
           __cc = (first = "wasi_crypto_asymmetric_common", second = 0)
         }
       }
     }
   }
   
   ```

   一个 PluginModule 只有一个 ModuleDescriptor 的成员变量, 其中 Create 是一个函数指针. Create 需要指向的函数在插件编写时, 就已经定义好了, 比如下面的 ctx.cpp 文件.
   ```c++
   // include/plugin/plugin.h
   struct ModuleDescriptor {
     const char *Name;
     const char *Description;
     Runtime::Instance::ModuleInstance *(*Create)(
         const ModuleDescriptor *) noexcept;
   };
   ```

   ```cpp
   // plugins/wasi_crypto/ctx.cpp
   Runtime::Instance::ModuleInstance *createAsymmetricCommon(
       const Plugin::PluginModule::ModuleDescriptor *) noexcept {
     return new WasiCryptoAsymmetricCommonModule(
         WasiCrypto::Context::getInstance());
   }
   ```

   createPluginModule 最后得到 Module, 这个 Module 是一个ModuleDescriptor的实例, 然后调用 Module->Create() 来创建 ModuleInstance 并返回指针.
   根据上面的代码已经知道, Module->Create 最终调用了上述代码块中的 `WasiCryptoAsymmetricCommonModule` 构造函数, 该构造函数, 位于 `plugins/wasi_crypto/common/module.cpp`. 该构造函数和 WasiModule 的构造函数如出一辙, 只不过前者使用 Ctx, 后者使用 Env, 两者都在构造函数中调用了很多 addHostFunc(), 将自己提供的宿主函数加入到 VM 中.

6. 如何使用插件? 如何在 Command Line 模式下使用已有插件, 使用自定义插件? 
   除了该问题, 我们已经知道了插件和 Wasi 的大概框架, 接下来我应该陷入细节. 包括: 1. Args_size_get 是如何声明(包括参数返回值), 定义, 实现的. 2. 插件是如何被加载的, Ctx 如何帮助插件实现功能.
   最后在切换到 Rust, 直接使用 SDK 进行开发.

### args_size_get 是如何声明的

一个 Host Function 应该如何声明才能正常工作? 如果你完全理解了 Host Function 从注册到工作到销毁的流程, 则你可以按你自己喜欢的方式设计并声明. 然而并不是每个人都想这么做, 庆幸的是类 ` class WasmEdge::Runtime::HostFunction<T>` 已经帮我们做好了部分工作. 通过查看该类的源代码发现, 如果复用其工作, 只需要将你的 Host Function 定义为类, 然后定义一个名为 body 的成员函数, 在 body 中实现真正的函数逻辑, 最后用你的类继承 `Runtime::HostFunction<T>`, 类名传给模版参数 T (因为在 HostFunction 的成员函数中, 其使用 T::body 做很多工作), 最后调用 addHostFunc 即大功告成.

在现有的 wasi 设计中, 每个 wasi function 很显然都需要一个 Env 才能正常工作, 所有的 wasi function 可以共享一个 Env, 为此, 可以开一个 Wasi 类, 存一个 Env 的引用, 每个 wasi function 都继承 Wasi, 然后 Wasi 继承 `Runtime::HostFunction<T>` 即可.





## ToDo

1. 插件中的 Module 和 Built-In Module 有什么区别? 
2. 需要继续探索的是: 插件实现 wasi 需要了解 Store Manager 的细节吗?

 





Ref

1. 断点列表, 文件 breakpoint_list_wasmedge.lldb. 在 lldb 中执行 run 之前, 执行 `breakpoint read -f  breakpoint_list_wasmedge.lldb` 获取.