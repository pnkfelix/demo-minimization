#![no_std]
#![no_main]
#![feature(const_fn, in_band_lifetimes)]

use capsules::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
use capsules::virtual_uart::{MuxUart, UartDevice};
use kernel::capabilities;
use kernel::common::ring_buffer::RingBuffer;
use kernel::component::Component;
use kernel::hil;
use kernel::Platform;
use kernel::{create_capability, debug, static_init};

mod timer_test {
#![allow(dead_code)]

use kernel::debug;
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
use arty_e21;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::str;
use kernel::debug;
use kernel::hil::gpio;
use kernel::hil::led;
use rv32i;

use crate::PROCESSES;

struct Writer;

static mut WRITER: Writer = Writer {};

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result { loop { } }
}

#[cfg(not(test))]
#[no_mangle]
#[panic_handler]
    pub unsafe extern "C" fn panic_fmt(pi: &PanicInfo) -> ! { loop { } }
}

const NUM_PROCS: usize = 4;

const FAULT_RESPONSE: kernel::procs::FaultResponse = kernel::procs::FaultResponse::Panic;

#[link_section = ".app_memory"]
static mut APP_MEMORY: [u8; 8192] = [0; 8192];

static mut PROCESSES: [Option<&'static dyn kernel::procs::ProcessType>; NUM_PROCS] =
    [None, None, None, None];

#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1000] = [0; 0x1000];

struct ArtyE21 {
    console: &'static capsules::console::Console<'static>,
    gpio: &'static capsules::gpio::GPIO<'static>,
    alarm: &'static capsules::alarm::AlarmDriver<
        'static,
        VirtualMuxAlarm<'static, rv32i::machine_timer::MachineTimer<'static>>,
    >,
    led: &'static capsules::led::LED<'static>,
    button: &'static capsules::button::Button<'static>,
}

impl Platform for ArtyE21 {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::Driver>) -> R,
    { loop { } }
}

#[no_mangle]
pub unsafe fn reset_handler() {
    rv32i::init_memory();

    let chip = static_init!(arty_e21::chip::ArtyExx, arty_e21::chip::ArtyExx::new());
/*
    chip.initialize();
*/

    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
/*
    let main_loop_cap = create_capability!(capabilities::MainLoopCapability);
    let memory_allocation_cap = create_capability!(capabilities::MemoryAllocationCapability);

*/
    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

/*
    kernel::debug::assign_gpios(
        Some(&arty_e21::gpio::PORT[0]), // Red
        Some(&arty_e21::gpio::PORT[1]),
        Some(&arty_e21::gpio::PORT[8]),
    );

    let uart_mux = static_init!(
        MuxUart<'static>,
        MuxUart::new(
            &arty_e21::uart::UART0,
            &mut capsules::virtual_uart::RX_BUF,
            115200
        )
    );
    uart_mux.initialize();

    hil::uart::Transmit::set_transmit_client(&arty_e21::uart::UART0, uart_mux);
    hil::uart::Receive::set_receive_client(&arty_e21::uart::UART0, uart_mux);

    let console = components::console::ConsoleComponent::new(board_kernel, uart_mux).finalize(());

    let mux_alarm = static_init!(
        MuxAlarm<'static, rv32i::machine_timer::MachineTimer>,
        MuxAlarm::new(&rv32i::machine_timer::MACHINETIMER)
    );
    hil::time::Alarm::set_client(&rv32i::machine_timer::MACHINETIMER, mux_alarm);

    let alarm = components::alarm::AlarmDriverComponent::new(board_kernel, mux_alarm).finalize(
        components::alarm_component_helper!(rv32i::machine_timer::MachineTimer),
    );

    let led_pins = static_init!(
        [(
            &'static dyn kernel::hil::gpio::Pin,
            capsules::led::ActivationMode
        ); 3],
        [
            (
                &arty_e21::gpio::PORT[0],
                capsules::led::ActivationMode::ActiveHigh
            ),
            (
                &arty_e21::gpio::PORT[1],
                capsules::led::ActivationMode::ActiveHigh
            ),
            (
                &arty_e21::gpio::PORT[2],
                capsules::led::ActivationMode::ActiveHigh
            ),
        ]
    );
    let led = static_init!(
        capsules::led::LED<'static>,
        capsules::led::LED::new(led_pins)
    );

    let button_pins = static_init!(
        [(
            &'static dyn kernel::hil::gpio::InterruptValuePin,
            capsules::button::GpioMode
        ); 1],
        [(
            static_init!(
                kernel::hil::gpio::InterruptValueWrapper,
                kernel::hil::gpio::InterruptValueWrapper::new(&arty_e21::gpio::PORT[4])
            )
            .finalize(),
            capsules::button::GpioMode::HighWhenPressed
        )]
    );
    let button = static_init!(
        capsules::button::Button<'static>,
        capsules::button::Button::new(
            button_pins,
            board_kernel.create_grant(&memory_allocation_cap)
        )
    );
    for &(btn, _) in button_pins.iter() {
        btn.set_client(button);
    }

    let gpio_pins = static_init!(
        [&'static dyn kernel::hil::gpio::InterruptValuePin; 3],
        [
            static_init!(
                kernel::hil::gpio::InterruptValueWrapper,
                kernel::hil::gpio::InterruptValueWrapper::new(&arty_e21::gpio::PORT[7])
            )
            .finalize(),
            static_init!(
                kernel::hil::gpio::InterruptValueWrapper,
                kernel::hil::gpio::InterruptValueWrapper::new(&arty_e21::gpio::PORT[5])
            )
            .finalize(),
            static_init!(
                kernel::hil::gpio::InterruptValueWrapper,
                kernel::hil::gpio::InterruptValueWrapper::new(&arty_e21::gpio::PORT[6])
            )
            .finalize(),
        ]
    );
    let gpio = static_init!(
        capsules::gpio::GPIO<'static>,
        capsules::gpio::GPIO::new(gpio_pins, board_kernel.create_grant(&memory_allocation_cap))
    );
    for pin in gpio_pins.iter() {
        pin.set_client(gpio);
    }

    chip.enable_all_interrupts();

    let artye21 = ArtyE21 {
        console: console,
        gpio: gpio,
        alarm: alarm,
        led: led,
        button: button,
    };

    let debugger_uart = static_init!(UartDevice, UartDevice::new(uart_mux, false));
    debugger_uart.setup();
    let ring_buffer = static_init!(
        RingBuffer<'static, u8>,
        RingBuffer::new(&mut kernel::debug::INTERNAL_BUF)
    );
    let debugger = static_init!(
        kernel::debug::DebugWriter,
        kernel::debug::DebugWriter::new(debugger_uart, &mut kernel::debug::OUTPUT_BUF, ring_buffer)
    );
    hil::uart::Transmit::set_transmit_client(debugger_uart, debugger);

    let debug_wrapper = static_init!(
        kernel::debug::DebugWriterWrapper,
        kernel::debug::DebugWriterWrapper::new(debugger)
    );
    kernel::debug::set_debug_writer_wrapper(debug_wrapper);

    debug!("Initialization complete. Entering main loop.");
*/

    extern "C" {
        static _sapps: u8;
    }

    kernel::procs::load_processes(
        board_kernel,
        chip,
        &_sapps as *const u8,
        &mut APP_MEMORY,
        &mut PROCESSES,
        FAULT_RESPONSE,
        &process_mgmt_cap,
    );

/*
    board_kernel.kernel_loop(&artye21, chip, None, &main_loop_cap);
*/
}
