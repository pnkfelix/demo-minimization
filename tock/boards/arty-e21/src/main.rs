#![no_std]
#![no_main]
#![feature(const_fn, in_band_lifetimes)]

use capsules::virtual_alarm::{VirtualMuxAlarm};
use kernel::capabilities;
use kernel::Platform;
use kernel::{create_capability, static_init};

mod timer_test {
#![allow(dead_code)]

use kernel::hil::time::{self, Alarm};

pub struct TimerTest<'a, A: Alarm<'a>> {
    alarm: &'a A,
}

impl<A: Alarm<'a>> TimerTest<'a, A> {
    pub const fn new(alarm: &'a A) -> TimerTest<'a, A> {
        TimerTest { alarm: alarm }
    }

    pub fn start(&self) { loop { } }
}

impl<A: Alarm<'a>> time::AlarmClient for TimerTest<'a, A> {
    fn fired(&self) { loop { } }
}
}

pub mod io {
use core::fmt::Write;
use core::panic::PanicInfo;
use core::str;

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, _: &str) -> ::core::fmt::Result { loop { } }
}

#[cfg(not(test))]
#[no_mangle]
#[panic_handler]
    pub unsafe extern "C" fn panic_fmt(_: &PanicInfo) -> ! { loop { } }
}

const NUM_PROCS: usize = 4;

// #[link_section = ".app_memory"]
// static mut APP_MEMORY: [u8; 8192] = [0; 8192];

static mut PROCESSES: [Option<&'static dyn kernel::procs::ProcessType>; NUM_PROCS] =
    [None, None, None, None];

#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1000] = [0; 0x1000];

struct ArtyE21 {
    _console: &'static capsules::console::Console<'static>,
    _gpio: &'static capsules::gpio::GPIO<'static>,
    _alarm: &'static capsules::alarm::AlarmDriver<
        'static,
        VirtualMuxAlarm<'static, rv32i::machine_timer::MachineTimer<'static>>,
    >,
    _led: &'static capsules::led::LED<'static>,
    _button: &'static capsules::button::Button<'static>,
}

impl Platform for ArtyE21 {
    fn with_driver<F, R>(&self, _: usize, _: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::Driver>) -> R,
    { loop { } }
}

#[no_mangle]
pub unsafe fn reset_handler() {
    let chip = static_init!(arty_e21::chip::ArtyExx, arty_e21::chip::ArtyExx::new());
    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    kernel::procs::load_processes(
        board_kernel,
        chip,
        &0u8 as *const u8,
        &mut [0; 8192], // APP_MEMORY,
        &mut PROCESSES,
        kernel::procs::FaultResponse::Panic,
        &process_mgmt_cap,
    );

}
