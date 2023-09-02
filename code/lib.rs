use std::{ffi::CString, os::raw::c_void};

use wasmedge_sys::{
    ffi::{self, WasmEdge_ModuleInstanceCreate, WasmEdge_String, WasmEdge_ValType_ExternRef, WasmEdge_StringCreateByCString, WasmEdge_FunctionInstanceCreate},
    plugin::{PluginDescriptor, ModuleDescriptor, PluginVersion, ModuleInstanceCreateFn},
    instance::{FuncType, Function},
    types::WasmValue, CallingFrame, BoxedFn, WasiModule,
};
use wasmedge_types::{ValType, error::HostFuncError, NeverType};


mod wasmedge;
// use wasmedge as ffi;

fn fd_write(cf: CallingFrame, vals: Vec<WasmValue>, ptr: *mut c_void) -> Result<Vec<WasmValue>, HostFuncError>
{
    return Ok(vec![WasmValue::from_i32(0)])
}

// func type == ModuleInstanceCreateFn
pub unsafe extern "C" fn create_module_instance(arg1: *const ffi::WasmEdge_ModuleDescriptor) -> *mut ffi::WasmEdge_ModuleInstanceContext
{
    let cstr = CString::new("wasi_plugin_module_instance").unwrap();
    let module_name: WasmEdge_String = WasmEdge_StringCreateByCString(cstr.as_ptr());
    let module_instance_ctx = WasmEdge_ModuleInstanceCreate(module_name);

    let func_name = CString::new("wasi_plugin_test_fd_write").unwrap();
    let func_name = WasmEdge_StringCreateByCString(func_name.as_ptr());
    let func_type  = FuncType::create(vec![ValType::I32; 3], vec![ValType::I32]).expect("failed");
    let real_func: BoxedFn= Box::new(fd_write);
    let func_instance = Function::create_sync_func::<NeverType>(&func_type, real_func, None, 0).expect("failed");

    // let func_instance_ctx = WasmEdge_FunctionInstanceCreate(Type, HostFunc, Data, Cost)
    ffi::WasmEdge_ModuleInstanceAddFunction(module_instance_ctx, func_name, func_instance.as_ptr() as *mut _);

    return module_instance_ctx;
}

#[export_name = "WasmEdge_Plugin_GetDescriptor"]
pub fn get_plugin_descriptor() -> *const ffi::WasmEdge_PluginDescriptor 
{
    let version = PluginVersion::create(1, 0, 0, 0);
    let pd = PluginDescriptor::create("wasi-plugin", "wasi-plugin for test", version).expect("Failed to create plugin descriptor");
    let pd = pd.add_module_descriptor("wasi_plugin_test", "wasi_plugin_test_module_descriptor", Some(create_module_instance)).expect("failed");

    let box_pd = Box::new(pd);
    let heap_pd = Box::leak(box_pd);
    return heap_pd.as_raw_ptr()  // value borrowed here after move
}
