#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[cfg(not(test))]
#[no_mangle]
#[panic_handler]
pub unsafe extern "C" fn panic_fmt(_: &PanicInfo) -> ! { loop { } }


#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1000] = [0; 0x1000];

#[no_mangle]
pub unsafe fn reset_handler() {
    kernel::procs::load_processes(None::<&'static arty_e21::chip::ArtyExx>.unwrap())
}
