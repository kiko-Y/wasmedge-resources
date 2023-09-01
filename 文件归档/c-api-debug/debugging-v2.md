

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