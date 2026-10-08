/*
 * Minimal WAMR interpreter bindings for the ePipe activation boundary.
 * SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
 */

use core::ffi::{c_char, c_void};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NativeSymbol {
    pub symbol: *const c_char,
    pub func_ptr: *mut c_void,
    pub signature: *const c_char,
    pub attachment: *mut c_void,
}

const _: () = {
    assert!(core::mem::size_of::<NativeSymbol>() == 4 * core::mem::size_of::<*mut c_void>());
    assert!(core::mem::align_of::<NativeSymbol>() == core::mem::align_of::<*mut c_void>());
};

#[repr(C)]
pub struct WASMModuleCommon {
    _private: [u8; 0],
}

#[repr(C)]
pub struct WASMModuleInstanceCommon {
    _private: [u8; 0],
}

#[repr(C)]
pub struct WASMExecEnv {
    _private: [u8; 0],
}

pub type WASMFunctionInstanceCommon = c_void;
pub type wasm_module_t = *mut WASMModuleCommon;
pub type wasm_module_inst_t = *mut WASMModuleInstanceCommon;
pub type wasm_function_inst_t = *mut WASMFunctionInstanceCommon;
pub type wasm_exec_env_t = *mut WASMExecEnv;

unsafe extern "C" {
    pub fn wasm_runtime_init() -> bool;
    pub fn wasm_runtime_register_natives(
        module_name: *const c_char,
        native_symbols: *mut NativeSymbol,
        n_native_symbols: u32,
    ) -> bool;
    pub fn wasm_runtime_destroy();
    pub fn wasm_runtime_load(
        buf: *mut u8,
        size: u32,
        error_buf: *mut c_char,
        error_buf_size: u32,
    ) -> wasm_module_t;
    pub fn wasm_runtime_unload(module: wasm_module_t);
    pub fn wasm_runtime_instantiate(
        module: wasm_module_t,
        default_stack_size: u32,
        host_managed_heap_size: u32,
        error_buf: *mut c_char,
        error_buf_size: u32,
    ) -> wasm_module_inst_t;
    pub fn wasm_runtime_deinstantiate(module_inst: wasm_module_inst_t);
    pub fn wasm_runtime_lookup_function(
        module_inst: wasm_module_inst_t,
        name: *const c_char,
    ) -> wasm_function_inst_t;
    pub fn wasm_runtime_get_exec_env_singleton(module_inst: wasm_module_inst_t) -> wasm_exec_env_t;
    pub fn wasm_runtime_call_wasm(
        exec_env: wasm_exec_env_t,
        function: wasm_function_inst_t,
        argc: u32,
        argv: *mut u32,
    ) -> bool;
    pub fn wasm_runtime_get_exception(module_inst: wasm_module_inst_t) -> *const c_char;
    pub fn wasm_runtime_get_function_attachment(exec_env: wasm_exec_env_t) -> *mut c_void;
}
