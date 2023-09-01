# WasmEdge 调试记录

## 1. lldb 学习使用

以下记录了一些常会使用到的命令：

### 1.1. 程序运行相关

run ( r )，运行 debug 程序，或者重新运行

step ( s )，单步执行程序，会进入函数

next ( n )，单步执行程序，不会进入函数

continue ( c )，继续执行程序，到下一个断点

thread return，跳出当前的函数，到上一级函数中

### 1.2. 断点相关

breakpoint set -n \<func_name\>，在某个函数入口处设置断点

breakpoint set -f \<file_mame\> -l \<line_number\>，在某个文件的某行设置断点

breakpoint list，查看所有断点列表

breakpoint delete \<breakpoint_id\>，删除某个断点 s

breakpoint write -f \<file_name\>，保存断点信息到文件

breakpoint read -f \<file_name\>， 从文件读入断点信息

### 1.3. 信息查看相关

frame select ( f )，查看当前栈帧的上下文

frame variable，查看当前栈帧中的变量

p/po，输出一个变量值

## 2. 调试过程



调试执行 wasmedge ./hello.wasm

1. 首先进入主函数，调用 WasmEdge_Driver_UniTool(Argc, Argv)

   ```cpp
   int main(int Argc, const char *Argv[]) {
   		return WasmEdge_Driver_UniTool(Argc, Argv);
   }
   ```

2. 再进入到 UniTool 中

   ```cpp
   WASMEDGE_CAPI_EXPORT int WasmEdge_Driver_UniTool(int Argc, const char *Argv[]) {
   	  return WasmEdge::Driver::UniTool(Argc, Argv, WasmEdge::Driver::ToolType::All);
   }
   ```

3. UniTool 进行了参数解析，放到了 Option 对象中：

   ```cpp
   // uniTool.cpp
   ...
   if (ToolSelect == ToolType::All) {
     	// Options 中进行添加，继续往下看
       ToolOptions.add_option(Parser);
   
       Parser.begin_subcommand(CompilerSubCommand, "compile"sv);
       CompilerOptions.add_option(Parser);
       Parser.end_subcommand();
   
       Parser.begin_subcommand(ToolSubCommand, "run"sv);
       ToolOptions.add_option(Parser);
       Parser.end_subcommand();
     }
   ```

   

   ```cpp
   void add_option(PO::ArgumentParser &Parser) noexcept {
   
       Parser.add_option(SoName)
           .add_option(Args)
           .add_option("reactor"sv, Reactor)
         
           ...
         
           .add_option("forbidden-plugin"sv, ForbiddenPlugins);
   
     	// 获取插件路径，继续看如何获取的插件路径 1
       for (const auto &Path : Plugin::Plugin::getDefaultPluginPaths()) {
         // 看如何加载的 2
         Plugin::Plugin::load(Path);
       }
       Plugin::Plugin::addPluginOptions(Parser);
     }
   };
   ```

   1. 获取插件路径

      ```cpp
      std::vector<std::filesystem::path> Plugin::getDefaultPluginPaths() noexcept {
        using namespace std::literals::string_view_literals;
        std::vector<std::filesystem::path> Result;
        std::error_code Error;
      
        // Extra directories from environ variable
        // 首先获取系统环境变量中配置的路径
        if (const auto ExtraEnv = ::getenv("WASMEDGE_PLUGIN_PATH")) {
          std::string_view ExtraEnvStr = ExtraEnv;
          for (auto Sep = ExtraEnvStr.find(':'); Sep != std::string_view::npos;
               Sep = ExtraEnvStr.find(':')) {
            Result.push_back(std::filesystem::u8path(ExtraEnvStr.substr(0, Sep)));
            const auto Next = ExtraEnvStr.find_first_not_of(':', Sep);
            ExtraEnvStr = ExtraEnvStr.substr(Next);
          }
          Result.push_back(std::filesystem::u8path(ExtraEnvStr));
        }
      #if WASMEDGE_OS_LINUX || WASMEDGE_OS_MACOS
        Dl_info DLInfo;
        int Status =
            dladdr(reinterpret_cast<void *>(Plugin::getDefaultPluginPaths), &DLInfo);
        if (Status != 0) {
          auto LibPath = std::filesystem::u8path(DLInfo.dli_fname)
                             .parent_path()
                             .lexically_normal();
          // debug 得到 LibPath 是 "/Users/kikoshi/kiko-projects/wasm/wasmedge/WasmEdge/build/lib/api"
          const auto UsrStr = "/usr"sv;
          const auto LibStr = "/lib"sv;
          const auto &PathStr = LibPath.native();
          if ((PathStr.size() >= UsrStr.size() &&
               std::equal(UsrStr.begin(), UsrStr.end(), PathStr.begin())) ||
              (PathStr.size() >= LibStr.size() &&
               std::equal(LibStr.begin(), LibStr.end(), PathStr.begin()))) {
            // The installation path of the WasmEdge library is under "/usr".
            // Plug-in path will be in "LIB_PATH/wasmedge".
            // If the installation path is under "/usr/lib" or "/usr/lib64", the
            // traced library path will be "/lib" or "/lib64".
            Result.push_back(LibPath / std::filesystem::u8path("wasmedge"sv));
          } else {
            // The installation path of the WasmEdge library is not under "/usr", such
            // as "$HOME/.wasmedge". Plug-in path will be in "LIB_PATH/../plugin".
            Result.push_back(LibPath / std::filesystem::u8path(".."sv) /
                             std::filesystem::u8path("plugin"sv));
          }
        }
      #endif
        return Result;
      }
      ```

   2. 从对应的路径加载插件

      ```cpp
      Plugin::load(const std::filesystem::path &Path) noexcept {
        std::error_code Error;
        auto Status = std::filesystem::status(Path, Error);
        if (likely(!Error)) {
          if (std::filesystem::is_directory(Status)) {
      
            bool Result = false;
            // /Users/kikoshi/kiko-projects/wasm/wasmedge/WasmEdge/build/lib/api/../plugin/ 路径下找插件
            for (const auto &Entry : std::filesystem::recursive_directory_iterator(
                     Path, std::filesystem::directory_options::skip_permission_denied,
                     Error)) {
              const auto &EntryPath = Entry.path();
             // 这里 WASMEDGE_LIB_EXTENSION = .dylib，找插件库的扩展名
              if (Entry.is_regular_file(Error) &&
                  EntryPath.extension().u8string() == WASMEDGE_LIB_EXTENSION) {
                // 找到插件之后把插件加载进来
                Result |= loadFile(EntryPath);
              }
            }
            return Result;
          } else if (std::filesystem::is_regular_file(Status) &&
                     Path.extension().u8string() == WASMEDGE_LIB_EXTENSION) {
            return loadFile(Path);
          }
        }
        return false;
      }
      ```

4. 在 UniTool 中进行了参数解析之后进入了 Tool 方法中，进行配置的加载和移除，配置用于初始化 VM 的时候使用

   ```cpp
   // runtimeTool.cpp
   int Tool(struct DriverToolOptions &Opt) noexcept {
     using namespace std::literals;
   
    ...
     // 各种配置的移除和添加
     Configure Conf;
     if (Opt.PropMutGlobals.value()) {
       Conf.removeProposal(Proposal::ImportExportMutGlobals);
     }
     if (Opt.PropNonTrapF2IConvs.value()) {
       Conf.removeProposal(Proposal::NonTrapFloatToIntConversions);
     }
     ... 
   	// 配置内存页大小
     if (Opt.MemLim.value().size() > 0) {
       Conf.getRuntimeConfigure().setMaxMemoryPage(
           static_cast<uint32_t>(Opt.MemLim.value().back()));
     }
   	...
   	// 设置禁用的插件
     for (const auto &Name : Opt.ForbiddenPlugins.value()) {
       Conf.addForbiddenPlugins(Name);
     }
   	// 添加 WASI 的 HostRegistration，（即开启了 WASI 支持）
     // API 文档里写的是目前只支持 WASI 的，之后会有别的 built-in host-function 支持
     Conf.addHostRegistration(HostRegistration::Wasi);
    	// InputPath 就是输入的 .wasm 文件路径 hello.wasm
     const auto InputPath =
         std::filesystem::absolute(std::filesystem::u8path(Opt.SoName.value()));
     // 根据配置构建 VM （见下面的源码）
     VM::VM VM(Conf);
   	// 获取导入的 Wasi Module
     Host::WasiModule *WasiMod = dynamic_cast<Host::WasiModule *>(
         VM.getImportModule(HostRegistration::Wasi));
   ```

   ```cpp
   // vm.cpp
   // VM 构造器
   VM::VM(const Configure &Conf)
       : Conf(Conf), Stage(VMStage::Inited),
         LoaderEngine(Conf, &Executor::Executor::Intrinsics),
         ValidatorEngine(Conf), ExecutorEngine(Conf, &Stat),
         Store(std::make_unique<Runtime::StoreManager>()), StoreRef(*Store.get()) {
     unsafeInitVM();
   }
   ```

   ```cpp
   // vm.cpp
   // 初始化
   void VM::unsafeInitVM() {
     // Load the built-in modules and the plug-ins.
     // 加载内置的模块和插件
     unsafeLoadBuiltInHosts();
     unsafeLoadPlugInHosts();
   
     // Register all module instances.
     // 注册所有的 module instance
     unsafeRegisterBuiltInHosts();
     unsafeRegisterPlugInHosts();
   }
   ```

   1.   unsafeLoadBuiltInHosts();

      ```cpp
      // vm.cpp
      void VM::unsafeLoadBuiltInHosts() {
        // Load the built-in host modules from configuration.
        // TODO: This will be extended for the versionlized WASI in the future.
        // BuiltInModInsts 是一个 unordered_map 用于映射 ModuleInstance 的地址
        // 是否注册了 WASI(之前注册过了)，做一个映射  HostRegistration -> ModuleInstance address
        BuiltInModInsts.clear();
        if (Conf.hasHostRegistration(HostRegistration::Wasi)) {
          // 下面可以看一下 Wasi Module 初始化的时候做了什么
          std::unique_ptr<Runtime::Instance::ModuleInstance> WasiMod =
              std::make_unique<Host::WasiModule>();
          BuiltInModInsts.insert({HostRegistration::Wasi, std::move(WasiMod)});
        }
      }
      ```

      - WasiModule 的初始化

        ```cpp
        // wasimodule.cpp
        // 
        WasiModule::WasiModule() : ModuleInstance("wasi_snapshot_preview1") {
          // 看如何加入一个 HostFunc
          // WasiArgsGet 是一个 HostFunctionBase，包含了方法的类型，入参出参
          addHostFunc("args_get", std::make_unique<WasiArgsGet>(Env));
          ...
          addHostFunc("fd_read", std::make_unique<WasiFdRead>(Env));
        	...
          addHostFunc("sock_getaddrinfo", std::make_unique<WasiSockGetAddrinfo>(Env));
        }
        
        ```

        ```cpp
        // 主体功能函数
        Expect<uint32_t> WasiArgsGet::body(const Runtime::CallingFrame &Frame,
                                           uint32_t ArgvPtr, uint32_t ArgvBufPtr) {
          // Check memory instance from module.
          auto *MemInst = Frame.getMemoryByIndex(0);
          if (MemInst == nullptr) {
            return __WASI_ERRNO_FAULT;
          }
        
          // Store **Argv.
          const auto &Arguments = Env.getArguments();
          const uint32_t ArgvSize = static_cast<uint32_t>(Arguments.size());
          const uint32_t ArgvBufSize = calculateBufferSize(Arguments);
        
          // Check for invalid address.
          const auto Argv = MemInst->getSpan<uint8_t_ptr>(ArgvPtr, ArgvSize);
          if (unlikely(Argv.size() != ArgvSize)) {
            return __WASI_ERRNO_FAULT;
          }
          const auto ArgvBuf = MemInst->getSpan<uint8_t>(ArgvBufPtr, ArgvBufSize);
          if (unlikely(ArgvBuf.size() != ArgvBufSize)) {
            return __WASI_ERRNO_FAULT;
          }
        
          if (!Argv.empty()) {
            Argv[0] = ArgvBufPtr;
          }
        
          if (auto Res = Env.argsGet(Argv, ArgvBuf); unlikely(!Res)) {
            return Res.error();
          }
        
          return __WASI_ERRNO_SUCCESS;
        }
        ```

        

        ```cpp
        // module.cpp
        void addHostFunc(std::string_view Name,
                         std::unique_ptr<HostFunctionBase> &&Func) {
          std::unique_lock Lock(Mutex);
          unsafeAddHostInstance(Name, OwnedFuncInsts, FuncInsts, ExpFuncs,
                                std::make_unique<Runtime::Instance::FunctionInstance>(
                                    this, std::move(Func)));
        }
        ```

        ```cpp
        // module.cpp
        template <typename T, typename... Args>
        std::enable_if_t<IsEntityV<T>, void>
        unsafeAddHostInstance(std::string_view Name,
                              std::vector<std::unique_ptr<T>> &OwnedInstsVec,
                              std::vector<T *> &InstsVec,
                              std::map<std::string, T *, std::less<>> &InstsMap,
                              std::unique_ptr<T> &&Inst) {
          OwnedInstsVec.push_back(std::move(Inst));
          InstsVec.push_back(OwnedInstsVec.back().get());
          InstsMap.insert_or_assign(std::string(Name), InstsVec.back());
        }
        ```

        

        

   2. unsafeLoadPlugInHosts();

      ```cpp
      // vm.cpp
      void VM::unsafeLoadPlugInHosts() {
        // Load the plugins and mock them if not found.
        // PlugInModInsts 是一个 vector，插件的 Module instance 都在这里
        using namespace std::literals::string_view_literals;
        PlugInModInsts.clear();
      	// 导入官方的插件， 创建了 Mock 版本(调试发现创建了空指针(可能是因为在 mac 上进行的 debug，没有安装插件))  createPluginModule(插件名, 模块名)
        PlugInModInsts.push_back(
            createPluginModule<Host::WasiNNModuleMock>("wasi_nn"sv, "wasi_nn"sv));
      	...
        PlugInModInsts.push_back(createPluginModule<Host::WasmEdgeImageModuleMock>(
            "wasmedge_image"sv, "wasmedge_image"sv));
        
        // 导入非官方的插件(debug 过程中没有其他的插件加入)
        // Load the other non-official plugins.
        for (const auto &Plugin : Plugin::Plugin::plugins()) {
          if (Conf.isForbiddenPlugins(Plugin.name())) {
            continue;
          }
          // Skip wasi_crypto, wasi_nn, wasi_logging, WasmEdge_Process,
          // WasmEdge_Tensorflow, WasmEdge_TensorflowLite, and WasmEdge_Image.
          if (Plugin.name() == "wasi_crypto"sv || Plugin.name() == "wasi_nn"sv ||
              ...
              Plugin.name() == "wasmedge_image"sv) {
            continue;
          }
          // 导入插件的所有模块
          for (const auto &Module : Plugin.modules()) {
            PlugInModInsts.push_back(Module.create());
          }
        }
      }
      ```
      

   3. unsafeRegisterBuiltInHosts();

      ```cpp
      // vm.cpp
      // wasi module 注册到执行引擎中
      void VM::unsafeRegisterBuiltInHosts() {
        // Register all created WASI host modules.
        for (auto &It : BuiltInModInsts) {
          ExecutorEngine.registerModule(StoreRef, *(It.second.get()));
        }
      }
      ```

   4. unsafeRegisterPlugInHosts();

      ```cpp
      // vm.cpp
      // 注册所有的 plugins
      void VM::unsafeRegisterPlugInHosts() {
        // Register all created module instances from plugins.
        for (auto &It : PlugInModInsts) {
          ExecutorEngine.registerModule(StoreRef, *(It.get()));
        }
      }
      ```

   主要两步：Load/Registoer BuiltInHosts会加载内置的 Wasi 模块，Load/Registoer PlugInHosts会加载插件，包括官方插件和所有非官方的插件。具体非官方插件是如何给到 WasmEdge 的目前没有找到具体的代码，需要后续继续 debug 来找到。

   目前的猜测是在开始的从默认路径加载的部分得到的，如果是这样的话所有的插件都应该放在某个固定的插件库中，或者设置插件位置的环境变量（WASMEDGE_PLUGIN_PATH）来定义插件的位置。

   

   

   ```cpp
   // runtimeTool.cpp
   // VM 加载 验证 实例化 wasm 模块
   if (auto Result = VM.loadWasm(InputPath.u8string()); !Result) {
       return EXIT_FAILURE;
     }
     if (auto Result = VM.validate(); !Result) {
       return EXIT_FAILURE;
     }
     if (auto Result = VM.instantiate(); !Result) {
       return EXIT_FAILURE;
     }
   ```

   1. unsafeLoadWasm();

      ```cpp
      // 从路径加载 .wasm 模块，加载到内存中变成 AST:Module 的数据结构 
      Expect<void> VM::unsafeLoadWasm(const std::filesystem::path &Path) {
        // If not load successfully, the previous status will be reserved.
        if (auto Res = LoaderEngine.parseModule(Path)) {
          Mod = std::move(*Res);
          Stage = VMStage::Loaded;
        } else {
          return Unexpect(Res);
        }
        return {};
      }
      ```

   2. unsafeValidate();

      ```cpp
      // 做 AST:Module 模块的验证
      Expect<void> VM::unsafeValidate() {
        if (Stage < VMStage::Loaded) {
          // When module is not loaded, not validate.
          spdlog::error(ErrCode::Value::WrongVMWorkflow);
          return Unexpect(ErrCode::Value::WrongVMWorkflow);
        }
        if (auto Res = ValidatorEngine.validate(*Mod.get())) {
          Stage = VMStage::Validated;
          return {};
        } else {
          return Unexpect(Res);
        }
      }
      
      ```

   3. unsafeInstantiate();

      ```cpp
      // 实例化 wasm 模块
      Expect<void> VM::unsafeInstantiate() {
        if (Stage < VMStage::Validated) {
          // When module is not validated, not instantiate.
          spdlog::error(ErrCode::Value::WrongVMWorkflow);
          return Unexpect(ErrCode::Value::WrongVMWorkflow);
        }
        // 执行引擎进行实例化模块
        // 实例化先实例化 ModuleInstance，然后是 Function, Table, Memory, Export 等ModuleInstance 的内置模块
        if (auto Res = ExecutorEngine.instantiateModule(StoreRef, *Mod.get())) {
          Stage = VMStage::Instantiated;
          ActiveModInst = std::move(*Res);
          return {};
        } else {
          return Unexpect(Res);
        }
      }
      
      ```

   

   ```cpp
   // runtimeTool.cpp
   // 初始化环境
   WasiMod->getEnv().init(
       Opt.Dir.value(),
       InputPath.filename()
           .replace_extension(std::filesystem::u8path("wasm"sv))
           .u8string(),
       Opt.Args.value(), Opt.Env.value());
   ```
   
   
   
   之后的文件执行应该是一个异步的操作，目前还没有 debug 到，需要后续继续深挖一下。















## 调试 v2

内置的 wasi 是如何加载的

```cpp
class VM {
public:
  VM() = delete;
  // VM 初始化
  VM(const Configure &Conf);
	...
}
```

```cpp
VM::VM(const Configure &Conf)
    : Conf(Conf), Stage(VMStage::Inited),
      LoaderEngine(Conf, &Executor::Executor::Intrinsics),
      ValidatorEngine(Conf), ExecutorEngine(Conf, &Stat),
      Store(std::make_unique<Runtime::StoreManager>()), StoreRef(*Store.get()) {
	// 初始化 VM
  unsafeInitVM();
}
```

```cpp
void VM::unsafeInitVM() {
  // Load the built-in modules and the plug-ins.
  // 加载 WASI
  unsafeLoadBuiltInHosts();
  unsafeLoadPlugInHosts();

  // Register all module instances.
  unsafeRegisterBuiltInHosts();
  unsafeRegisterPlugInHosts();
}
```

```cpp
void VM::unsafeLoadBuiltInHosts() {
  // Load the built-in host modules from configuration.
  // TODO: This will be extended for the versionlized WASI in the future.
  BuiltInModInsts.clear();
  if (Conf.hasHostRegistration(HostRegistration::Wasi)) {
    // WasiModule 进行初始化
    std::unique_ptr<Runtime::Instance::ModuleInstance> WasiMod =
        std::make_unique<Host::WasiModule>();
    BuiltInModInsts.insert({HostRegistration::Wasi, std::move(WasiMod)});
  }
}
```

```cpp
// WasiModule 继承了 ModuleInstance
// Wasimodule.h
class WasiModule : public Runtime::Instance::ModuleInstance {
public:
  WasiModule();

  WASI::Environ &getEnv() noexcept { return Env; }
  const WASI::Environ &getEnv() const noexcept { return Env; }

private:
  // WasiModule 里面包含了一个 Environ，供后面的 Wasi HostFunction 使用
  WASI::Environ Env;
};


// Wasimodule.cpp
// 具体的初始化方法，加入了很多的 HostFunc，如何 addHostFunc 的
WasiModule::WasiModule() : ModuleInstance("wasi_snapshot_preview1") {
  // 用 Env 初始化 WasiArgsGet，看 WasiArgsGet 的初始化
  addHostFunc("args_get", std::make_unique<WasiArgsGet>(Env));
  ...
}
```

如何 addHostFunc：

```cpp
// module.h
// Name 就是上面的 "args_get" Func 就是上面的 WasiArgsGet
void addHostFunc(std::string_view Name,
                 std::unique_ptr<HostFunctionBase> &&Func) {
  std::unique_lock Lock(Mutex);
  unsafeAddHostInstance(Name, OwnedFuncInsts, FuncInsts, ExpFuncs,
                        std::make_unique<Runtime::Instance::FunctionInstance>(
                            this, std::move(Func)));
}




/// Unsafe add and export the existing instance into this module.
template <typename T, typename... Args>
std::enable_if_t<IsEntityV<T>, void>
unsafeAddHostInstance(std::string_view Name,
                      std::vector<std::unique_ptr<T>> &OwnedInstsVec,
                      std::vector<T *> &InstsVec,
                      std::map<std::string, T *, std::less<>> &InstsMap,
                      std::unique_ptr<T> &&Inst) {
  // 存的是 FunctionInstance
  OwnedInstsVec.push_back(std::move(Inst));
  // 存的指针
  InstsVec.push_back(OwnedInstsVec.back().get());
  // 存的方法名到 FunctionInstance 的映射
  InstsMap.insert_or_assign(std::string(Name), InstsVec.back());
}
```



看一下 WasiArgsGet：

继承了 Wasi，每个 Wasi 类都包含一个 body 方法，实际上就是这个 wasi 的主方法

```cpp
// wasifunc.h
class WasiArgsGet : public Wasi<WasiArgsGet> {
public:
  WasiArgsGet(WASI::Environ &HostEnv) : Wasi(HostEnv) {}

  Expect<uint32_t> body(const Runtime::CallingFrame &Frame, uint32_t ArgvPtr,
                        uint32_t ArgvBufPtr);
};
```

Wasi 又继承了 HostFunction，且每一个 Wasi 都包含一个 Environ 的引用

```cpp
template <typename T> class Wasi : public Runtime::HostFunction<T> {
public:
  Wasi(WASI::Environ &HostEnv) : Runtime::HostFunction<T>(0), Env(HostEnv) {}

protected:
  WASI::Environ &Env;
};
```

WasiArgsGet 的  body 方法

```cpp
Expect<uint32_t> WasiArgsGet::body(const Runtime::CallingFrame &Frame,
                                   uint32_t ArgvPtr, uint32_t ArgvBufPtr) {
  // Check memory instance from module.
  auto *MemInst = Frame.getMemoryByIndex(0);
  if (MemInst == nullptr) {
    return __WASI_ERRNO_FAULT;
  }

  // Store **Argv.
  /*												这里调用了 Env.getArguments													*/
  const auto &Arguments = Env.getArguments();
  const uint32_t ArgvSize = static_cast<uint32_t>(Arguments.size());
  const uint32_t ArgvBufSize = calculateBufferSize(Arguments);

  // 这里获取了 MemoryInstance 中的一段空间，用于保存返回值
  // Check for invalid address.
  const auto Argv = MemInst->getSpan<uint8_t_ptr>(ArgvPtr, ArgvSize);
  if (unlikely(Argv.size() != ArgvSize)) {
    return __WASI_ERRNO_FAULT;
  }
  const auto ArgvBuf = MemInst->getSpan<uint8_t>(ArgvBufPtr, ArgvBufSize);
  if (unlikely(ArgvBuf.size() != ArgvBufSize)) {
    return __WASI_ERRNO_FAULT;
  }

  if (!Argv.empty()) {
    Argv[0] = ArgvBufPtr;
  }
	// 这里Env.argsGet() 就是把结果放到了 MemoryInstance 里面去，作为返回值
  if (auto Res = Env.argsGet(Argv, ArgvBuf); unlikely(!Res)) {
    return Res.error();
  }

  return __WASI_ERRNO_SUCCESS;
}
```

再看一下其他的方法

比如获取参数数量：

```cpp
Expect<uint32_t> WasiArgsSizesGet::body(const Runtime::CallingFrame &Frame,
                                        uint32_t /* Out */ ArgcPtr,
                                        uint32_t /* Out */ ArgvBufSizePtr) {
  // Check memory instance from module.
  auto *MemInst = Frame.getMemoryByIndex(0);
  if (MemInst == nullptr) {
    return __WASI_ERRNO_FAULT;
  }

  // Check for invalid address.
  // 获取地址的指针，用于保存结果
  auto *const __restrict__ Argc = MemInst->getPointer<__wasi_size_t *>(ArgcPtr);
  if (unlikely(Argc == nullptr)) {
    return __WASI_ERRNO_FAULT;
  }
  auto *const __restrict__ ArgvBufSize =
      MemInst->getPointer<__wasi_size_t *>(ArgvBufSizePtr);
  if (unlikely(ArgvBufSize == nullptr)) {
    return __WASI_ERRNO_FAULT;
  }
	// 调用 Env.argsSizesGet 将参数保存到 MemoryInstance 这段空间去
  if (auto Res = Env.argsSizesGet(*Argc, *ArgvBufSize); unlikely(!Res)) {
    return Res.error();
  }
  return __WASI_ERRNO_SUCCESS;
}
```



比如关闭文件描述符：

```cpp
Expect<uint32_t> WasiFdClose::body(const Runtime::CallingFrame &, int32_t Fd) {
  const __wasi_fd_t WasiFd = Fd;

  // 这里调用了 Env.fdClose(WasiFd)
  if (auto Res = Env.fdClose(WasiFd); unlikely(!Res)) {
    return Res.error();
  }
  return __WASI_ERRNO_SUCCESS;
}
```



以下是 Environ 类

```cpp
// Environ.h
// 所有的 Wasi instance(host function)共享一个 Environ 变量
// 以下是 environ 类的部分方法
// environ 类提供了各种底层的方法

constexpr const std::vector<std::string> &getArguments() const noexcept {
  return Arguments;
}

constexpr const std::vector<std::string> &
getEnvironVariables() const noexcept {
  return EnvironVariables;
}

...
  
WasiExpect<void> argsGet(Span<uint8_t_ptr> Argv,
                         Span<uint8_t> ArgvBuffer) const noexcept {
  for (const auto &Argument : Arguments) {
    const __wasi_size_t Size = static_cast<__wasi_size_t>(Argument.size());
    std::copy_n(Argument.begin(), Size, ArgvBuffer.begin());
    ArgvBuffer[Size] = '\0';
    ArgvBuffer = ArgvBuffer.subspan(Size + UINT32_C(1));
    if (Argv.size() > 1) {
      Argv[1] = Argv[0] + Size + UINT32_C(1);
    }
    Argv = Argv.subspan(1);
  }
  assert(ArgvBuffer.empty());
  assert(Argv.empty());

  return {};
}

WasiExpect<void> argsSizesGet(__wasi_size_t &Argc,
                              __wasi_size_t &ArgvSize) const noexcept {
  Argc = static_cast<__wasi_size_t>(Arguments.size());
  ArgvSize = 0;
  for (const auto &Argument : Arguments) {
    ArgvSize += static_cast<__wasi_size_t>(Argument.size()) + UINT32_C(1);
  }

  return {};
}
```

理解了内置 Wasi 的工作方式，以及加载方式：

内置的 Wasi 模块默认直接加载进来，作为 WasiHostFunctionInstance，放在 WasiModuleInstance 里，并做了方法名和wasi 方法的映射，Wasi 方法体通过调用 Environ 里的各种方法来实现对应的功能。

实际上 Wasi 的 HostFunction 是作为一个系统环境和上层的一个中间层，封装底层功能为上层提供统一的接口。

各种插件是如何加载进 VM 并被调用的，目前只知道会从哪些路径下去找到插件库。