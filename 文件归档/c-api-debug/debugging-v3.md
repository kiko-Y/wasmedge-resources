## 调试 V3

### 加载插件

```cpp
// 进入所有的默认路径
// 1. 环境变量 WASMEDGE_PLUGIN_PATH 中定义的路径
// 2. wasmedge /lib/plugin/
for (const auto &Path : Plugin::Plugin::getDefaultPluginPaths()) {
  // 从路径下加载插件
  Plugin::Plugin::load(Path);
}
Plugin::Plugin::addPluginOptions(Parser);
```

```cpp
// 找到路径下的所有带有 WASMEDGE_LIB_EXTENSION 扩展的文件进行加载
// linux 下为 .so，mac 下为 .dylib，windows 下为 .ddl
// 如果是特定扩展的文件，则调用加载 loadFile

Plugin::load(const std::filesystem::path &Path) noexcept {
  std::error_code Error;
  auto Status = std::filesystem::status(Path, Error);
  if (likely(!Error)) {
    if (std::filesystem::is_directory(Status)) {

      bool Result = false;
      for (const auto &Entry : std::filesystem::recursive_directory_iterator(
               Path, std::filesystem::directory_options::skip_permission_denied,
               Error)) {
        const auto &EntryPath = Entry.path();
        if (Entry.is_regular_file(Error) &&
            EntryPath.extension().u8string() == WASMEDGE_LIB_EXTENSION) {
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

```cpp
// 1.  Lib->load(Path);
//  	Handle = ::dlopen(Path.c_str(), RTLD_LAZY | RTLD_LOCAL); 读入动态库文件
//		通过 dlopen 获取共享库的句柄 handle
// 2. 获取共享库中 WasmEdge_Plugin_GetDescriptor 的符号地址
// 3. 调用 WasmEdge_Plugin_GetDescriptor 获取 描述符 Descriptor
// 4. 通过描述符得到插件注册器
bool Plugin::loadFile(const std::filesystem::path &Path) noexcept {
  const auto Index = PluginRegistory.size();

  auto Lib = std::make_shared<Loader::SharedLibrary>();
  // load 内部调用 Handle = ::dlopen(Path.c_str(), RTLD_LAZY | RTLD_LOCAL); 获得共享库句柄
  if (auto Res = Lib->load(Path); unlikely(!Res)) {
    return false;
  }

  if (PluginRegistory.size() != Index + 1) {
    // Check C interface
    /**
    		// get 内部操作：获取共享库中 WasmEdge_Plugin_GetDescriptor 的符号地址
    	  template <typename T> Symbol<T> get(const char *Name) {
	        return Symbol<T>(shared_from_this(), reinterpret_cast<T *>(getSymbolAddr(Name)));
       }
       // getSymbolAddr(Name) 调用 dlsym(Handle, Name) 获取 Name 对应的符号地址
       // 得到获取插件描述符方法的地址，用 reinterpret_cast 转换为对应的指针
    */
    if (auto GetDescriptor = Lib->get<decltype(WasmEdge_Plugin_GetDescriptor)>(
            "WasmEdge_Plugin_GetDescriptor");
        unlikely(!GetDescriptor)) {
      return false;
      /**
      	调用方法获取插件描述符
      */
    } else if (const auto *Descriptor = GetDescriptor();
               unlikely(!Descriptor)) {
      return false;
    } else {
      /**
      	构造 CAPIPluginRegister 放入到插件注册器中
      */
      CAPIPluginRegisters.push_back(
          std::make_unique<CAPIPluginRegister>(Descriptor));
    }
  }

  auto &Plugin = PluginRegistory.back();
  Plugin.Path = Path;
  Plugin.Lib = std::move(Lib);
  return true;
}
```

```cpp
// 插件描述符
typedef struct WasmEdge_PluginDescriptor {
  const char *Name;
  const char *Description;
  uint32_t APIVersion;
  WasmEdge_PluginVersionData Version;
  uint32_t ModuleCount;
  uint32_t ProgramOptionCount;
  WasmEdge_ModuleDescriptor *ModuleDescriptions;
  WasmEdge_ProgramOption *ProgramOptions;
} WasmEdge_PluginDescriptor;
// 模块描述符
typedef struct WasmEdge_ModuleDescriptor {
  const char *Name;
  const char *Description;
  // ModuleInstance 创建函数
  WasmEdge_ModuleInstanceContext *(*Create)(
      const struct WasmEdge_ModuleDescriptor *);
} WasmEdge_ModuleDescriptor;

typedef struct WasmEdge_ProgramOption {
  const char *Name;
  const char *Description;
  WasmEdge_ProgramOptionType Type;
  void *Storage;
  const void *DefaultValue;
} WasmEdge_ProgramOption;

```

```cpp
// CAPIPluginRegister
CAPIPluginRegister(const WasmEdge_PluginDescriptor *Desc) noexcept {
  IncreaseNiftyCounter();
  // copy 一份到本地
  ModuleDescriptions.resize(Desc->ModuleCount);
  for (size_t I = 0; I < ModuleDescriptions.size(); ++I) {
    ModuleDescriptions[I].Name = Desc->ModuleDescriptions[I].Name;
    ModuleDescriptions[I].Description =
        Desc->ModuleDescriptions[I].Description;
    // createWrapper 创建了插件的 ModuleInstance
    ModuleDescriptions[I].Create = &createWrapper;
    DescriptionLookup.emplace(&ModuleDescriptions[I],
                              &Desc->ModuleDescriptions[I]);
  }
  ...
	// 注册进去
  Plugin::registerPlugin(&Descriptor);
}
```

1. createWrapper

```cpp
// createWrapper
createWrapper(const PluginModule::ModuleDescriptor *Descriptor) noexcept {
  static_assert(std::is_standard_layout_v<CAPIPluginRegister>);
  if (auto Iter = DescriptionLookup.find(Descriptor);
      unlikely(Iter == DescriptionLookup.end())) {
    return nullptr;
  } else {
    // 得到 Module Instance
    return reinterpret_cast<Runtime::Instance::ModuleInstance *>(
        Iter->second->Create(Iter->second));
  }
}
```

2. registerPlugin

```cpp
Plugin::registerPlugin(const PluginDescriptor *Desc) noexcept {
  assuming(NiftyCounter != 0);
  if (Desc->APIVersion != CurrentAPIVersion) {
    return;
  }

  const auto Index = PluginRegistory.size();
  // 调用 Plugin 的构造器，加入到插件注册器中，并做名称到注册器 id 的映射
  PluginRegistory.push_back(Plugin(Desc));
  PluginNameLookup.emplace(Desc->Name, Index);

  return;
}
```

```cpp
// 加载插件所有模块，并做名称到注册器 id 的映射
Plugin::Plugin(const PluginDescriptor *D) noexcept : Desc(D) {
  for (const auto &ModuleDesc : Span<const PluginModule::ModuleDescriptor>(
           D->ModuleDescriptions, D->ModuleCount)) {
    const auto Index = ModuleRegistory.size();
    // 调用 PluginModule 的构造器，加入到 ModuleRegistory 中，并做名称到注册器 id 的映射
    ModuleRegistory.push_back(PluginModule(&ModuleDesc));
    ModuleNameLookup.emplace(ModuleDesc.Name, Index);
  }
}
```

综上可以看出，wasmedge 加载插件的过程如下：

1. 先获取插件的目标路径
2. 然后在目标路径下找到 wasmedge 插件指定后缀的文件进行加载
3. 先打开动态库文件获取动态库的句柄
4. 通过句柄从动态库文件中获取 "WasmEdge_Plugin_GetDescriptor" 函数
5. 调用函数获取插件描述符（插件描述符中包括了模块描述符，其中又包含了 ModuleInstance 的创建函数）
6. 将插件描述符用 Plugin 的构造函数构造成 Plugin 放入 PluginRegistory 中去，并在 PluginNameLookup 做插件名称到 PluginRegistory 中 ID 的映射
7. 在 Plugin 的构造函数中将插件描述符中的模块描述符放到 ModuleRegistory 中去，并在 ModuleNameLookup 做模块名称到 ModuleRegistory 中 ID 的映射



#### 加载自定义插件的模块

```cpp

void VM::unsafeInitVM() {
  // Load the built-in modules and the plug-ins.
  unsafeLoadBuiltInHosts();
  // 这里加载插件模块
  unsafeLoadPlugInHosts();

  // Register all module instances.
  unsafeRegisterBuiltInHosts();
  // 这里注册插件模块
  unsafeRegisterPlugInHosts();
}
```

1. 加载插件模型

```cpp
void VM::unsafeLoadPlugInHosts() {
  // Load the plugins and mock them if not found.
  using namespace std::literals::string_view_literals;
  PlugInModInsts.clear();
	...
  // Load the other non-official plugins.
   // plugins() 就是获取前面的 PluginRegistry
  for (const auto &Plugin : Plugin::Plugin::plugins()) {
    if (Conf.isForbiddenPlugins(Plugin.name())) {
      continue;
    }
    ...
    // 把里面插件里面的模块都加载进来
    /**
      std::unique_ptr<Runtime::Instance::ModuleInstance> create() const noexcept {
        assuming(Desc);
        // 调用 Create 进行获取 ModuleInstance，放入 PlugInModInsts
        return std::unique_ptr<Runtime::Instance::ModuleInstance>(Desc->Create(Desc));
      }
    */
    for (const auto &Module : Plugin.modules()) {
      PlugInModInsts.push_back(Module.create());
    }
  }
}
```

2. 注册插件模型

```cpp
void VM::unsafeRegisterPlugInHosts() {
  // Register all created module instances from plugins.
  // 执行引擎注册插件的 ModuleInstance，注册到 Store 中去
  for (auto &It : PlugInModInsts) {
    ExecutorEngine.registerModule(StoreRef, *(It.get()));
  }
}

Executor::registerModule(Runtime::StoreManager &StoreMgr,
                         const Runtime::Instance::ModuleInstance &ModInst) {
  // 存入 Store 并 和 Store 做关联
  if (auto Res = StoreMgr.registerModule(&ModInst); !Res) {
    ...
    return Unexpect(ErrCode::Value::ModuleNameConflict);
  }
  return {};
}

Expect<void> registerModule(const Instance::ModuleInstance *ModInst) {
  std::unique_lock Lock(Mutex);
  auto Iter = NamedMod.find(ModInst->getModuleName());
	...
  // 做模块名和 ModuleInstance 的映射
  NamedMod.emplace(ModInst->getModuleName(), ModInst);
  // Link the module instance to this store manager.
  (const_cast<Instance::ModuleInstance *>(ModInst))
      ->linkStore(this, [](StoreManager *Store,
                           const Instance::ModuleInstance *Inst) {
        // The unlink callback.
        std::unique_lock CallbackLock(Store->Mutex);
        (Store->NamedMod).erase(std::string(Inst->getModuleName()));
      });
  return {};
}

```

在 VM 初始化里面做的工作总结起来如下：

1. 从 PluginRegister 中获取之前预处理阶段从路径中加载进来的 Plugin(内部包含了插件描述符和 PluginModuleInstance 的创建函数)
2. 调用 Plugin 里面的 ModuleDescriptor 中的 Create 方法获取 PluginModuleInstance， 存入PlugInModInsts中
3. Store 中做了模块名和 ModuleInstance 的映射
4. 完成注册



### wasm 模块执行

```cpp
// 判断是否有启动方法
auto HasValidCommandModStartFunc = [&]() {
  bool HasStart = false;
  bool Valid = false;

  // 加载模块中的所有导出方法
  auto Functions = VM.getFunctionList();
  for (auto &[FuncName, Type] : Functions) {
    // 判断是否有 start 方法
    if (FuncName == "_start") {
      HasStart = true;
      if (Type.getReturnTypes().size() == 0 &&
          Type.getParamTypes().size() == 0) {
        Valid = true;
        break;
      }
    }
  }

  // if HasStart but not Valid, insert _start to enter reactor mode
  if (HasStart && !Valid) {
    Opt.Args.value().insert(Opt.Args.value().begin(), "_start");
  }

  return HasStart && Valid;
};
```

调用启动方法

```cpp
auto AsyncResult = VM.asyncExecute("_start"sv);
```

```cpp
Async<Expect<std::vector<std::pair<ValVariant, ValType>>>>
VM::asyncExecute(std::string_view Func, Span<const ValVariant> Params,
                 Span<const ValType> ParamTypes) {
  // 函数指针，最主要的就是调用这个方法
  Expect<std::vector<std::pair<ValVariant, ValType>>> (VM::*FPtr)(
      std::string_view, Span<const ValVariant>, Span<const ValType>) =
      &VM::execute;
  // 调用异步方法 Async 构造器，并返回对象
  return {FPtr, *this, std::string(Func),
          std::vector(Params.begin(), Params.end()),
          std::vector(ParamTypes.begin(), ParamTypes.end())};
}
```

```cpp
template <typename Inst, typename... FArgsT, typename... ArgsT>
Async(T (Inst::*FPtr)(FArgsT...), Inst &TargetInst, ArgsT &&...Args)
    : StopFunc([&TargetInst]() { TargetInst.stop(); }) {
  std::promise<T> Promise;
  Future = Promise.get_future();
  Thread =
      std::thread([FPtr, P = std::move(Promise),
                   Tuple = std::tuple(
                       &TargetInst, std::forward<ArgsT>(Args)...)]() mutable {
        P.set_value(std::apply(FPtr, Tuple));
      });
  Thread.detach();
}
```

execute 执行 wasm 函数

```cpp
Expect<std::vector<std::pair<ValVariant, ValType>>>
execute(std::string_view Func, Span<const ValVariant> Params = {},
        Span<const ValType> ParamTypes = {}) {
  std::shared_lock Lock(Mutex);
  return unsafeExecute(Func, Params, ParamTypes);
}



Expect<std::vector<std::pair<ValVariant, ValType>>>
VM::unsafeExecute(std::string_view Func, Span<const ValVariant> Params,
                  Span<const ValType> ParamTypes) {
  if (ActiveModInst) {
    // Execute function and return values with the module instance.
    return unsafeExecute(ActiveModInst.get(), Func, Params, ParamTypes);
  } else {
    ...
    return Unexpect(ErrCode::Value::WrongInstanceAddress);
  }
}



Expect<std::vector<std::pair<ValVariant, ValType>>>
VM::unsafeExecute(const Runtime::Instance::ModuleInstance *ModInst,
                  std::string_view Func, Span<const ValVariant> Params,
                  Span<const ValType> ParamTypes) {
  
  // Find exported function by name.
  // 找到导出的 FunctionInstance
  Runtime::Instance::FunctionInstance *FuncInst =
      ModInst->findFuncExports(Func);

  // Execute function.
	// 执行引擎执行 FunctionInstance
  if (auto Res = ExecutorEngine.invoke(FuncInst, Params, ParamTypes);
      unlikely(!Res)) {
    if (Res.error() != ErrCode::Value::Terminated) {
      spdlog::error(ErrInfo::InfoExecuting(ModInst->getModuleName(), Func));
    }
    return Unexpect(Res);
  } else {
    return Res;
  }
}



// Invoke function. See "include/executor/executor.h".
Expect<std::vector<std::pair<ValVariant, ValType>>>
Executor::invoke(const Runtime::Instance::FunctionInstance *FuncInst,
                 Span<const ValVariant> Params,
                 Span<const ValType> ParamTypes) {
	...

  // Check parameter and function type.
  const auto &FuncType = FuncInst->getFuncType();
  const auto &PTypes = FuncType.getParamTypes();
  const auto &RTypes = FuncType.getReturnTypes();
  std::vector<ValType> GotParamTypes(ParamTypes.begin(), ParamTypes.end());
  GotParamTypes.resize(Params.size(), ValType::I32);
  if (PTypes != GotParamTypes) {
		...
    return Unexpect(ErrCode::Value::FuncSigMismatch);
  }
  
  Runtime::StackManager StackMgr;

  // Call runFunction.
  // 调用方法
  if (auto Res = runFunction(StackMgr, *FuncInst, Params); !Res) {
    return Unexpect(Res);
  }

  // Get return values.
  // 获取返回值
  std::vector<std::pair<ValVariant, ValType>> Returns(RTypes.size());
  for (uint32_t I = 0; I < RTypes.size(); ++I) {
    Returns[RTypes.size() - I - 1] =
        std::make_pair(StackMgr.pop(), RTypes[RTypes.size() - I - 1]);
  }

  // After execution, the value stack size should be 0.
  assuming(StackMgr.size() == 0);
  return Returns;
}

```

runFunction

```cpp
Expect<void>
Executor::runFunction(Runtime::StackManager &StackMgr,
                    const Runtime::Instance::FunctionInstance &Func,
                    Span<const ValVariant> Params) {
	...
  // Reset and push a dummy frame into stack.
  StackMgr.pushFrame(nullptr, AST::InstrView::iterator(), 0, 0);
  // Push arguments.
  for (auto &Val : Params) {
    StackMgr.push(Val);
  }

  // Enter and execute function.
  AST::InstrView::iterator StartIt;
  Expect<void> Res = {};
 	// 进入执行方法里面会调用:
  // auto Ret = HostFunc.run(CallFrame, std::move(Args), Rets);
  if (auto GetIt = enterFunction(StackMgr, Func, Func.getInstrs().end())) {
    StartIt = *GetIt;
  } else {
    if (GetIt.error() == ErrCode::Value::Terminated) {
      // Handle the terminated case in entering AOT or host functions.
      // For the terminated case, not return now to print the statistics.
      Res = Unexpect(GetIt.error());
    } else {
      return Unexpect(GetIt);
    }
  }
  if (Res) {
    // If not terminated, execute the instructions in interpreter mode.
    // For the entering AOT or host functions, the `StartIt` is equal to the end
    // of instruction list, therefore the execution will return immediately.
    Res = execute(StackMgr, StartIt, Func.getInstrs().end());
  }
	...
  return Unexpect(Res);
}

```

内部调用非常复杂，不过已经知道了插件如何加载以及如何运行 wasm 模块的

接下来看 rust 的 wasmedge 的使用和实现