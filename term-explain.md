# Explain Terms

1. `Host Function`

    [ref](https://www.secondstate.io/articles/extend-webassembly/)
    > &emsp;&emsp;WebAssembly was developed for the browser. It gradually gain popularity on the server-side, but a significant disadvantage is its incomplete functionality and capability. The WASI proposal was initiated to solve these problems. But the forming and implementation of a standard is usually slow.  
    > &emsp;&emsp;What if you want to use a function urgently? The answer is to use the Host Function to customize your WebAssembly Runtime.  
    > &emsp;&emsp;As the name suggests, a Host Function is a function defined in the Host program. For Wasm, the Host Function can be used as an `import` segment to be registered in a `module`, and then it can be called when Wasm is running.  
    > &emsp;&emsp;Wasm has limited capability, but those can't be achieved with Wasm itself can be resolved with Host Function, which **expanded Wasm functionality to a large extent**.  
    > &emsp;&emsp;WasmEdge‘s other extensions apart from standards are majorly based on Host Function, for example, WasmEdge‘s Tensorflow API is implemented with Host Function and thus achieving the goal of running AI inference with the native speed.  
    > &emsp;&emsp;Networking socket is implemented with host function as well. Thus we can run asynchronous HTTP client and server in WasmEdge which compensate for the WebAssembly's disadvantage in networking.  
    > &emsp;&emsp;Another example. Fastly uses Host Function to add HTTP Request and Key-value store APIs to Wasm which added the extension functions.

    简单来说就是因为 wasm 能够提供的功能有限，有些无法用 wasm 实现的功能可以使用 `host function` 进行实现。而 `host function` 则是定义在 **host program** 的方法，通过 **import module** 导入到 wasm，然后进行使用。