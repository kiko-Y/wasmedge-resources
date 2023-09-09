### Wasmedge 是如何支持 wasi 接口的？

#### 1. wasi 加载

首先通过之前的 debug 已经知道了 wasmedge 默认会开启 wasi，在开启 wasi 的情况下 wasmedge 会加载一个 WasiModule 的类。这个类在初始化的时候，首先会调用父类的初始化方法：

```cpp
WasiModule::WasiModule() : ModuleInstance("wasi_snapshot_preview1") {...}
```

这个 WasiModule 就是一个 ModuleInstance，只是初始化的时候将其 ModName 命名为 wasi_snapshot_preview1，同时初始化了一系列的 HostFunction，这些 HostFunction 就是 wasi spec 中需要实现的接口函数，同时这些 HostFunction 的 FunctionName 也命名为 wasi spec 中需要实现的接口函数的名称，比如 args_get、args_sizes_get。

以上是 WasiModule 进行加载时进行的工作(插件的 Module 也是以同样的方式加入的，只是初始化的时候需要通过加载的插件描述符来创建插件 ModuleInstance)。

<hr>

#### 2. wasi 注册

然后 ModuleInstance 和 HostFunctionInstance 是如何注册进执行引擎里的呢？
在`void VM::unsafeRegisterBuiltInHosts() `中调用链如下：

1. ExecutorEngine 中： `ExecutorEngine::registerModule(StoreManager, ModuleInstance)`
2. StoreManager 中：`StoreManager::registerModule(ModuleInstance)`
3. StoreManager 中：`NamedMod.emplace(ModInst->getModuleName(), ModInst)`
4. StoreManager 中：`ModuleInstance.linkStore(StoreManager, BeforeModuleDestroyCallback)`

也即最后将 ModuleInstance 和其名称的映射加到了 StoreManager 的 NamedMod 中去了，并将 ModuleInstnce 链接到 Store 中，设置了 ModleInstance 销毁的回调函数。

至此 ModuleInstance 的注册也完成了，上述步骤实现了 ModuleInstance 注册到 VM 中的 StoreManager

<hr>

#### 3. wasm 模块的加载验证实例化

1. `VM.loadWasm(InputPath.u8string())`

   加载入内存，转化为 AST Module

2. `VM.validate()`

   对 AST Module 进行[验证](https://webassembly.github.io/spec/core/valid/conventions.html)

3. `VM.instantiate()`

   实例化 AST Module 为 NamedModuleInstance

   ```cpp
   // vm.cpp
   
   Expect<void> VM::unsafeInstantiate() {
     if (Stage < VMStage::Validated) {
       // When module is not validated, not instantiate.
       spdlog::error(ErrCode::Value::WrongVMWorkflow);
       return Unexpect(ErrCode::Value::WrongVMWorkflow);
     }
     // 通过调用 ExecutorEngine.instantiateModule() 方法来获取待允许模块的 ModuleInstance
     if (auto Res = ExecutorEngine.instantiateModule(StoreRef, *Mod.get())) {
       Stage = VMStage::Instantiated;
       // 设置为 ActiveModuleInstance
       ActiveModInst = std::move(*Res);
       return {};
     } else {
       return Unexpect(Res);
     }
   }
   ```

   

   1. `ExecutorEngine.instantiateModule(Runtime::StoreManager, AST::Module)`

      ```cpp
      // executor.cpp
      
      /// Instantiate a WASM Module. See "include/executor/executor.h".
      Expect<std::unique_ptr<Runtime::Instance::ModuleInstance>>
      Executor::instantiateModule(Runtime::StoreManager &StoreMgr,
                                  const AST::Module &Mod) {
        if (auto Res = instantiate(StoreMgr, Mod)) {
          return Res;
        } else {...}
      }
      ```

   2. `Executor::instantiate(Runtime::StoreManager, AST::Module, std::optional<std::string_view>)`

      

      ```cpp
      // module.cpp
      
      Executor::instantiate(Runtime::StoreManager &StoreMgr, const AST::Module &Mod,
                            std::optional<std::string_view> Name) {
        // 实例化为有名或无名模块(这里应该是无名模块)
        if (Name.has_value()) {
          ModInst = std::make_unique<Runtime::Instance::ModuleInstance>(Name.value());
        } else {
          ModInst = std::make_unique<Runtime::Instance::ModuleInstance>("");
        }
      	// 各个 section 的实例化
        // Instantiate Function Types in Module Instance. (TypeSec)
        for (auto &FuncType : Mod.getTypeSection().getContent()) {
          // Copy param and return lists to module instance.
          ModInst->addFuncType(FuncType);
        }
        ...
        // This function will always success.
        instantiate(*ModInst, FuncSec, CodeSec);
        // This function will always success.
        instantiate(*ModInst, TabSec);
        ...
        return ModInst;
      }
      ```

   #### 4. 执行方法，命令行模式执行 _start 方法

   ```cpp
   // runtimeTool.cpp 162
   auto AsyncResult = VM.asyncExecute("_start"sv);
   ```

   ```cpp
   // vm.cpp 413
   Async<Expect<std::vector<std::pair<ValVariant, ValType>>>>
   VM::asyncExecute(std::string_view Func, Span<const ValVariant> Params,
                    Span<const ValType> ParamTypes) {
     Expect<std::vector<std::pair<ValVariant, ValType>>> (VM::*FPtr)(
         std::string_view, Span<const ValVariant>, Span<const ValType>) =
         &VM::execute;
     return {FPtr, *this, std::string(Func),
             std::vector(Params.begin(), Params.end()),
             std::vector(ParamTypes.begin(), ParamTypes.end())};
   }
   ```

   ```cpp
   // vm.h 137
   Expect<std::vector<std::pair<ValVariant, ValType>>>
   execute(std::string_view Func, Span<const ValVariant> Params = {},
           Span<const ValType> ParamTypes = {}) {
     std::shared_lock Lock(Mutex);
     return unsafeExecute(Func, Params, ParamTypes);
   }
   ```

   ```cpp
   // vm.cpp 364
   Expect<std::vector<std::pair<ValVariant, ValType>>>
   VM::unsafeExecute(std::string_view Func, Span<const ValVariant> Params,
                     Span<const ValType> ParamTypes) {
     if (ActiveModInst) {
       // Execute function and return values with the module instance.
       return unsafeExecute(ActiveModInst.get(), Func, Params, ParamTypes);
     } else {...}
   }
   ```

   ```cpp
   // vm.cpp 393
   Expect<std::vector<std::pair<ValVariant, ValType>>>
   VM::unsafeExecute(const Runtime::Instance::ModuleInstance *ModInst,
                     std::string_view Func, Span<const ValVariant> Params,
                     Span<const ValType> ParamTypes) {
     // Find exported function by name.
     Runtime::Instance::FunctionInstance *FuncInst =
         ModInst->findFuncExports(Func);
   
     // Execute function.
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
   ```

   ```cpp
   // executor.cpp 59
   Expect<std::vector<std::pair<ValVariant, ValType>>>
   Executor::invoke(const Runtime::Instance::FunctionInstance *FuncInst,
                    Span<const ValVariant> Params,
                    Span<const ValType> ParamTypes) {
   ...
     // 执行方法
     if (auto Res = runFunction(StackMgr, *FuncInst, Params); !Res) {
       return Unexpect(Res);
     }
     // Get return values.
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

   ```cpp
   // engine.cpp 18
   Expect<void>
   Executor::runFunction(Runtime::StackManager &StackMgr,
                         const Runtime::Instance::FunctionInstance &Func,
                         Span<const ValVariant> Params) {
     // Push arguments.
     // 将参数推入栈中
     for (auto &Val : Params) {
       StackMgr.push(Val);
     }
     // Enter and execute function.
     AST::InstrView::iterator StartIt;
     Expect<void> Res = {};
     // ----------------执行方法-------------------
     if (auto GetIt = enterFunction(StackMgr, Func, Func.getInstrs().end())) {
       StartIt = *GetIt;
     } else {...}
     if (Res) {
       // If not terminated, execute the instructions in interpreter mode.
       // For the entering AOT or host functions, the `StartIt` is equal to the end
       // of instruction list, therefore the execution will return immediately.
       Res = execute(StackMgr, StartIt, Func.getInstrs().end());
     }
   	...
     if (Res) {
       return {};
     }
     if (Res.error() == ErrCode::Value::Terminated) {
       StackMgr.reset();
     }
     return Unexpect(Res);
   }
   ```

   ```cpp
   // helper.cpp 16
   Expect<AST::InstrView::iterator>
   Executor::enterFunction(Runtime::StackManager &StackMgr,
                           const Runtime::Instance::FunctionInstance &Func,
                           const AST::InstrView::iterator RetIt, bool IsTailCall) {
     // RetIt: the return position when the entered function returns.
   
     if (Func.isHostFunction()) {
       ...
     } else if (Func.isCompiledFunction()) {
       // Compiled function case: Execute the function and jump to the
       ...
     } else {
       // Native function case: Jump to the start of the function body.
       // Push local variables into the stack.
       for (auto &Def : Func.getLocals()) {
         for (uint32_t I = 0; I < Def.first; I++) {
           StackMgr.push(ValueFromType(Def.second));
         }
       }
   
       // Push frame.
       // The PC must -1 here because in the interpreter mode execution, the PC
       // will increase after the callee return.
       StackMgr.pushFrame(Func.getModule(),           // Module instance
                          RetIt - 1,                  // Return PC
                          ArgsN + Func.getLocalNum(), // Arguments num + local num
                          RetsN,                      // Returns num
                          IsTailCall                  // For tail-call
       );
   		// --------------------- 首条指令 ----------------------
       // For native function case, the continuation will be the start of the
       // function body.
       return Func.getInstrs().begin();
     }
   }
   ```

   结束获取返回值到上一个函数栈中继续执行

   ```cpp
   // engine.cpp 53
   Res = execute(StackMgr, StartIt, Func.getInstrs().end());
   ```

   ```cpp
   // engine.cpp 83  
   Expect<void> Executor::execute(Runtime::StackManager &StackMgr,
                                  const AST::InstrView::iterator Start,
                                  const AST::InstrView::iterator End) {
     // 起始指令
     AST::InstrView::iterator PC = Start;
     // 结束指令
     AST::InstrView::iterator PCEnd = End;
     // 这里定义了指令分发函数，对于某个操作码，执行相应的方法
     auto Dispatch = [this, &PC, &StackMgr]() -> Expect<void> {
       const AST::Instruction &Instr = *PC;
       switch (Instr.getOpCode()) {
         case OpCode::xxx:
           ...
           return ...
         ...
       }
     while (PC != PCEnd) {
       if (Stat) {
         OpCode Code = PC->getOpCode();
         if (Conf.getStatisticsConfigure().isInstructionCounting()) {
           Stat->incInstrCount();
         }
         // Add cost. Note: if-else case should be processed additionally.
         if (Conf.getStatisticsConfigure().isCostMeasuring()) {
           if (unlikely(!Stat->addInstrCost(Code))) {
             const AST::Instruction &Instr = *PC;
             spdlog::error(
                 ErrInfo::InfoInstruction(Instr.getOpCode(), Instr.getOffset()));
             return Unexpect(ErrCode::Value::CostLimitExceeded);
           }
         }
       }
       if (auto Res = Dispatch(); !Res) {
         return Unexpect(Res);
       }
       PC++;
     }
     return {};
   }
      
   ```

   执行引擎内部类似是一个 CPU，负责取指，然后根据指令码执行相应的指令。内部关于调用方法的指令只有一个只有

   ```cpp
     case OpCode::Ref__func: {
       const auto *ModInst = StackMgr.getModule();
       const auto *FuncInst = *ModInst->getFunc(Instr.getTargetIndex());
       StackMgr.push<FuncRef>(FuncRef(FuncInst));
       return {};
     }
   ```

   该指令用于取栈顶的 ModuleInstance 然后在其内部找到指令码中指定的 FunctionInstance，然后将方法引用放入栈中，但是在这之前肯定得要先把要**用到的方法所在的模块压入栈顶**才可使用，这部分工作没有看到在哪里完成。

   **如何调用 wasm 模块中引用的方法**还没找到在什么地方，这里只是把方法引用放到了栈中，没有具体执行。后续要找到 wasm 模块中调用方法会在哪里执行。应该就可以知道 wasmedge 是如何在执行 _start 函数的时候调用到对应的 wasiModule 中的相应接口的。

   猜测：

   1. 也许是在加载的待运行的 wasm 进行实例化的时候，将内部的 wasi 方法映射为 wasiModule 的模块名+方法索引，以此来进行调用。

   可能需要去看一下通过 --target wasm32-wasi 编译成 wasm 之后的 wasi 文件内部是什么样的，目前有 wasi2wat 工具，应该可以将 wasi 转成可读的格式。

   





