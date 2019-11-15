//! Drivers and chip support for the E21 soft core.

#![feature(asm, concat_idents, const_fn)]
#![feature(exclusive_range_pattern)]
#![no_std]
#![crate_name = "arty_e21"]
#![crate_type = "rlib"]

mod interrupts {
//! Named interrupts for the E21 Arty core.

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
use kernel::debug;
use rv32i;
use rv32i::machine_timer;

use crate::gpio;
use crate::interrupts;
use crate::uart;

extern "C" {
    fn _start_trap();
}

pub struct ArtyExx {
    userspace_kernel_boundary: rv32i::syscall::SysCall,
    clic: rv32i::clic::Clic,
}

impl ArtyExx {
    pub unsafe fn new() -> ArtyExx {
        // Make a bit-vector of all interrupt locations that we actually intend
        // to use on this chip.
        // 0001 1111 1111 1111 1111 0000 0000 1000 0000
        let in_use_interrupts: u64 = 0x1FFFF0080;

        ArtyExx {
            userspace_kernel_boundary: rv32i::syscall::SysCall::new(),
            clic: rv32i::clic::Clic::new(in_use_interrupts),
        }
    }

    pub fn enable_all_interrupts(&self) {
        self.clic.enable_all();
    }

    /// Configure the PMP to allow all accesses in both machine mode (the
    /// default) and in user mode.
    ///
    /// This needs to be replaced with a real PMP driver. See
    /// https://github.com/tock/tock/issues/1135
    pub unsafe fn disable_pmp(&self) {
        asm!("
            // PMP PMP PMP
            // PMP PMP PMP
            // PMP PMP PMP
            // PMP PMP PMP
            // TODO: Add a real PMP driver!!
            // Take some time to disable the PMP.

            // Set the first region address to 0xFFFFFFFF. When using top-of-range mode
            // this will include the entire address space.
            lui  t0, %hi(0xFFFFFFFF)
            addi t0, t0, %lo(0xFFFFFFFF)
            csrw 0x3b0, t0    // CSR=pmpaddr0

            // Set the first region to use top-of-range and allow everything.
            // This is equivalent to:
            // R=1, W=1, X=1, A=01, L=0
            li   t0, 0x0F
            csrw 0x3a0, t0    // CSR=pmpcfg0
        "
        :
        :
        :
        : "volatile");
    }

    /// By default the machine timer is enabled and will trigger interrupts. To
    /// prevent that we can make the compare register very large to effectively
    /// stop the interrupt from triggering, and then the machine timer can be
    /// used later as needed.
    pub unsafe fn disable_machine_timer(&self) {
        asm!("
            // Initialize machine timer mtimecmp to disable the machine timer
            // interrupt.
            li   t0, -1       // Set mtimecmp to 0xFFFFFFFF
            lui  t1, %hi(0x02004000)     // Load the address of mtimecmp to t1
            addi t1, t1, %lo(0x02004000) // Load the address of mtimecmp to t1
            sw   t0, 0(t1)    // mtimecmp is 64 bits, set to all ones
            sw   t0, 4(t1)    // mtimecmp is 64 bits, set to all ones
        "
        :
        :
        :
        : "volatile");
    }

    /// Setup the function that should run when a trap happens.
    ///
    /// This needs to be chip specific because how the CLIC works is configured
    /// when the trap handler address is specified in mtvec, and that is only
    /// valid for platforms with a CLIC.
    pub unsafe fn configure_trap_handler(&self) {
        asm!("
            // The csrw instruction writes a Control and Status Register (CSR)
            // with a new value.
            //
            // CSR 0x305 (mtvec, 'Machine trap-handler base address.') sets the
            // address of the trap handler. We do not care about its old value,
            // so we don't bother reading it. We want to enable direct CLIC mode
            // so we set the second lowest bit.
            lui  t0, %hi(_start_trap)
            addi t0, t0, %lo(_start_trap)
            ori  t0, t0, 0x02 // Set CLIC direct mode
            csrw 0x305, t0    // Write the mtvec CSR.
        "
        :
        :
        :
        : "volatile");
    }

    /// Generic helper initialize function to setup all of the chip specific
    /// operations. Different boards can call the functions that `initialize()`
    /// calls directly if it needs to use a custom setup operation.
    pub unsafe fn initialize(&self) {
        self.disable_pmp();
        self.disable_machine_timer();
        self.configure_trap_handler();
    }
}

impl kernel::Chip for ArtyExx {
    // While there is initial support for a PMP driver (as of 2019-10-04), it is not
    // complete, and while it should disable the PMP, it seems to cause some negative
    // side effects (context switching does not work correctly) on the Arty-E21 platform.
    //
    // TODO: implement the PMP driver and add it here `type MPU = rv32i::pmp::PMPConfig;`.
    //
    // See https://github.com/tock/tock/pull/1382 for (a little) more information.
    type MPU = ();
    type UserspaceKernelBoundary = rv32i::syscall::SysCall;
    type SysTick = ();

    fn mpu(&self) -> &Self::MPU {
        &()
    }

    fn systick(&self) -> &Self::SysTick {
        &()
    }

    fn userspace_kernel_boundary(&self) -> &rv32i::syscall::SysCall {
        &self.userspace_kernel_boundary
    }

    fn service_pending_interrupts(&self) {
        unsafe {
            while let Some(interrupt) = self.clic.next_pending() {
                match interrupt {
                    interrupts::MTIP => machine_timer::MACHINETIMER.handle_interrupt(),

                    interrupts::GPIO0 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO1 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO2 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO3 => gpio::PORT[3].handle_interrupt(),
                    interrupts::GPIO4 => gpio::PORT[4].handle_interrupt(),
                    interrupts::GPIO5 => gpio::PORT[5].handle_interrupt(),
                    interrupts::GPIO6 => gpio::PORT[6].handle_interrupt(),
                    interrupts::GPIO7 => gpio::PORT[7].handle_interrupt(),
                    interrupts::GPIO8 => gpio::PORT[8].handle_interrupt(),
                    interrupts::GPIO9 => gpio::PORT[9].handle_interrupt(),
                    interrupts::GPIO10 => gpio::PORT[10].handle_interrupt(),
                    interrupts::GPIO11 => gpio::PORT[11].handle_interrupt(),
                    interrupts::GPIO12 => gpio::PORT[12].handle_interrupt(),
                    interrupts::GPIO13 => gpio::PORT[13].handle_interrupt(),
                    interrupts::GPIO14 => gpio::PORT[14].handle_interrupt(),
                    interrupts::GPIO15 => gpio::PORT[15].handle_interrupt(),

                    interrupts::UART0 => uart::UART0.handle_interrupt(),

                    _ => debug!("Pidx {}", interrupt),
                }

                // Mark that we are done with this interrupt and the hardware
                // can clear it.
                self.clic.complete(interrupt);
            }
        }
    }

    fn has_pending_interrupts(&self) -> bool {
        self.clic.has_pending()
    }

    fn sleep(&self) {
        unsafe {
            rv32i::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        rv32i::support::atomic(f)
    }
}

/// Trap handler for board/chip specific code.
///
/// For the arty-e21 this gets called when an interrupt occurs while the chip is
/// in kernel mode. All we need to do is check which interrupt occurred and
/// disable it.
#[export_name = "_start_trap_rust"]
pub extern "C" fn start_trap_rust() {
    let mut mcause: i32;

    unsafe {
        asm!("
            // Read the mcause CSR to determine why we entered the trap handler.
            // Since we are using the CLIC, the hardware includes the interrupt
            // index in the mcause register.
            csrr $0, 0x342    // CSR=0x342=mcause
        "
        : "=r"(mcause)
        :
        :
        : "volatile");
    }

    // Check if the trap was from an interrupt or some other exception.
    if mcause < 0 {
        // If the most significant bit is set (i.e. mcause is negative) then
        // this was an interrupt. The interrupt number is then the lowest 8
        // bits.
        let interrupt_index = mcause & 0xFF;
        unsafe {
            rv32i::clic::disable_interrupt(interrupt_index as u32);
        }
    } else {
        // Otherwise, the kernel encountered a fault...so panic!()?
        panic!("kernel exception");
    }
}

/// Function that gets called if an interrupt occurs while an app was running.
/// mcause is passed in, and this function should correctly handle disabling the
/// interrupt that fired so that it does not trigger again.
#[export_name = "_disable_interrupt_trap_handler"]
pub extern "C" fn disable_interrupt_trap_handler(mcause: u32) {
    // The interrupt number is then the lowest 8
    // bits.
    let interrupt_index = mcause & 0xFF;
    unsafe {
        rv32i::clic::disable_interrupt(interrupt_index as u32);
    }
}
}
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

    fn index(&self, index: usize) -> &GpioPin {
        &self.pins[index]
    }
}

impl IndexMut<usize> for Port {
    fn index_mut(&mut self, index: usize) -> &mut GpioPin {
        &mut self.pins[index]
    }
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
mod uart {
use kernel::common::StaticRef;
use sifive::uart::{Uart, UartRegisters};

pub static mut UART0: Uart = Uart::new(UART0_BASE, 32_000_000);

const UART0_BASE: StaticRef<UartRegisters> =
    unsafe { StaticRef::new(0x2000_0000 as *const UartRegisters) };
}
