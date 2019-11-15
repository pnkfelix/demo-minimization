#![feature(asm, concat_idents, const_fn)]
#![feature(exclusive_range_pattern)]
#![no_std]
#![crate_name = "arty_e21"]
#![crate_type = "rlib"]

#[cfg(not_now)]
mod interrupts {
#![allow(dead_code)]

pub const MTIP: u32 = 7; // Machine Timer

pub const GPIO0: u32 = 18;
pub const GPIO1: u32 = 19;
pub const GPIO2: u32 = 20;
pub const GPIO3: u32 = 21;
pub const GPIO4: u32 = 22;
pub const GPIO5: u32 = 23;
pub const GPIO6: u32 = 24;
pub const GPIO7: u32 = 25;
pub const GPIO8: u32 = 26;
pub const GPIO9: u32 = 27;
pub const GPIO10: u32 = 28;
pub const GPIO11: u32 = 29;
pub const GPIO12: u32 = 30;
pub const GPIO13: u32 = 31;
pub const GPIO14: u32 = 32;
pub const GPIO15: u32 = 33;
pub const UART0: u32 = 16;
}

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

    pub fn enable_all_interrupts(&self) { loop { } }

    pub unsafe fn disable_pmp(&self) { loop { } }

    pub unsafe fn disable_machine_timer(&self) { loop { } }

    pub unsafe fn configure_trap_handler(&self) { loop { } }

    pub unsafe fn initialize(&self) { loop { } }
}

impl kernel::Chip for ArtyExx {
    type MPU = ();
    type UserspaceKernelBoundary = rv32i::syscall::SysCall;
    type SysTick = ();

    fn mpu(&self) -> &Self::MPU { loop { } }

    fn systick(&self) -> &Self::SysTick { loop { } }

    fn userspace_kernel_boundary(&self) -> &rv32i::syscall::SysCall { loop { } }

    fn service_pending_interrupts(&self) { loop { } }

    fn has_pending_interrupts(&self) -> bool { loop { } }

    fn sleep(&self) { loop { } }

    unsafe fn atomic<F, R>(&self, _: F) -> R
    where
        F: FnOnce() -> R,
    { loop { } }
}

#[export_name = "_start_trap_rust"]
    pub extern "C" fn start_trap_rust() { loop { } }

#[export_name = "_disable_interrupt_trap_handler"]
    pub extern "C" fn disable_interrupt_trap_handler(_: u32) { loop { } }
}
#[cfg(not_now)]
mod gpio {
use core::ops::{Index, IndexMut};

use kernel::common::StaticRef;
use sifive::gpio::{pins, GpioPin, GpioRegisters};

const GPIO0_BASE: StaticRef<GpioRegisters> =
    unsafe { StaticRef::new(0x2000_2000 as *const GpioRegisters) };

pub struct Port {
    pins: [GpioPin; 16],
}

impl Index<usize> for Port {
    type Output = GpioPin;

    fn index(&self, index: usize) -> &GpioPin { loop { } }
}

impl IndexMut<usize> for Port {
    fn index_mut(&mut self, index: usize) -> &mut GpioPin { loop { } }
}

pub static mut PORT: Port = Port {
    pins: [
        GpioPin::new(GPIO0_BASE, pins::pin0, pins::pin0::SET, pins::pin0::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin1, pins::pin1::SET, pins::pin1::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin2, pins::pin2::SET, pins::pin2::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin3, pins::pin3::SET, pins::pin3::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin4, pins::pin4::SET, pins::pin4::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin5, pins::pin5::SET, pins::pin5::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin6, pins::pin6::SET, pins::pin6::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin7, pins::pin7::SET, pins::pin7::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin8, pins::pin8::SET, pins::pin8::CLEAR),
        GpioPin::new(GPIO0_BASE, pins::pin9, pins::pin9::SET, pins::pin9::CLEAR),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin10,
            pins::pin10::SET,
            pins::pin10::CLEAR,
        ),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin11,
            pins::pin11::SET,
            pins::pin11::CLEAR,
        ),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin12,
            pins::pin12::SET,
            pins::pin12::CLEAR,
        ),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin13,
            pins::pin13::SET,
            pins::pin13::CLEAR,
        ),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin14,
            pins::pin14::SET,
            pins::pin14::CLEAR,
        ),
        GpioPin::new(
            GPIO0_BASE,
            pins::pin15,
            pins::pin15::SET,
            pins::pin15::CLEAR,
        ),
    ],
};
}
#[cfg(not_now)]
mod uart {
use kernel::common::StaticRef;
use sifive::uart::{Uart, UartRegisters};

pub static mut UART0: Uart = Uart::new(UART0_BASE, 32_000_000);

const UART0_BASE: StaticRef<UartRegisters> =
    unsafe { StaticRef::new(0x2000_0000 as *const UartRegisters) };
}
