# Wasmedge Rust

## 1. Wasmedge Rust SDK 内部工作机制

### 1.1. Rust sdk 如何加载一个 wasm 模块并运行

这里以 wasmedge-rustsdk-examples 中最基础的一个例子来研究，之前看这个例子只是通过这个例子了解了 rust-sdk 如何使用，并没有深入内部，在看过 wasmedge C-API 并对 wasmedge 的执行过程有了一定的了解之后，现在回过头来再来仔细研究一下 wasmedge-rustsdk 是如何工作的，希望能通过相对上层的 wasmedge-sdk 进入到 wasmedge-sys 中，并了解其工作机制。

<hr>

以下是实现调用 wasm 模块并执行的主函数

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
  	// 1. 获取 .wasm 文件路径
    let wasm_app_file = std::env::args().nth(1).expect("Please specify a wasm file");
    // 2. 创建 config 并打开 wasi 支持
    let config = ConfigBuilder::new(CommonConfigOptions::default())
        .with_host_registration_config(HostRegistrationConfigOptions::default().wasi(true))
        .build()?;
    assert!(config.wasi_enabled());
    // 3. 根据 config 创建 wasmedge VM
    let mut vm = VmBuilder::new().with_config(config).build()?;
    // 4. 初始化 VM
    vm.wasi_module_mut()
        .expect("Not found wasi module")
        .initialize(None, None, None);
  	// 5. 加载 .wasm 文件并将 wasm 模块注册到 vm 中
    vm.register_module_from_file("wasm-app", &wasm_app_file)?
  	// 6. 执行相应的导出函数
        .run_func(Some("wasm-app"), "_start", params!())?;
    Ok(())
}
```

可以分为如下几步：

1. 获取 .wasm 文件路径
2. 创建 config 并打开 wasi 支持
3. 根据 config 创建 wasmedge VM
4. 初始化 VM
5. 加载 .wasm 文件并将 wasm 模块注册到 vm 中
6. 执行相应的导出函数

#### 1. 获取 .wasm 文件路径

这部分很简单，就是指定 .wasm 文件的路径，用于后续加载

#### 2. 创建 config 并打开 wasi 支持

```rust
let config = ConfigBuilder::new(CommonConfigOptions::default())
        .with_host_registration_config(HostRegistrationConfigOptions::default().wasi(true))
        .build()?;
```

**看一下 ConfigBuilder 这个结构体:**

ConfigBuilder 中包含了各种配置

```rust
pub struct ConfigBuilder {
    common_config: CommonConfigOptions,
    stat_config: Option<StatisticsConfigOptions>,
  	// #[cfg(feature = "aot")] 表示只有打开了 "aot" 特性才启用对应的字段
    #[cfg(feature = "aot")]
    compiler_config: Option<CompilerConfigOptions>,
    runtime_config: Option<RuntimeConfigOptions>,
    host_config: Option<HostRegistrationConfigOptions>,
}
```

再看一下 build 函数：

```rust
pub fn build(self) -> WasmEdgeResult<Config> {
    let mut inner = sys::Config::create()?;
    inner.mutable_globals(self.common_config.mutable_globals);
  	...
    inner.interpreter_mode(self.common_config.interpreter_mode);
  	...
    if let Some(host_config) = self.host_config {
        inner.wasi(host_config.wasi);
    }
    Ok(Config { inner })
}
```

最后调用 build 之后会返回一个 **Config** 结构体

可以看一下这个 Config 底层是什么：

```rust
// wasmedge-sdk
pub struct Config {
    pub(crate) inner: sys::Config,
}

// wasmedge-sys
#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) inner: std::sync::Arc<InnerConfig>,
  	// #[cfg(all(feature = "async", target_os = "linux"))] 表示只在同时启用了 async 特性且目标操作系统为 Linux 时字段才存在
    #[cfg(all(feature = "async", target_os = "linux"))]
    async_wasi_enabled: bool,
}

#[derive(Debug)]
pub(crate) struct InnerConfig(pub(crate) *mut ffi::WasmEdge_ConfigureContext);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WasmEdge_ConfigureContext {
    _unused: [u8; 0],
}
```

再来看一下 WasmEdge_ConfigureContext 在 C-API 中是什么

```cpp
// api/wasmedge/wasmedge.h

typedef struct WasmEdge_ConfigureContext WasmEdge_ConfigureContext;
// 							|
//							v
struct WasmEdge_ConfigureContext {
  WasmEdge::Configure Conf;
};
// 							|
//							v
class Configure {
public:
  ...
private:
  mutable std::shared_mutex Mutex;
  std::bitset<static_cast<uint8_t>(Proposal::Max)> Proposals;
  std::bitset<static_cast<uint8_t>(HostRegistration::Max)> Hosts;
  std::unordered_set<std::string> ForbiddenPlugins;

  CompilerConfigure CompilerConf;
  RuntimeConfigure RuntimeConf;
  StatisticsConfigure StatisticsConf;
};
```

可以看到 sdk 的 Config 内部包含了 sys 的 Config，sys 就是 wasmedge_sys，sys 的 Config 又包含了 InnerConfig，InnerConfig 实际上就是一个 C-API 中的 WasmEdge_ConfigureContext
**也就是说sdk 的 Config 和 sys 的 Config 本质上都是 C-API 中的  WasmEdge_ConfigureContext**，也即 C-API 中的 Configure

再看一下 build 函数中的 `let mut inner = sys::Config::create()?;` 做了什么

```rust
impl Config {
    pub fn create() -> WasmEdgeResult<Self> {
        let ctx = unsafe { ffi::WasmEdge_ConfigureCreate() };
        match ctx.is_null() {
            true => Err(Box::new(WasmEdgeError::ConfigCreate)),
            false => Ok(Self {
                inner: std::sync::Arc::new(InnerConfig(ctx)),
                #[cfg(all(feature = "async", target_os = "linux"))]
                async_wasi_enabled: false,
            }),
        }
    }
}
```

可以看到，也是通过 ffi 调用了 C-API 中的函数 WasmEdge_ConfigureCreate()

至此，我们可以根据 config 的调用栈大概猜测出 sdk 是如何工作的了，也即 sdk -> sys -> ffi -> c-api

#### 3. 根据 config 创建 wasmedge VM

```rust
let mut vm = VmBuilder::new().with_config(config).build()?;
```

首先看一下 VMBuilder 的结构:

```rust
#[derive(Debug, Default)]
pub struct VmBuilder {
    config: Option<Config>,
    stat: Option<Statistics>,
    store: Option<Store>,
    plugins: Vec<(String, String)>,
    #[cfg(all(feature = "async", target_os = "linux"))]
    wasi_ctx: Option<WasiContext>,
}
```

里面包含了第二步中创建的 Config，还包含了 Store、WasiContext 等类型的字段，往下继续追踪下去可以发现这些类型和 Config 一样，最终也是 C-API 中定义的结构。
例如 Store 这个结构体：

```rust
// wasmedge-sdk
pub struct Store {
    pub(crate) inner: sys::Store,
}

// wasmedge-sys
pub struct Store {
    pub(crate) inner: Arc<InnerStore>,
    pub(crate) registered: bool,
}

pub(crate) struct InnerStore(pub(crate) *mut ffi::WasmEdge_StoreContext);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WasmEdge_StoreContext {
    _unused: [u8; 0],
}
```

**可以看到，里面的很多类型，都是一个套路，sdk 中的 Xxx 包装了 sys 中的 Xxx，sys 中的 Xxx 包装了 InnerXxx，而 InnerXxx 则通过 ffi 包装了 C-API 中的 WasmEdge_XxxContext，WasmEdge_XxxContext 又是 C-API 中的 Xxx 的别名**

在来看一下 VMBuilder 的 build 做了什么，按照猜测应该是和前面的 ConfigBuilder 的 build 差不多？通过调用 C-API 提供的函数来创建 VM？实际我们可以看一下代码里是怎么样的。

```rust
#[cfg(not(feature = "async"))]
pub fn build(mut self) -> WasmEdgeResult<Vm> {
    // 和其他的一样 sdk Exector -> sys Exector -> InnerExector -> WasmEdge_ExecutorContext
    let executor = Executor::new(self.config.as_ref(), self.stat.as_mut())?;
    // store
    let store = match self.store {
        Some(store) => store,
        None => Store::new()?,
    };
    // 创建了 VM 的实例，但是这个 VM 是 sdk 的 VM，并没有 sys 的 VM 以及 C-API 中的 WasmEdge_VMContext
    let mut vm = Vm {
        config: self.config,
        stat: self.stat,
        executor,
        store,
        named_instances: HashMap::new(),
        active_instance: None,
        builtin_host_instances: HashMap::new(),
        plugin_host_instances: Vec::new(),
    };

    // * built-in host instances
    if let Some(cfg) = vm.config.as_ref() {
        if cfg.wasi_enabled() {
          	// 这个 sys 的 WasiModule 追下去是 C-API 中的 WasmEdge_ModuleInstanceContext
          	// 但是 C-API 中的 WasmEdge_ModuleInstanceContext 定义如下:
          	// struct WasmEdge_ModuleInstanceContext {};
          	// 
          	// 
          	// sys 中定义如下：
          	// pub struct WasiModule {
            //    pub(crate) inner: Arc<InnerInstance>,
            //    pub(crate) registered: bool,
            //    funcs: Vec<Function>,
            //}
          	//
          
            if let Ok(wasi_module) = sys::WasiModule::create(None, None, None) {
                vm.executor.inner.register_wasi_instance(
                    &vm.store.inner,
                    &sys::WasiInstance::Wasi(wasi_module.clone()),
                )?;

                vm.builtin_host_instances.insert(
                    HostRegistration::Wasi,
                    HostRegistrationInstance::Wasi(WasiInstance { inner: wasi_module }),
                );
            } else {
                panic!("failed to create WasiModule")
            }
        }
    }
    // * load and register plugin instances
    for (pname, mname) in self.plugins.iter() {
      	// 这里根据插件名和插件模块名创建插件实例，加载操作在 VM 的方法 with_plugin 中
        match Self::create_plugin_instance(pname, mname) {
            Some(instance) => {
                vm.plugin_host_instances.push(instance);
                vm.executor.inner.register_plugin_instance(
                    &vm.store.inner,
                    &vm.plugin_host_instances.last().unwrap().inner,
                )?;
            }
            None => panic!("Not found {}::{} plugin", pname, mname),
        }
    }
    Ok(vm)
}
```

这里并没有通过 C-API 的 VMContext 来创建，而且 sys 中没有 VMContext，而是通过组合内部组件来实现，原因可能是 sdk 才是真正需要加载 wasi 模块并调用的，所以 vm 只会在 sdk 中实现，而 sys 中并无实现必要，但是具体操作和 C-API 中的函数实现基本相同。

#### 4. 初始化 VM

```rust
// wasmedge-sdk
vm.wasi_module_mut()
    .expect("Not found wasi module")
    .initialize(None, None, None);
```

```rust
pub fn initialize(
    &mut self,
    args: Option<Vec<&str>>,
    envs: Option<Vec<&str>>,
    preopens: Option<Vec<&str>>,
) {
    self.inner.init_wasi(args, envs, preopens);
}

// wasmedge-sys
pub fn init_wasi(&mut self, args: Option<Vec<&str>>, envs: Option<Vec<&str>>,
        preopens: Option<Vec<&str>>,
    ) {
      	...
        unsafe {
      ffi::WasmEdge_ModuleInstanceInitWASI(
          self.inner.0,
          p_args.as_ptr(),
          p_args_len as u32,
          p_envs.as_ptr(),
          p_envs_len as u32,
          p_preopens.as_ptr(),
          p_preopens_len as u32,
      )
  };
}
```

**看最后返回的 WasmEdge_ModuleInstanceInitWASI**

```cpp
WASMEDGE_CAPI_EXPORT void WasmEdge_ModuleInstanceInitWASI(
    WasmEdge_ModuleInstanceContext *Cxt, const char *const *Args,
    const uint32_t ArgLen, const char *const *Envs, const uint32_t EnvLen,
    const char *const *Preopens, const uint32_t PreopenLen) {
  if (!Cxt) {
    return;
  }
  // 这个 WasiModule 初始化的时候就把所有的 wasi 方法加进去了
  // (具体可以看 c-api 中的 ./lib/host/wasi/wasimodule.cpp#WasiModule)
  auto *WasiMod = dynamic_cast<WasmEdge::Host::WasiModule *>(fromModCxt(Cxt));
  if (!WasiMod) {
    return;
  }
  std::vector<std::string> ArgVec, EnvVec, DirVec;
	...
  auto &WasiEnv = WasiMod->getEnv();
  // 在这里初始化了 Wasi 的 Env，供 wasi 方法使用
  WasiEnv.init(DirVec, ProgName, ArgVec, EnvVec);
}
```



#### 5. 加载 .wasm 文件并将 wasm 模块注册到 vm 中

```rust
vm.register_module_from_file("wasm-app", &wasm_app_file)?
    .run_func(Some("wasm-app"), "_start", params!())?;
```

register_module_from_file 第一个参数是模块名，第二个参数是文件地址

```rust
pub fn register_module_from_file(
    self,
    mod_name: impl AsRef<str>,
    file: impl AsRef<Path>,
) -> WasmEdgeResult<Self> {
    // 文件中加载 wasm 模块
    let module = Module::from_file(self.config.as_ref(), file.as_ref())?;

    // 注册命名模块
    self.register_module(Some(mod_name.as_ref()), module)
}

pub fn register_module(
    mut self,
    mod_name: Option<&str>,
    module: Module,
) -> WasmEdgeResult<Self> {
    match mod_name {
        Some(name) => {
          	// 最终调用 WasmEdge_ExecutorRegister 进行注册
            let named_instance =
                self.store
                    .register_named_module(&mut self.executor, name, &module)?;
          	// 结果放到 named_instances 中
            self.named_instances.insert(name.into(), named_instance);
        }
        None => {
            self.active_instance = Some(
                self.store
                    .register_active_module(&mut self.executor, &module)?,
            );
        }
    };
    Ok(self)
}
```

#### 6. 执行相应的导出函数

```rust
vm.register_module_from_file("wasm-app", &wasm_app_file)?
    .run_func(Some("wasm-app"), "_start", params!())?;
```

run_func 第一个参数是方法的模块名，第二个是方法名，第三个是传递的方法参数

```rust
pub fn run_func(
    &self,
    mod_name: Option<&str>,
    func_name: impl AsRef<str>,
    args: impl IntoIterator<Item = WasmValue>,
) -> WasmEdgeResult<Vec<WasmValue>> {
    match mod_name {
      	// 根据模块名找到模块实例(ModuleInstance)
        Some(mod_name) => match self.named_instances.get(mod_name) {
          	// 根据方法名调用方法
            Some(named_instance) => named_instance
          			// 根据方法名获取 FunctionInstance
                .func(func_name.as_ref())?
          			// 调用 exector 执行方法，最终调用 C-API 的 WasmEdge_ExecutorInvoke
                .run(self.executor(), args),
            None => Err(Box::new(WasmEdgeError::Vm(VmError::NotFoundModule(
                mod_name.into(),
            )))),
        },
        None => match &self.active_instance {
            Some(active_instance) => active_instance
                .func(func_name.as_ref())?
                .run(self.executor(), args),
            None => Err(Box::new(WasmEdgeError::Vm(VmError::NotFoundActiveModule))),
        },
    }
}
```

至此，分析了 wasmedge-rust-sdk 是如何工作的，也借此了解了 wasmedge-rust-sys 是如何实现的，插件是通过 vm 进行加载的，但是在本例子中并没有加载插件，后续需要继续研究 rust-sdk 具体是如何加载插件，以及插件要如何编写以及 被使用。