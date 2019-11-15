//! Implementation of the SAM4L External Interrupt Controller (EIC).
//!
//! Datasheet section "21. External Interrupt Controller (EIC)".
//!
//! The External Interrupt Controller (EIC) allows pins to be configured as external
//! interrupts. Each external interrupt has its own interrupt request and can be individually
//! interrupted. Each external interrupt can generate an interrupt on rising or falling edge, or
//! high or low level. Every interrupt input has a configurable filter to remove spikes from
//! the interrupt source. Every interrupt pin can also be configured to be asynchronous in order
//! to wake up the part from sleep modes where the CLK_SYNC clock has been disabled.
//!
//! In order to use eic module on imix, comment out button driver in main.rs please!
//! The default setup is filter-enabled and asynchronous.
// Author: Josh Zhang <jiashuoz@cs.princeton.edu>
// Last modified July 22, 2019

use crate::pm::{self, Clock, PBDClock};
use kernel::common::cells::OptionalCell;
use kernel::common::peripherals::PeripheralManagement;
use kernel::common::registers::{register_bitfields, ReadOnly, ReadWrite, WriteOnly};
use kernel::common::StaticRef;
use kernel::hil;
use kernel::ClockInterface;

/// Enum for enabling or disabling spurious event filtering (i.e. de-bouncing control).
pub enum FilterMode {
    FilterEnable,
    FilterDisable,
}

/// Enum for selecting synchronous or asynchronous mode. Interrupts in asynchronous mode
/// can wake up the system from deep sleep mode.
pub enum SynchronizationMode {
    Synchronous,
    Asynchronous,
}

/// The sam4l chip supports 9 external interrupt lines: Ext1 - Ext8 and an additional
/// Non-Maskable Interrupt (NMI) pin. NMI has the same properties as the other external
/// interrupts, but is connected to the NMI request of the CPU, enabling it to interrupt
/// any other interrupt mode.
#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum Line {
    Nmi = 1,
    Ext1 = 2,
    Ext2 = 4,
    Ext3 = 8,
    Ext4 = 16,
    Ext5 = 32,
    Ext6 = 64,
    Ext7 = 128,
    Ext8 = 256,
}

#[repr(C)]
pub struct EicRegisters {
    /// Enables propagation from eic to nvic
    ier: WriteOnly<u32, Interrupt::Register>,
    /// Disables propagation from eic to nvic
    idr: WriteOnly<u32, Interrupt::Register>,
    /// Indicates if the propagation is on
    imr: ReadOnly<u32, Interrupt::Register>,
    /// A bit is set when an interrupt triggers
    isr: ReadOnly<u32, Interrupt::Register>,
    /// Clears ISR  
    icr: WriteOnly<u32, Interrupt::Register>,
    /// Sets interrupt mode
    mode: ReadWrite<u32, Interrupt::Register>,
    /// Configures falling or rising edge
    edge: ReadWrite<u32, Interrupt::Register>,
    /// Configures low or high level
    level: ReadWrite<u32, Interrupt::Register>,
    /// Configures filter
    filter: ReadWrite<u32, Interrupt::Register>,
    /// For testing
    test: ReadWrite<u32, Test::Register>,
    /// Configures synchronization
    asynchronous: ReadWrite<u32, Interrupt::Register>,
    _reserved0: ReadOnly<u32>,
    /// Enables an interrupt line
    en: WriteOnly<u32, Interrupt::Register>,
    /// Disables an interrupt line
    dis: WriteOnly<u32, Interrupt::Register>,
    /// Indicates if an interrupt line is enabled or not
    ctrl: ReadOnly<u32, Interrupt::Register>,
}

// IER: Writing a one to this bit will set the corresponding bit in IMR.
// IDR: Writing a one to this bit will clear the corresponding bit in IMR.
// IMR: 0: The corresponding interrupt is disabled.
//      1: The corresponding interrupt is enabled.
// ISR: 0: An interrupt event has not occurred.
//      1: An interrupt event has occurred.
// ICR: Writing a one to this bit will clear the corresponding bit in ISR.
// MODE:    0: The external interrupt is edge triggered.
//          1: The external interrupt is level triggered.'
// EDGE:    0: The external interrupt triggers on falling edge.
//          1: The external interrupt triggers on rising edge.
// LEVEL:   0: The external interrupt triggers on low level.
//          1: The external interrupt triggers on high level.
// FILTER:  0: The external interrupt is not filtered.
//          1: The external interrupt is filtered.
// ASYNC:   0: The external interrupt is synchronized to CLK_SYNC.
//          1: The external interrupt is asynchronous.
// EN: Writing a one to this bit will enable the corresponding external interrupt.
// DIS: Writing a one to this bit will disable the corresponding external interrupt.
// CTRL:    0: The corresponding external interrupt is disabled.
//          1: The corresponding external interrupt is enabled.
register_bitfields![
    u32,
    Interrupt [
        /// Each bit represents the setup for each line, for sam4l, only bit 0-8 makes sense
        INT OFFSET(0) NUMBITS(32) []
    ],
    /// Test is not being used right now
    Test [
        /// 0: This bit disables external interrupt test mode.
        /// 1: This bit enables external interrupt test mode.
        TESTEN OFFSET(31) NUMBITS(1) [],

        /// Writing a zero to this bit will set the input value to INTn to zero, if test mode is enabled. 
        /// Writing a one to this bit will set the input value to INTn to one, if test mode is enabled.
        INT OFFSET(0) NUMBITS(31) []
    ]
];

// Page 59 of SAM4L data sheet
const EIC_BASE: StaticRef<EicRegisters> =
    unsafe { StaticRef::new(0x400F1000 as *const EicRegisters) };

impl PeripheralManagement<pm::Clock> for Eic<'a> {
    type RegisterType = EicRegisters;

    fn get_registers(&self) -> &EicRegisters {
        &*EIC_BASE
    }

    fn get_clock(&self) -> &pm::Clock {
        &Clock::PBD(PBDClock::EIC)
    }

    fn before_peripheral_access(&self, clock: &pm::Clock, _: &EicRegisters) {
        clock.enable();
    }

    fn after_peripheral_access(&self, clock: &pm::Clock, registers: &EicRegisters) {
        if registers.imr.get() == 0 && registers.ctrl.get() == 0 {
            clock.disable();
        }
    }
}

pub struct Eic<'a> {
    callbacks: [OptionalCell<&'a dyn hil::eic::Client>; 9],
}

impl<'a> hil::eic::ExternalInterruptController for Eic<'a> {
    type Line = Line;

    fn line_enable(&self, line: &Self::Line, interrupt_mode: hil::eic::InterruptMode) {
        let regs = self.get_registers();

        // enables interrupt line, sets ctrl register
        regs.en.write(Interrupt::INT.val(*line as u32));

        self.line_configure(
            line,
            interrupt_mode,
            FilterMode::FilterEnable,
            SynchronizationMode::Asynchronous,
        );

        // enables propagation from eic to nvic, sets imr register
        regs.ier.write(Interrupt::INT.val(*line as u32));
    }

    fn line_disable(&self, line: &Self::Line) {
        let regs = self.get_registers();

        // disables interrupt line, sets ctrl register
        regs.dis.write(Interrupt::INT.val(*line as u32));

        // disables propagation from eic to nvic, sets imr register
        regs.idr.write(Interrupt::INT.val(*line as u32));
    }
}

impl<'a> Eic<'a> {
    fn line_configure(
        &self,
        line: &Line,
        interrupt_mode: hil::eic::InterruptMode,
        filter_mode: FilterMode,
        synchronization_mode: SynchronizationMode,
    ) {
        let mode_bits = match interrupt_mode {
            hil::eic::InterruptMode::RisingEdge => 0b00,
            hil::eic::InterruptMode::FallingEdge => 0b01,
            hil::eic::InterruptMode::HighLevel => 0b10,
            hil::eic::InterruptMode::LowLevel => 0b11,
        };

        self.set_interrupt_mode(mode_bits, line);

        match filter_mode {
            FilterMode::FilterEnable => self.line_enable_filter(line),
            FilterMode::FilterDisable => self.line_disable_filter(line),
        }

        match synchronization_mode {
            SynchronizationMode::Synchronous => self.line_disable_asyn(line),
            SynchronizationMode::Asynchronous => self.line_enable_asyn(line),
        }
    }

    fn set_interrupt_mode(&self, mode_bits: u8, line: &Line) {
        let regs = self.get_registers();

        let original_mode: u32 = regs.mode.get();
        let original_level: u32 = regs.level.get();
        let original_edge: u32 = regs.edge.get();
        let interrupt_line: u32 = *line as u32;

        if mode_bits & 0b10 != 0 {
            regs.mode.set(original_mode | interrupt_line); // 0b10 or 0b11 -> level
        } else {
            regs.mode.set(original_mode & !interrupt_line); // 0b00 or 0b01 -> edge
        }

        if mode_bits & 0b01 != 0 {
            regs.edge.set(original_edge & !interrupt_line); // falling edge
            regs.level.set(original_level & !interrupt_line); // low level
        } else {
            regs.edge.set(original_edge | interrupt_line); // rising edge
            regs.level.set(original_level | interrupt_line); // high level
        }
    }

    const fn new() -> Eic<'a> {
        Eic {
            callbacks: [
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
                OptionalCell::empty(),
            ],
        }
    }

    /// Registers a client associated with a line.
    pub fn set_client(&self, client: &'a dyn hil::eic::Client, line: &Line) {
        self.callbacks.get(*line as usize).map(|c| c.set(client));
    }

    /// Executes client function when an interrupt is triggered.
    pub fn handle_interrupt(&self, line: &Line) {
        // Clears interrupt bit and then handle interrupt
        let regs = self.get_registers();
        regs.icr.write(Interrupt::INT.val(*line as u32));

        self.callbacks[*line as usize].map(|cb| {
            cb.fired();
        });
    }

    /// Returns true is a line is enabled. This doesn't mean the interrupt is being
    /// propagated through. Developers can use this function for testing.
    pub fn line_is_enabled(&self, line: &Line) -> bool {
        let regs = self.get_registers();

        ((*line as u32) & regs.ctrl.get()) != 0
    }

    /// Returns true if interrupt is being propagated from EIC to the interrupt controller of
    /// the external interrupt on a specific line, false otherwise. Developers can use this
    /// function for testing.
    pub fn line_interrupt_is_enabled(&self, line: &Line) -> bool {
        let regs = self.get_registers();

        ((*line as u32) & regs.imr.get()) != 0
    }

    /// Returns true if a line's interrupt is pending, false otherwise. Developers can use this
    /// function for testing.
    pub fn line_interrupt_pending(&self, line: &Line) -> bool {
        let regs = self.get_registers();

        ((*line as u32) & regs.isr.get()) != 0
    }

    /// Enables filtering mode on synchronous interrupt
    fn line_enable_filter(&self, line: &Line) {
        let regs = self.get_registers();

        let original_filter: u32 = regs.filter.get();
        regs.filter.set(original_filter | (*line as u32));
    }

    /// Disables filtering mode on synchronous interrupt
    fn line_disable_filter(&self, line: &Line) {
        let regs = self.get_registers();

        let original_filter: u32 = regs.filter.get();
        regs.filter.set(original_filter & (!(*line as u32)));
    }

    /// Returns true if a line is in filter mode, false otherwise.
    pub fn line_enable_filter_is_enabled(&self, line: &Line) -> bool {
        let regs = self.get_registers();

        ((*line as u32) & regs.filter.get()) != 0
    }

    /// Enables asynchronous mode
    fn line_enable_asyn(&self, line: &Line) {
        let regs = self.get_registers();

        let original_asyn: u32 = regs.asynchronous.get();
        regs.asynchronous
            .modify(Interrupt::INT.val(original_asyn | (*line as u32)));
    }

    /// Disables asynchronous mode, goes back to synchronous mode
    fn line_disable_asyn(&self, line: &Line) {
        let regs = self.get_registers();

        let original_asyn: u32 = regs.asynchronous.get();
        regs.asynchronous
            .modify(Interrupt::INT.val(original_asyn & (!(*line as u32))));
    }

    /// Returns true if a line is in asynchronous mode, false otherwise
    pub fn line_asyn_is_enabled(&self, line: &Line) -> bool {
        let regs = self.get_registers();

        ((*line as u32) & regs.asynchronous.get()) != 0
    }
}

/// Static state to manage the EIC
pub static mut EIC: Eic = Eic::new();
