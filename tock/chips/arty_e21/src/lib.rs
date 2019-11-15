#![feature(asm, concat_idents, const_fn)]
#![feature(exclusive_range_pattern)]
#![no_std]
#![crate_name = "arty_e21"]
#![crate_type = "rlib"]

pub mod chip {
use kernel;
use rv32i;

extern "C" {
    fn _start_trap();
}

pub struct ArtyExx {
    _userspace_kernel_boundary: rv32i::syscall::SysCall,
    _clic: rv32i::clic::Clic,
}

impl ArtyExx {
    pub unsafe fn new() -> ArtyExx { loop { } }
}

impl kernel::Chip for ArtyExx {
    type MPU = ();
    type UserspaceKernelBoundary = rv32i::syscall::SysCall;
    type SysTick = ();
}

#[export_name = "_start_trap_rust"]
    pub extern "C" fn start_trap_rust() { loop { } }

#[export_name = "_disable_interrupt_trap_handler"]
    pub extern "C" fn disable_interrupt_trap_handler(_: u32) { loop { } }
}
