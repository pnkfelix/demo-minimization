#![feature(derive_clone_copy, compiler_builtins_lib, fmt_internals,
           core_panic, derive_eq)]
#![feature(prelude_import)]
#![feature(core_intrinsics, ptr_internals, const_fn)]
#![feature(panic_info_message)]
#![feature(in_band_lifetimes, crate_visibility_modifier)]
#![feature(associated_type_defaults)]
#![warn(unreachable_pub)]
#![no_std]

pub mod capabilities {



    pub unsafe trait ProcessManagementCapability { }
    pub unsafe trait MainLoopCapability { }
    pub unsafe trait MemoryAllocationCapability { }
}

pub mod common {
    pub mod registers {
        pub use tock_registers::registers::InMemoryRegister;
        pub use tock_registers::registers::RegisterLongName;
        pub use tock_registers::registers::{Field, FieldValue,
                                            LocalRegisterCopy};
        pub use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
        pub use tock_registers::{register_bitfields, register_structs};
    }
    pub mod deferred_call {
        use core::convert::Into;
        use core::convert::TryFrom;
        use core::convert::TryInto;
        use core::marker::Copy;
        use core::sync::atomic::AtomicUsize;
        use core::sync::atomic::Ordering;
        static DEFERRED_CALL: AtomicUsize = AtomicUsize::new(0);
        pub fn has_tasks() -> bool { loop  { } }
        pub struct DeferredCall<T>(T);
        impl <T: Into<usize> + TryFrom<usize> + Copy> DeferredCall<T> {
            pub const unsafe fn new(task: T) -> Self { DeferredCall(task) }
            pub fn set(&self) { loop  { } }
            pub fn next_pending() -> Option<T> { loop  { } }
        }
    }
    pub mod dynamic_deferred_call {
        use crate::common::cells::OptionalCell;
        use core::cell::Cell;
        static mut DYNAMIC_DEFERRED_CALL: Option<&'static DynamicDeferredCall>
               =
            None;
        pub struct DynamicDeferredCallClientState {
            scheduled: Cell<bool>,
            client: OptionalCell<&'static dyn DynamicDeferredCallClient>,
        }
        impl Default for DynamicDeferredCallClientState {
            fn default() -> DynamicDeferredCallClientState { loop  { } }
        }
        pub struct DynamicDeferredCall {
            client_states: &'static [DynamicDeferredCallClientState],
            handle_counter: Cell<usize>,
            call_pending: Cell<bool>,
        }
        impl DynamicDeferredCall {
            pub fn new(client_states:
                           &'static [DynamicDeferredCallClientState])
             -> DynamicDeferredCall {
                loop  { }
            }
            pub unsafe fn set_global_instance(ddc:
                                                  &'static DynamicDeferredCall)
             -> bool {
                loop  { }
            }
            pub unsafe fn call_global_instance() -> bool { loop  { } }
            pub unsafe fn call_global_instance_while<F: Fn() -> bool>(f: F)
             -> bool {
                loop  { }
            }
            pub unsafe fn global_instance_calls_pending() -> Option<bool> {
                loop  { }
            }
            pub fn set(&self, handle: DeferredCallHandle) -> Option<bool> {
                loop  { }
            }
            pub fn register(&self,
                            ddc_client:
                                &'static dyn DynamicDeferredCallClient)
             -> Option<DeferredCallHandle> {
                loop  { }
            }
            pub fn has_pending(&self) -> bool { loop  { } }
            pub(self) fn call(&self) { loop  { } }
            pub(self) fn call_while<F: Fn() -> bool>(&self, f: F) {
                loop  { }
            }
        }
        pub trait DynamicDeferredCallClient {
            fn call(&self, handle: DeferredCallHandle);
        }
        pub struct DeferredCallHandle(usize);
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for DeferredCallHandle { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for DeferredCallHandle {
            #[inline]
            fn clone(&self) -> DeferredCallHandle { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DeferredCallHandle {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
    }
    pub mod list {
        use core::cell::Cell;
        pub struct ListLink<'a, T: 'a + ?Sized>(Cell<Option<&'a T>>);
        impl <T: ?Sized> ListLink<'a, T> {
            pub const fn empty() -> ListLink<'a, T> {
                ListLink(Cell::new(None))
            }
        }
        pub trait ListNode<'a, T: ?Sized> {
            fn next(&'a self)
            -> &'a ListLink<'a, T>;
        }
        pub struct List<'a, T: 'a + ?Sized + ListNode<'a, T>> {
            head: ListLink<'a, T>,
        }
        pub struct ListIterator<'a, T: 'a + ?Sized + ListNode<'a, T>> {
            cur: Option<&'a T>,
        }
        impl <T: ?Sized + ListNode<'a, T>> Iterator for ListIterator<'a, T> {
            type
            Item
            =
            &'a T;
            fn next(&mut self) -> Option<&'a T> { loop  { } }
        }
        impl <T: ?Sized + ListNode<'a, T>> List<'a, T> {
            pub const fn new() -> List<'a, T> {
                List{head: ListLink(Cell::new(None)),}
            }
            pub fn head(&self) -> Option<&'a T> { loop  { } }
            pub fn push_head(&self, node: &'a T) { loop  { } }
            pub fn push_tail(&self, node: &'a T) { loop  { } }
            pub fn pop_head(&self) -> Option<&'a T> { loop  { } }
            pub fn iter(&self) -> ListIterator<'a, T> { loop  { } }
        }
    }
    pub mod math {
        use core::convert::{From, Into};
        use core::intrinsics as int;
        pub fn sqrtf32(num: f32) -> f32 { loop  { } }
        extern "C" {
            fn __errno() -> &'static mut i32;
        }
        pub fn get_errno() -> i32 { loop  { } }
        pub fn closest_power_of_two(mut num: u32) -> u32 { loop  { } }
        pub struct PowerOfTwo(u32);
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for PowerOfTwo { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for PowerOfTwo {
            #[inline]
            fn clone(&self) -> PowerOfTwo { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for PowerOfTwo {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for PowerOfTwo {
            #[inline]
            fn eq(&self, other: &PowerOfTwo) -> bool { loop  { } }
            #[inline]
            fn ne(&self, other: &PowerOfTwo) -> bool { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialOrd for PowerOfTwo {
            #[inline]
            fn partial_cmp(&self, other: &PowerOfTwo)
             -> ::core::option::Option<::core::cmp::Ordering> {
                loop  { }
            }
            #[inline]
            fn lt(&self, other: &PowerOfTwo) -> bool { loop  { } }
            #[inline]
            fn le(&self, other: &PowerOfTwo) -> bool { loop  { } }
            #[inline]
            fn gt(&self, other: &PowerOfTwo) -> bool { loop  { } }
            #[inline]
            fn ge(&self, other: &PowerOfTwo) -> bool { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for PowerOfTwo {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Ord for PowerOfTwo {
            #[inline]
            fn cmp(&self, other: &PowerOfTwo) -> ::core::cmp::Ordering {
                loop  { }
            }
        }
        impl PowerOfTwo {
            pub fn exp<R>(self) -> R where R: From<u32> { loop  { } }
            pub fn floor<F: Into<u32>>(f: F) -> PowerOfTwo { loop  { } }
            pub fn ceiling<F: Into<u32>>(f: F) -> PowerOfTwo { loop  { } }
            pub fn zero() -> PowerOfTwo { loop  { } }
            pub fn as_num<F: From<u32>>(self) -> F { loop  { } }
        }
        pub fn log_base_two(num: u32) -> u32 { loop  { } }
        pub fn log_base_two_u64(num: u64) -> u32 { loop  { } }
    }
    pub mod peripherals {
        use crate::ClockInterface;
        pub trait PeripheralManagement<C> where C: ClockInterface {
            type
            RegisterType;
            fn get_registers(&self)
            -> &Self::RegisterType;
            fn get_clock(&self)
            -> &C;
            fn before_peripheral_access(&self, _: &C, _: &Self::RegisterType);
            fn after_peripheral_access(&self, _: &C, _: &Self::RegisterType);
        }
        pub struct PeripheralManager<'a, H, C> where H: 'a +
                   PeripheralManagement<C>, C: 'a + ClockInterface {
            pub registers: &'a H::RegisterType,
            peripheral_hardware: &'a H,
            clock: &'a C,
        }
        impl <H, C> PeripheralManager<'a, H, C> where H: 'a +
         PeripheralManagement<C>, C: 'a + ClockInterface {
            pub fn new(peripheral_hardware: &'a H)
             -> PeripheralManager<'a, H, C> {
                loop  { }
            }
        }
        impl <H, C> Drop for PeripheralManager<'a, H, C> where H: 'a +
         PeripheralManagement<C>, C: 'a + ClockInterface {
            fn drop(&mut self) { loop  { } }
        }
    }
    pub mod queue {
        pub trait Queue<T> {
            fn has_elements(&self)
            -> bool;
            fn is_full(&self)
            -> bool;
            fn len(&self)
            -> usize;
            fn enqueue(&mut self, val: T)
            -> bool;
            fn dequeue(&mut self)
            -> Option<T>;
            fn empty(&mut self);
            fn retain<F>(&mut self, f: F)
            where
            F: FnMut(&T)
            ->
            bool;
        }
    }
    pub mod ring_buffer {
        use crate::common::queue;
        pub struct RingBuffer<'a, T: 'a> {
            ring: &'a mut [T],
            head: usize,
            tail: usize,
        }
        impl <T: Copy> RingBuffer<'a, T> {
            pub fn new(ring: &'a mut [T]) -> RingBuffer<'a, T> { loop  { } }
            pub fn available_len(&self) -> usize { loop  { } }
            pub fn as_slices(&'a self) -> (Option<&'a [T]>, Option<&'a [T]>) {
                loop  { }
            }
        }
        impl <T: Copy> queue::Queue<T> for RingBuffer<'a, T> {
            fn has_elements(&self) -> bool { loop  { } }
            fn is_full(&self) -> bool { loop  { } }
            fn len(&self) -> usize { loop  { } }
            fn enqueue(&mut self, val: T) -> bool { loop  { } }
            fn dequeue(&mut self) -> Option<T> { loop  { } }
            fn empty(&mut self) { loop  { } }
            fn retain<F>(&mut self, mut f: F) where F: FnMut(&T) -> bool {
                loop  { }
            }
        }
    }
    pub mod utils {
        #[macro_export]
        macro_rules! static_init {
            ($ T : ty, $ e : expr) =>
            {
                {
                    use core :: { mem, ptr } ; static mut BUF : Option < $ T >
                    = None ; let tmp : & 'static mut $ T = mem :: transmute
                    (& mut BUF) ; ptr :: write (tmp as * mut $ T, $ e) ; tmp
                } ;
            }
        }
        #[macro_export]
        macro_rules! storage_volume {
            ($ N : ident, $ kB : expr) =>
            {
                # [link_section = ".storage"] # [used] pub static $ N :
                [u8 ; $ kB * 1024] = [0x00 ; $ kB * 1024] ;
            } ;
        }
        #[macro_export]
        macro_rules! create_capability {
            ($ T : ty) =>
            { { struct Cap ; unsafe impl $ T for Cap { } Cap } ; } ;
        }
    }
    mod static_ref {
        use core::ops::Deref;
        pub struct StaticRef<T> {
            ptr: *const T,
        }
        #[allow(unused_qualifications)]
        impl <T: ::core::fmt::Debug> ::core::fmt::Debug for StaticRef<T> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        impl <T> StaticRef<T> {
            pub const unsafe fn new(ptr: *const T) -> StaticRef<T> {
                StaticRef{ptr: ptr,}
            }
        }
        impl <T> Clone for StaticRef<T> {
            fn clone(&self) -> Self { loop  { } }
        }
        impl <T> Copy for StaticRef<T> { }
        impl <T> Deref for StaticRef<T> {
            type
            Target
            =
            T;
            fn deref(&self) -> &'static T { loop  { } }
        }
    }
    pub use self::list::{List, ListLink, ListNode};
    pub use self::queue::Queue;
    pub use self::ring_buffer::RingBuffer;
    pub use self::static_ref::StaticRef;
    pub mod cells {
        pub use tock_cells::map_cell::MapCell;
        pub use tock_cells::numeric_cell_ext::NumericCellExt;
        pub use tock_cells::optional_cell::OptionalCell;
        pub use tock_cells::take_cell::TakeCell;
        pub use tock_cells::volatile_cell::VolatileCell;
    }
}
pub mod component {
    pub trait Component {
        type
        StaticInput
        =
        ();
        type
        Output;
        unsafe fn finalize(&mut self, static_memory: Self::StaticInput)
        -> Self::Output;
    }
}
pub mod debug {
    use core::cell::Cell;
    use core::fmt::{write, Arguments, Result, Write};
    use core::panic::PanicInfo;
    use core::ptr;
    use core::str;
    use crate::common::cells::NumericCellExt;
    use crate::common::cells::{MapCell, TakeCell};
    use crate::common::queue::Queue;
    use crate::common::ring_buffer::RingBuffer;
    use crate::hil;
    use crate::process::ProcessType;
    use crate::ReturnCode;
    pub unsafe fn panic<L: hil::led::Led,
                        W: Write>(leds: &mut [&mut L], writer: &mut W,
                                  panic_info: &PanicInfo, nop: &dyn Fn(),
                                  processes:
                                      &'static [Option<&'static dyn ProcessType>])
     -> ! {
        loop  { }
    }
    pub unsafe fn panic_begin(nop: &dyn Fn()) { loop  { } }
    pub unsafe fn panic_banner<W: Write>(writer: &mut W,
                                         panic_info: &PanicInfo) {
        loop  { }
    }
    pub unsafe fn panic_process_info<W: Write>(procs:
                                                   &'static [Option<&'static dyn ProcessType>],
                                               writer: &mut W) {
        loop  { }
    }
    pub fn panic_blink_forever<L: hil::led::Led>(leds: &mut [&mut L]) -> ! {
        loop  { }
    }
    pub static mut DEBUG_GPIOS:
               (Option<&'static dyn hil::gpio::Pin>,
                Option<&'static dyn hil::gpio::Pin>,
                Option<&'static dyn hil::gpio::Pin>) =
        (None, None, None);
    pub unsafe fn assign_gpios(gpio0: Option<&'static dyn hil::gpio::Pin>,
                               gpio1: Option<&'static dyn hil::gpio::Pin>,
                               gpio2: Option<&'static dyn hil::gpio::Pin>) {
        loop  { }
    }
    #[macro_export]
    macro_rules! debug_gpio {
        ($ i : tt, $ method : ident) =>
        {
            {
                # [allow (unused_unsafe)] unsafe
                {
                    $ crate :: debug :: DEBUG_GPIOS . $ i . map
                    (| g | g . $ method ()) ;
                }
            }
        } ;
    }
    pub struct DebugWriterWrapper {
        dw: MapCell<&'static DebugWriter>,
    }
    pub struct DebugWriter {
        uart: &'static dyn hil::uart::Transmit<'static>,
        output_buffer: TakeCell<'static, [u8]>,
        internal_buffer: TakeCell<'static, RingBuffer<'static, u8>>,
        count: Cell<usize>,
    }
    static mut DEBUG_WRITER: Option<&'static mut DebugWriterWrapper> = None;
    pub static mut OUTPUT_BUF: [u8; 64] = [0; 64];
    pub static mut INTERNAL_BUF: [u8; 1024] = [0; 1024];
    pub unsafe fn get_debug_writer() -> &'static mut DebugWriterWrapper {
        loop  { }
    }
    pub unsafe fn set_debug_writer_wrapper(debug_writer:
                                               &'static mut DebugWriterWrapper) {
        loop  { }
    }
    impl DebugWriterWrapper {
        pub fn new(dw: &'static DebugWriter) -> DebugWriterWrapper {
            loop  { }
        }
    }
    impl DebugWriter {
        pub fn new(uart: &'static dyn hil::uart::Transmit,
                   out_buffer: &'static mut [u8],
                   internal_buffer: &'static mut RingBuffer<'static, u8>)
         -> DebugWriter {
            loop  { }
        }
        fn increment_count(&self) { loop  { } }
        fn get_count(&self) -> usize { loop  { } }
        fn publish_str(&self) { loop  { } }
        fn extract(&self) -> Option<&mut RingBuffer<'static, u8>> {
            loop  { }
        }
    }
    impl hil::uart::TransmitClient for DebugWriter {
        fn transmitted_buffer(&self, buffer: &'static mut [u8],
                              _tx_len: usize, _rcode: ReturnCode) {
            loop  { }
        }
        fn transmitted_word(&self, _rcode: ReturnCode) { loop  { } }
    }
    impl DebugWriterWrapper {
        fn increment_count(&self) { loop  { } }
        fn get_count(&self) -> usize { loop  { } }
        fn publish_str(&self) { loop  { } }
        fn extract(&self) -> Option<&mut RingBuffer<'static, u8>> {
            loop  { }
        }
    }
    impl Write for DebugWriterWrapper {
        fn write_str(&mut self, s: &str) -> Result {
            const FULL_MSG: &[u8] = b"\n*** DEBUG BUFFER FULL ***\n";
            loop  { }
        }
    }
    pub fn begin_debug_fmt(args: Arguments) { loop  { } }
    pub fn begin_debug_verbose_fmt(args: Arguments,
                                   file_line: &(&'static str, u32)) {
        loop  { }
    }
    #[macro_export]
    macro_rules! debug {
        () => ({ debug ! ("") }) ; ($ msg : expr) =>
        ({ $ crate :: debug :: begin_debug_fmt (format_args ! ($ msg)) }) ;
        ($ fmt : expr, $ ($ arg : tt) +) =>
        ({
             $ crate :: debug :: begin_debug_fmt
             (format_args ! ($ fmt, $ ($ arg) +))
         }) ;
    }
    #[macro_export]
    macro_rules! debug_verbose {
        () => ({ debug_verbose ! ("") }) ; ($ msg : expr) =>
        ({
             $ crate :: debug :: begin_debug_verbose_fmt
             (format_args ! ($ msg),
              {
                  static _FILE_LINE : (& 'static str, u32) =
                  (file ! (), line ! ()) ; & _FILE_LINE
              })
         }) ; ($ fmt : expr, $ ($ arg : tt) +) =>
        ({
             $ crate :: debug :: begin_debug_verbose_fmt
             (format_args ! ($ fmt, $ ($ arg) +),
              {
                  static _FILE_LINE : (& 'static str, u32) =
                  (file ! (), line ! ()) ; & _FILE_LINE
              })
         }) ;
    }
    pub trait Debug {
        fn write(&self, buf: &'static mut [u8], len: usize);
    }
    pub unsafe fn flush<W: Write>(writer: &mut W) { loop  { } }
}
pub mod hil {
    pub mod adc {
        use crate::returncode::ReturnCode;
        pub trait Adc {
            type
            Channel;
            fn sample(&self, channel: &Self::Channel)
            -> ReturnCode;
            fn sample_continuous(&self, channel: &Self::Channel,
                                 frequency: u32)
            -> ReturnCode;
            fn stop_sampling(&self)
            -> ReturnCode;
            fn get_resolution_bits(&self)
            -> usize;
            fn get_voltage_reference_mv(&self)
            -> Option<usize>;
        }
        pub trait Client {
            fn sample_ready(&self, sample: u16);
        }
        pub trait AdcHighSpeed: Adc {
            fn sample_highspeed(&self, channel: &Self::Channel,
                                frequency: u32, buffer1: &'static mut [u16],
                                length1: usize, buffer2: &'static mut [u16],
                                length2: usize)
            ->
                (ReturnCode, Option<&'static mut [u16]>,
                 Option<&'static mut [u16]>);
            fn provide_buffer(&self, buf: &'static mut [u16], length: usize)
            -> (ReturnCode, Option<&'static mut [u16]>);
            fn retrieve_buffers(&self)
            ->
                (ReturnCode, Option<&'static mut [u16]>,
                 Option<&'static mut [u16]>);
        }
        pub trait HighSpeedClient {
            fn samples_ready(&self, buf: &'static mut [u16], length: usize);
        }
    }
    pub mod analog_comparator {
        use crate::returncode::ReturnCode;
        pub trait AnalogComparator {
            type
            Channel;
            fn comparison(&self, channel: &Self::Channel)
            -> bool;
            fn start_comparing(&self, channel: &Self::Channel)
            -> ReturnCode;
            fn stop_comparing(&self, channel: &Self::Channel)
            -> ReturnCode;
        }
        pub trait Client {
            fn fired(&self, _: usize);
        }
    }
    pub mod ble_advertising {
        use crate::returncode::ReturnCode;
        pub trait BleAdvertisementDriver {
            fn transmit_advertisement(&self, buf: &'static mut [u8],
                                      len: usize, channel: RadioChannel)
            -> &'static mut [u8];
            fn receive_advertisement(&self, channel: RadioChannel);
            fn set_receive_client(&self, client: &'static dyn RxClient);
            fn set_transmit_client(&self, client: &'static dyn TxClient);
        }
        pub trait BleConfig {
            fn set_tx_power(&self, power: u8)
            -> ReturnCode;
        }
        pub trait RxClient {
            fn receive_event(&self, buf: &'static mut [u8], len: u8,
                             result: ReturnCode);
        }
        pub trait TxClient {
            fn transmit_event(&self, result: ReturnCode);
        }
        pub enum RadioChannel {
            DataChannel0 = 4,
            DataChannel1 = 6,
            DataChannel2 = 8,
            DataChannel3 = 10,
            DataChannel4 = 12,
            DataChannel5 = 14,
            DataChannel6 = 16,
            DataChannel7 = 18,
            DataChannel8 = 20,
            DataChannel9 = 22,
            DataChannel10 = 24,
            DataChannel11 = 28,
            DataChannel12 = 30,
            DataChannel13 = 32,
            DataChannel14 = 34,
            DataChannel15 = 36,
            DataChannel16 = 38,
            DataChannel17 = 40,
            DataChannel18 = 42,
            DataChannel19 = 44,
            DataChannel20 = 46,
            DataChannel21 = 48,
            DataChannel22 = 50,
            DataChannel23 = 52,
            DataChannel24 = 54,
            DataChannel25 = 56,
            DataChannel26 = 58,
            DataChannel27 = 60,
            DataChannel28 = 62,
            DataChannel29 = 64,
            DataChannel30 = 66,
            DataChannel31 = 68,
            DataChannel32 = 70,
            DataChannel33 = 72,
            DataChannel34 = 74,
            DataChannel35 = 76,
            DataChannel36 = 78,
            AdvertisingChannel37 = 2,
            AdvertisingChannel38 = 26,
            AdvertisingChannel39 = 80,
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for RadioChannel {
            #[inline]
            fn eq(&self, other: &RadioChannel) -> bool { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for RadioChannel {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for RadioChannel { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for RadioChannel {
            #[inline]
            fn clone(&self) -> RadioChannel { loop  { } }
        }
        impl RadioChannel {
            pub fn get_channel_index(&self) -> u32 { loop  { } }
        }
    }
    pub mod crc {
        use crate::returncode::ReturnCode;
        pub enum CrcAlg {

            Crc32,

            Crc32C,

            Sam4L16,

            Sam4L32,

            Sam4L32C,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for CrcAlg { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for CrcAlg {
            #[inline]
            fn clone(&self) -> CrcAlg { loop  { } }
        }
        pub trait CRC {
            fn compute(&self, data: &[u8], _: CrcAlg)
            -> ReturnCode;
            fn disable(&self);
        }
        pub trait Client {
            fn receive_result(&self, _: u32);
        }
    }
    pub mod dac {
        use crate::returncode::ReturnCode;
        pub trait DacChannel {
            fn initialize(&self)
            -> ReturnCode;
            fn set_value(&self, value: usize)
            -> ReturnCode;
        }
    }
    pub mod eic {
        pub enum InterruptMode {
            RisingEdge,
            FallingEdge,
            HighLevel,
            LowLevel,
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for InterruptMode {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub trait ExternalInterruptController {
            type
            Line;
            fn line_enable(&self, line: &Self::Line,
                           interrupt_mode: InterruptMode);
            fn line_disable(&self, line: &Self::Line);
        }
        pub trait Client {
            fn fired(&self);
        }
    }
    pub mod entropy {
        use crate::returncode::ReturnCode;
        pub enum Continue {

            More,

            Done,
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Continue {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Continue {
            #[inline]
            fn eq(&self, other: &Continue) -> bool { loop  { } }
        }
        pub trait Entropy32<'a> {
            fn get(&self)
            -> ReturnCode;
            fn cancel(&self)
            -> ReturnCode;
            fn set_client(&'a self, _: &'a dyn Client32);
        }
        pub trait Client32 {
            fn entropy_available(&self,
                                 entropy: &mut dyn Iterator<Item = u32>,
                                 error: ReturnCode)
            -> Continue;
        }
        pub trait Entropy8<'a> {
            fn get(&self)
            -> ReturnCode;
            fn cancel(&self)
            -> ReturnCode;
            fn set_client(&'a self, _: &'a dyn Client8);
        }
        pub trait Client8 {
            fn entropy_available(&self, entropy: &mut dyn Iterator<Item = u8>,
                                 error: ReturnCode)
            -> Continue;
        }
    }
    pub mod flash {
        use crate::returncode::ReturnCode;
        pub enum Error {

            CommandComplete,

            FlashError,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool { loop  { } }
        }
        pub trait HasClient<'a, C> {
            fn set_client(&'a self, client: &'a C);
        }
        pub trait Flash {
            type
            Page: AsMut<[u8]>;
            fn read_page(&self, page_number: usize,
                         buf: &'static mut Self::Page)
            -> ReturnCode;
            fn write_page(&self, page_number: usize,
                          buf: &'static mut Self::Page)
            -> ReturnCode;
            fn erase_page(&self, page_number: usize)
            -> ReturnCode;
        }
        pub trait Client<F: Flash> {
            fn read_complete(&self, read_buffer: &'static mut F::Page,
                             error: Error);
            fn write_complete(&self, write_buffer: &'static mut F::Page,
                              error: Error);
            fn erase_complete(&self, error: Error);
        }
    }
    pub mod gpio {
        use crate::common::cells::OptionalCell;
        use crate::ReturnCode;
        use core::cell::Cell;
        pub enum FloatingState { PullUp, PullDown, PullNone, }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for FloatingState {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub enum InterruptEdge { RisingEdge, FallingEdge, EitherEdge, }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for InterruptEdge {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub enum Configuration {

            LowPower,

            Input,

            Output,

            InputOutput,

            Function,

            Other,
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Configuration {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub trait Pin: Input + Output + Configure { }
        pub trait InterruptPin: Pin + Interrupt { }
        pub trait InterruptValuePin: Pin + InterruptWithValue { }
        pub trait Configure {
            fn configuration(&self)
            -> Configuration;
            fn make_output(&self)
            -> Configuration;
            fn disable_output(&self)
            -> Configuration;
            fn make_input(&self)
            -> Configuration;
            fn disable_input(&self)
            -> Configuration;
            fn deactivate_to_low_power(&self);
            fn set_floating_state(&self, state: FloatingState);
            fn floating_state(&self)
            -> FloatingState;
            fn is_input(&self) -> bool { loop  { } }
            fn is_output(&self) -> bool { loop  { } }
        }
        pub trait ConfigureInputOutput: Configure {
            fn make_input_output(&self)
            -> Configuration;
            fn is_input_output(&self)
            -> bool;
        }
        pub trait Output {
            fn set(&self);
            fn clear(&self);
            fn toggle(&self)
            -> bool;
        }
        pub trait Input {
            fn read(&self)
            -> bool;
        }
        pub trait Interrupt: Input {
            fn set_client(&self, client: &'static dyn Client);
            fn enable_interrupts(&self, mode: InterruptEdge);
            fn disable_interrupts(&self);
            fn is_pending(&self)
            -> bool;
        }
        pub trait Client {
            fn fired(&self);
        }
        pub trait InterruptWithValue: Input {
            fn set_client(&self, client: &'static dyn ClientWithValue);
            fn enable_interrupts(&self, mode: InterruptEdge)
            -> ReturnCode;
            fn disable_interrupts(&self);
            fn is_pending(&self)
            -> bool;
            fn set_value(&self, value: u32);
            fn value(&self)
            -> u32;
        }
        pub trait ClientWithValue {
            fn fired(&self, value: u32);
        }
        pub struct InterruptValueWrapper {
            value: Cell<u32>,
            client: OptionalCell<&'static dyn ClientWithValue>,
            source: &'static dyn InterruptPin,
        }
        impl InterruptValueWrapper {
            pub fn new(pin: &'static dyn InterruptPin)
             -> InterruptValueWrapper {
                loop  { }
            }
            pub fn finalize(&'static self) -> &'static Self { loop  { } }
        }
        impl InterruptWithValue for InterruptValueWrapper {
            fn set_value(&self, value: u32) { loop  { } }
            fn value(&self) -> u32 { loop  { } }
            fn set_client(&self, client: &'static dyn ClientWithValue) {
                loop  { }
            }
            fn is_pending(&self) -> bool { loop  { } }
            fn enable_interrupts(&self, edge: InterruptEdge) -> ReturnCode {
                loop  { }
            }
            fn disable_interrupts(&self) { loop  { } }
        }
        impl Input for InterruptValueWrapper {
            fn read(&self) -> bool { loop  { } }
        }
        impl Configure for InterruptValueWrapper {
            fn configuration(&self) -> Configuration { loop  { } }
            fn make_output(&self) -> Configuration { loop  { } }
            fn disable_output(&self) -> Configuration { loop  { } }
            fn make_input(&self) -> Configuration { loop  { } }
            fn disable_input(&self) -> Configuration { loop  { } }
            fn deactivate_to_low_power(&self) { loop  { } }
            fn set_floating_state(&self, state: FloatingState) { loop  { } }
            fn floating_state(&self) -> FloatingState { loop  { } }
            fn is_input(&self) -> bool { loop  { } }
            fn is_output(&self) -> bool { loop  { } }
        }
        impl Output for InterruptValueWrapper {
            fn set(&self) { loop  { } }
            fn clear(&self) { loop  { } }
            fn toggle(&self) -> bool { loop  { } }
        }
        impl InterruptValuePin for InterruptValueWrapper { }
        impl Pin for InterruptValueWrapper { }
        impl Client for InterruptValueWrapper {
            fn fired(&self) { loop  { } }
        }
    }
    pub mod gpio_async {
        use crate::hil;
        use crate::returncode::ReturnCode;
        pub trait Port {
            fn disable(&self, pin: usize)
            -> ReturnCode;
            fn make_output(&self, pin: usize)
            -> ReturnCode;
            fn make_input(&self, pin: usize, mode: hil::gpio::FloatingState)
            -> ReturnCode;
            fn read(&self, pin: usize)
            -> ReturnCode;
            fn toggle(&self, pin: usize)
            -> ReturnCode;
            fn set(&self, pin: usize)
            -> ReturnCode;
            fn clear(&self, pin: usize)
            -> ReturnCode;
            fn enable_interrupt(&self, pin: usize,
                                mode: hil::gpio::InterruptEdge)
            -> ReturnCode;
            fn disable_interrupt(&self, pin: usize)
            -> ReturnCode;
            fn is_pending(&self, pin: usize)
            -> bool;
        }
        pub trait Client {
            fn fired(&self, pin: usize, identifier: usize);
            fn done(&self, value: usize);
        }
    }
    pub mod i2c {
        use core::fmt::{Display, Formatter, Result};
        pub enum Error {

            AddressNak,

            DataNak,

            ArbitrationLost,

            Overrun,

            CommandComplete,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool { loop  { } }
        }
        impl Display for Error {
            fn fmt(&self, fmt: &mut Formatter) -> Result { loop  { } }
        }
        pub enum SlaveTransmissionType { Write, Read, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for SlaveTransmissionType { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for SlaveTransmissionType {
            #[inline]
            fn clone(&self) -> SlaveTransmissionType { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for SlaveTransmissionType {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub trait I2CMaster {
            fn enable(&self);
            fn disable(&self);
            fn write_read(&self, addr: u8, data: &'static mut [u8],
                          write_len: u8, read_len: u8);
            fn write(&self, addr: u8, data: &'static mut [u8], len: u8);
            fn read(&self, addr: u8, buffer: &'static mut [u8], len: u8);
        }
        pub trait I2CSlave {
            fn enable(&self);
            fn disable(&self);
            fn set_address(&self, addr: u8);
            fn write_receive(&self, data: &'static mut [u8], max_len: u8);
            fn read_send(&self, data: &'static mut [u8], max_len: u8);
            fn listen(&self);
        }
        pub trait I2CMasterSlave: I2CMaster + I2CSlave { }
        pub trait I2CHwMasterClient {
            fn command_complete(&self, buffer: &'static mut [u8],
                                error: Error);
        }
        pub trait I2CHwSlaveClient {
            fn command_complete(&self, buffer: &'static mut [u8], length: u8,
                                transmission_type: SlaveTransmissionType);
            fn read_expected(&self);
            fn write_expected(&self);
        }
        pub trait I2CDevice {
            fn enable(&self);
            fn disable(&self);
            fn write_read(&self, data: &'static mut [u8], write_len: u8,
                          read_len: u8);
            fn write(&self, data: &'static mut [u8], len: u8);
            fn read(&self, buffer: &'static mut [u8], len: u8);
        }
        pub trait I2CClient {
            fn command_complete(&self, buffer: &'static mut [u8],
                                error: Error);
        }
    }
    pub mod led {
        use crate::hil::gpio;
        pub trait Led {
            fn init(&mut self);
            fn on(&mut self);
            fn off(&mut self);
            fn toggle(&mut self);
            fn read(&self)
            -> bool;
        }
        pub struct LedHigh<'a> {
            pub pin: &'a mut dyn gpio::Pin,
        }
        pub struct LedLow<'a> {
            pub pin: &'a mut dyn gpio::Pin,
        }
        impl LedHigh<'a> {
            pub fn new(p: &'a mut dyn gpio::Pin) -> LedHigh { loop  { } }
        }
        impl LedLow<'a> {
            pub fn new(p: &'a mut dyn gpio::Pin) -> LedLow { loop  { } }
        }
        impl Led for LedHigh<'a> {
            fn init(&mut self) { loop  { } }
            fn on(&mut self) { loop  { } }
            fn off(&mut self) { loop  { } }
            fn toggle(&mut self) { loop  { } }
            fn read(&self) -> bool { loop  { } }
        }
        impl Led for LedLow<'a> {
            fn init(&mut self) { loop  { } }
            fn on(&mut self) { loop  { } }
            fn off(&mut self) { loop  { } }
            fn toggle(&mut self) { loop  { } }
            fn read(&self) -> bool { loop  { } }
        }
    }
    pub mod nonvolatile_storage {
        use crate::returncode::ReturnCode;
        pub trait NonvolatileStorage<'a> {
            fn set_client(&self,
                          client: &'a dyn NonvolatileStorageClient<'a>);
            fn read(&self, buffer: &'a mut [u8], address: usize,
                    length: usize)
            -> ReturnCode;
            fn write(&self, buffer: &'a mut [u8], address: usize,
                     length: usize)
            -> ReturnCode;
        }
        pub trait NonvolatileStorageClient<'a> {
            fn read_done(&self, buffer: &'a mut [u8], length: usize);
            fn write_done(&self, buffer: &'a mut [u8], length: usize);
        }
    }
    pub mod pwm {
        use crate::returncode::ReturnCode;
        pub trait Pwm {
            type
            Pin;
            fn start(&self, pin: &Self::Pin, frequency_hz: usize,
                     duty_cycle: usize)
            -> ReturnCode;
            fn stop(&self, pin: &Self::Pin)
            -> ReturnCode;
            fn get_maximum_frequency_hz(&self)
            -> usize;
            fn get_maximum_duty_cycle(&self)
            -> usize;
        }
        pub trait PwmPin {
            fn start(&self, frequency_hz: usize, duty_cycle: usize)
            -> ReturnCode;
            fn stop(&self)
            -> ReturnCode;
            fn get_maximum_frequency_hz(&self)
            -> usize;
            fn get_maximum_duty_cycle(&self)
            -> usize;
        }
    }
    pub mod radio {
        use crate::returncode::ReturnCode;
        pub trait TxClient {
            fn send_done(&self, buf: &'static mut [u8], acked: bool,
                         result: ReturnCode);
        }
        pub trait RxClient {
            fn receive(&self, buf: &'static mut [u8], frame_len: usize,
                       crc_valid: bool, result: ReturnCode);
        }
        pub trait ConfigClient {
            fn config_done(&self, result: ReturnCode);
        }
        pub trait PowerClient {
            fn changed(&self, on: bool);
        }
        pub const MIN_MHR_SIZE: usize = 9;
        pub const MFR_SIZE: usize = 2;
        pub const MAX_MTU: usize = 127;
        pub const MIN_FRAME_SIZE: usize = MIN_MHR_SIZE + MFR_SIZE;
        pub const MAX_FRAME_SIZE: usize = MAX_MTU;
        pub const PSDU_OFFSET: usize = 2;
        pub const MAX_BUF_SIZE: usize = PSDU_OFFSET + MAX_MTU;
        pub const MIN_PAYLOAD_OFFSET: usize = PSDU_OFFSET + MIN_MHR_SIZE;
        pub trait Radio: RadioConfig + RadioData { }
        pub trait RadioConfig {
            fn initialize(&self, spi_buf: &'static mut [u8],
                          reg_write: &'static mut [u8],
                          reg_read: &'static mut [u8])
            -> ReturnCode;
            fn reset(&self)
            -> ReturnCode;
            fn start(&self)
            -> ReturnCode;
            fn stop(&self)
            -> ReturnCode;
            fn is_on(&self)
            -> bool;
            fn busy(&self)
            -> bool;
            fn set_power_client(&self, client: &'static dyn PowerClient);
            fn config_commit(&self);
            fn set_config_client(&self, client: &'static dyn ConfigClient);
            fn get_address(&self)
            -> u16;
            fn get_address_long(&self)
            -> [u8; 8];
            fn get_pan(&self)
            -> u16;
            fn get_tx_power(&self)
            -> i8;
            fn get_channel(&self)
            -> u8;
            fn set_address(&self, addr: u16);
            fn set_address_long(&self, addr: [u8; 8]);
            fn set_pan(&self, id: u16);
            fn set_tx_power(&self, power: i8)
            -> ReturnCode;
            fn set_channel(&self, chan: u8)
            -> ReturnCode;
        }
        pub trait RadioData {
            fn set_transmit_client(&self, client: &'static dyn TxClient);
            fn set_receive_client(&self, client: &'static dyn RxClient,
                                  receive_buffer: &'static mut [u8]);
            fn set_receive_buffer(&self, receive_buffer: &'static mut [u8]);
            fn transmit(&self, spi_buf: &'static mut [u8], frame_len: usize)
            -> (ReturnCode, Option<&'static mut [u8]>);
        }
    }
    pub mod rng {
        use crate::returncode::ReturnCode;
        pub enum Continue {

            More,

            Done,
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Continue {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Continue {
            #[inline]
            fn eq(&self, other: &Continue) -> bool { loop  { } }
        }
        pub trait Rng<'a> {
            fn get(&self)
            -> ReturnCode;
            fn cancel(&self)
            -> ReturnCode;
            fn set_client(&'a self, _: &'a dyn Client);
        }
        pub trait Client {
            fn randomness_available(&self,
                                    randomness: &mut dyn Iterator<Item = u32>,
                                    error: ReturnCode)
            -> Continue;
        }
        pub trait Random<'a> {
            fn initialize(&'a self);
            fn reseed(&self, seed: u32);
            fn random(&self)
            -> u32;
        }
    }
    pub mod sensors {
        use crate::returncode::ReturnCode;
        pub trait TemperatureDriver {
            fn set_client(&self, client: &'static dyn TemperatureClient);
            fn read_temperature(&self)
            -> ReturnCode;
        }
        pub trait TemperatureClient {
            fn callback(&self, value: usize);
        }
        pub trait HumidityDriver {
            fn set_client(&self, client: &'static dyn HumidityClient);
            fn read_humidity(&self)
            -> ReturnCode;
        }
        pub trait HumidityClient {
            fn callback(&self, value: usize);
        }
        pub trait AmbientLight {
            fn set_client(&self, client: &'static dyn AmbientLightClient);
            fn read_light_intensity(&self) -> ReturnCode { loop  { } }
        }
        pub trait AmbientLightClient {
            fn callback(&self, lux: usize);
        }
        pub trait NineDof {
            fn set_client(&self, client: &'static dyn NineDofClient);
            fn read_accelerometer(&self) -> ReturnCode { loop  { } }
            fn read_magnetometer(&self) -> ReturnCode { loop  { } }
            fn read_gyroscope(&self) -> ReturnCode { loop  { } }
        }
        pub trait NineDofClient {
            fn callback(&self, arg1: usize, arg2: usize, arg3: usize);
        }
    }
    pub mod spi {
        use crate::returncode::ReturnCode;
        use core::option::Option;
        pub enum DataOrder { MSBFirst, LSBFirst, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for DataOrder { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for DataOrder {
            #[inline]
            fn clone(&self) -> DataOrder { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DataOrder {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub enum ClockPolarity { IdleLow, IdleHigh, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPolarity { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPolarity {
            #[inline]
            fn clone(&self) -> ClockPolarity { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for ClockPolarity {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for ClockPolarity {
            #[inline]
            fn eq(&self, other: &ClockPolarity) -> bool { loop  { } }
        }
        pub enum ClockPhase { SampleLeading, SampleTrailing, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPhase { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPhase {
            #[inline]
            fn clone(&self) -> ClockPhase { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for ClockPhase {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for ClockPhase {
            #[inline]
            fn eq(&self, other: &ClockPhase) -> bool { loop  { } }
        }
        pub trait SpiMasterClient {
            fn read_write_done(&self, write_buffer: &'static mut [u8],
                               read_buffer: Option<&'static mut [u8]>,
                               len: usize);
        }
        pub trait SpiMaster {
            type
            ChipSelect: Copy;
            fn set_client(&self, client: &'static dyn SpiMasterClient);
            fn init(&self);
            fn is_busy(&self)
            -> bool;
            fn read_write_bytes(&self, write_buffer: &'static mut [u8],
                                read_buffer: Option<&'static mut [u8]>,
                                len: usize)
            -> ReturnCode;
            fn write_byte(&self, val: u8);
            fn read_byte(&self)
            -> u8;
            fn read_write_byte(&self, val: u8)
            -> u8;
            fn specify_chip_select(&self, cs: Self::ChipSelect);
            fn set_rate(&self, rate: u32)
            -> u32;
            fn get_rate(&self)
            -> u32;
            fn set_clock(&self, polarity: ClockPolarity);
            fn get_clock(&self)
            -> ClockPolarity;
            fn set_phase(&self, phase: ClockPhase);
            fn get_phase(&self)
            -> ClockPhase;
            fn hold_low(&self);
            fn release_low(&self);
        }
        pub trait SpiMasterDevice {
            fn configure(&self, cpol: ClockPolarity, cpal: ClockPhase,
                         rate: u32);
            fn read_write_bytes(&self, write_buffer: &'static mut [u8],
                                read_buffer: Option<&'static mut [u8]>,
                                len: usize)
            -> ReturnCode;
            fn set_polarity(&self, cpol: ClockPolarity);
            fn set_phase(&self, cpal: ClockPhase);
            fn set_rate(&self, rate: u32);
            fn get_polarity(&self)
            -> ClockPolarity;
            fn get_phase(&self)
            -> ClockPhase;
            fn get_rate(&self)
            -> u32;
        }
        pub trait SpiSlaveClient {
            fn chip_selected(&self);
            fn read_write_done(&self, write_buffer: Option<&'static mut [u8]>,
                               read_buffer: Option<&'static mut [u8]>,
                               len: usize);
        }
        pub trait SpiSlave {
            fn init(&self);
            fn has_client(&self)
            -> bool;
            fn set_client(&self, client: Option<&'static dyn SpiSlaveClient>);
            fn set_write_byte(&self, write_byte: u8);
            fn read_write_bytes(&self,
                                write_buffer: Option<&'static mut [u8]>,
                                read_buffer: Option<&'static mut [u8]>,
                                len: usize)
            -> ReturnCode;
            fn set_clock(&self, polarity: ClockPolarity);
            fn get_clock(&self)
            -> ClockPolarity;
            fn set_phase(&self, phase: ClockPhase);
            fn get_phase(&self)
            -> ClockPhase;
        }
        pub trait SpiSlaveDevice {
            fn configure(&self, cpol: ClockPolarity, cpal: ClockPhase);
            fn read_write_bytes(&self,
                                write_buffer: Option<&'static mut [u8]>,
                                read_buffer: Option<&'static mut [u8]>,
                                len: usize)
            -> ReturnCode;
            fn set_polarity(&self, cpol: ClockPolarity);
            fn get_polarity(&self)
            -> ClockPolarity;
            fn set_phase(&self, cpal: ClockPhase);
            fn get_phase(&self)
            -> ClockPhase;
        }
    }
    pub mod symmetric_encryption {
        use crate::returncode::ReturnCode;
        pub trait Client<'a> {
            fn crypt_done(&'a self, source: Option<&'a mut [u8]>,
                          dest: &'a mut [u8]);
        }
        pub const AES128_BLOCK_SIZE: usize = 16;
        pub const AES128_KEY_SIZE: usize = 16;
        pub trait AES128<'a> {
            fn enable(&self);
            fn disable(&self);
            fn set_client(&'a self, client: &'a dyn Client<'a>);
            fn set_key(&self, key: &[u8])
            -> ReturnCode;
            fn set_iv(&self, iv: &[u8])
            -> ReturnCode;
            fn start_message(&self);
            fn crypt(&'a self, source: Option<&'a mut [u8]>,
                     dest: &'a mut [u8], start_index: usize,
                     stop_index: usize)
            -> Option<(ReturnCode, Option<&'a mut [u8]>, &'a mut [u8])>;
        }
        pub trait AES128Ctr {
            fn set_mode_aes128ctr(&self, encrypting: bool);
        }
        pub trait AES128CBC {
            fn set_mode_aes128cbc(&self, encrypting: bool);
        }
        pub trait CCMClient {
            fn crypt_done(&self, buf: &'static mut [u8], res: ReturnCode,
                          tag_is_valid: bool);
        }
        pub const CCM_NONCE_LENGTH: usize = 13;
        pub trait AES128CCM<'a> {
            fn set_client(&'a self, client: &'a dyn CCMClient);
            fn set_key(&self, key: &[u8])
            -> ReturnCode;
            fn set_nonce(&self, nonce: &[u8])
            -> ReturnCode;
            fn crypt(&self, buf: &'static mut [u8], a_off: usize,
                     m_off: usize, m_len: usize, mic_len: usize,
                     confidential: bool, encrypting: bool)
            -> (ReturnCode, Option<&'static mut [u8]>);
        }
    }
    pub mod time {
        use crate::ReturnCode;
        pub trait Time<W = u32> {
            type
            Frequency: Frequency;
            fn now(&self)
            -> W;
            fn max_tics(&self)
            -> W;
        }
        pub trait Counter<W = u32>: Time<W> {
            fn start(&self)
            -> ReturnCode;
            fn stop(&self)
            -> ReturnCode;
            fn is_running(&self)
            -> bool;
        }
        pub trait Frequency {
            fn frequency()
            -> u32;
        }
        pub struct Freq16MHz;
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq16MHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        impl Frequency for Freq16MHz {
            fn frequency() -> u32 { loop  { } }
        }
        pub struct Freq32KHz;
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq32KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        impl Frequency for Freq32KHz {
            fn frequency() -> u32 { loop  { } }
        }
        pub struct Freq16KHz;
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq16KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        impl Frequency for Freq16KHz {
            fn frequency() -> u32 { loop  { } }
        }
        pub struct Freq1KHz;
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq1KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        impl Frequency for Freq1KHz {
            fn frequency() -> u32 { loop  { } }
        }
        pub trait Alarm<'a, W = u32>: Time<W> {
            fn set_alarm(&self, tics: W);
            fn get_alarm(&self)
            -> W;
            fn set_client(&'a self, client: &'a dyn AlarmClient);
            fn is_enabled(&self)
            -> bool;
            fn enable(&self) { loop  { } }
            fn disable(&self);
        }
        pub trait AlarmClient {
            fn fired(&self);
        }
        pub trait Timer<'a, W = u32>: Time<W> {
            fn set_client(&'a self, client: &'a dyn TimerClient);
            fn oneshot(&self, interval: W);
            fn repeat(&self, interval: W);
            fn interval(&self)
            -> Option<W>;
            fn is_oneshot(&self) -> bool { loop  { } }
            fn is_repeating(&self) -> bool { loop  { } }
            fn time_remaining(&self)
            -> Option<W>;
            fn is_enabled(&self) -> bool { loop  { } }
            fn cancel(&self);
        }
        pub trait TimerClient {
            fn fired(&self);
        }
    }
    pub mod uart {
        use crate::returncode::ReturnCode;
        pub enum StopBits { One = 1, Two = 2, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for StopBits { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for StopBits {
            #[inline]
            fn clone(&self) -> StopBits { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for StopBits {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for StopBits {
            #[inline]
            fn eq(&self, other: &StopBits) -> bool { loop  { } }
        }
        pub enum Parity { None = 0, Odd = 1, Even = 2, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Parity { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Parity {
            #[inline]
            fn clone(&self) -> Parity { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Parity {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Parity {
            #[inline]
            fn eq(&self, other: &Parity) -> bool { loop  { } }
        }
        pub enum Width { Six = 6, Seven = 7, Eight = 8, }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Width { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Width {
            #[inline]
            fn clone(&self) -> Width { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Width {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Width {
            #[inline]
            fn eq(&self, other: &Width) -> bool { loop  { } }
        }
        pub struct Parameters {
            pub baud_rate: u32,
            pub width: Width,
            pub parity: Parity,
            pub stop_bits: StopBits,
            pub hw_flow_control: bool,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Parameters { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Parameters {
            #[inline]
            fn clone(&self) -> Parameters { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Parameters {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub enum Error {

            None,

            ParityError,

            FramingError,

            OverrunError,

            RepeatCallError,

            ResetError,

            Aborted,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { loop  { } }
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool { loop  { } }
        }
        pub trait Uart<'a>: Configure + Transmit<'a> + Receive<'a> { }
        pub trait UartData<'a>: Transmit<'a> + Receive<'a> { }
        pub trait UartAdvanced<'a>: Configure + Transmit<'a> +
         ReceiveAdvanced<'a> {
        }
        pub trait Client: ReceiveClient + TransmitClient { }
        pub trait Configure {
            fn configure(&self, params: Parameters)
            -> ReturnCode;
        }
        pub trait Transmit<'a> {
            fn set_transmit_client(&self, client: &'a dyn TransmitClient);
            fn transmit_buffer(&self, tx_buffer: &'static mut [u8],
                               tx_len: usize)
            -> (ReturnCode, Option<&'static mut [u8]>);
            fn transmit_word(&self, word: u32)
            -> ReturnCode;
            fn transmit_abort(&self)
            -> ReturnCode;
        }
        pub trait Receive<'a> {
            fn set_receive_client(&self, client: &'a dyn ReceiveClient);
            fn receive_buffer(&self, rx_buffer: &'static mut [u8],
                              rx_len: usize)
            -> (ReturnCode, Option<&'static mut [u8]>);
            fn receive_word(&self)
            -> ReturnCode;
            fn receive_abort(&self)
            -> ReturnCode;
        }
        pub trait TransmitClient {
            fn transmitted_word(&self, _rval: ReturnCode) { loop  { } }
            fn transmitted_buffer(&self, tx_buffer: &'static mut [u8],
                                  tx_len: usize, rval: ReturnCode);
        }
        pub trait ReceiveClient {
            fn received_word(&self, _word: u32, _rval: ReturnCode,
                             _error: Error) {
                loop  { }
            }
            fn received_buffer(&self, rx_buffer: &'static mut [u8],
                               rx_len: usize, rval: ReturnCode, error: Error);
        }
        pub trait ReceiveAdvanced<'a>: Receive<'a> {
            fn receive_automatic(&self, rx_buffer: &'static mut [u8],
                                 rx_len: usize, interbyte_timeout: u8)
            -> (ReturnCode, Option<&'static mut [u8]>);
        }
    }
    pub mod usb {
        use crate::common::cells::VolatileCell;
        pub trait UsbController {
            fn endpoint_set_buffer(&self, endpoint: usize,
                                   buf: &[VolatileCell<u8>]);
            fn enable_as_device(&self, speed: DeviceSpeed);
            fn attach(&self);
            fn detach(&self);
            fn set_address(&self, addr: u16);
            fn enable_address(&self);
            fn endpoint_ctrl_out_enable(&self, endpoint: usize);
            fn endpoint_bulk_in_enable(&self, endpoint: usize);
            fn endpoint_bulk_out_enable(&self, endpoint: usize);
            fn endpoint_bulk_resume(&self, endpoint: usize);
        }
        pub enum DeviceSpeed { Full, Low, }
        pub trait Client {
            fn enable(&self);
            fn attach(&self);
            fn bus_reset(&self);
            fn ctrl_setup(&self, endpoint: usize)
            -> CtrlSetupResult;
            fn ctrl_in(&self, endpoint: usize)
            -> CtrlInResult;
            fn ctrl_out(&self, endpoint: usize, packet_bytes: u32)
            -> CtrlOutResult;
            fn ctrl_status(&self, endpoint: usize);
            fn ctrl_status_complete(&self, endpoint: usize);
            fn bulk_in(&self, endpoint: usize)
            -> BulkInResult;
            fn bulk_out(&self, endpoint: usize, packet_bytes: u32)
            -> BulkOutResult;
        }
        pub enum CtrlSetupResult {

            Ok,
            OkSetAddress,
            ErrBadLength,
            ErrNoParse,
            ErrNonstandardRequest,
            ErrUnrecognizedDescriptorType,
            ErrUnrecognizedRequestType,
            ErrNoDeviceQualifier,
            ErrInvalidDeviceIndex,
            ErrInvalidConfigurationIndex,
            ErrInvalidInterfaceIndex,
            ErrInvalidStringIndex,
            ErrGeneric,
        }
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for CtrlSetupResult {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                loop  { }
            }
        }
        pub enum CtrlInResult {

            Packet(usize, bool),

            Delay,

            Error,
        }
        pub enum CtrlOutResult {

            Ok,

            Delay,

            Halted,
        }
        pub enum BulkInResult {

            Packet(usize),

            Delay,

            Error,
        }
        pub enum BulkOutResult {

            Ok,

            Delay,

            Error,
        }
    }
    pub mod watchdog {
        pub trait Watchdog {
            fn start(&self, period: usize);
            fn stop(&self);
            fn tickle(&self);
        }
    }
    pub trait Controller {
        type
        Config;
        fn configure(&self, _: Self::Config);
    }
}
pub mod introspection {
    use core::cell::Cell;
    use crate::callback::AppId;
    use crate::capabilities::ProcessManagementCapability;
    use crate::common::cells::NumericCellExt;
    use crate::process;
    use crate::sched::Kernel;
    pub struct KernelInfo {
        kernel: &'static Kernel,
    }
    impl KernelInfo {
        pub fn new(kernel: &'static Kernel) -> KernelInfo { loop  { } }
        pub fn number_loaded_processes(&self,
                                       _capability:
                                           &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn number_active_processes(&self,
                                       _capability:
                                           &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn number_inactive_processes(&self,
                                         _capability:
                                             &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn process_name(&self, app: AppId,
                            _capability: &dyn ProcessManagementCapability)
         -> &'static str {
            loop  { }
        }
        pub fn number_app_syscalls(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn number_app_dropped_callbacks(&self, app: AppId,
                                            _capability:
                                                &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn number_app_restarts(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn number_app_timeslice_expirations(&self, app: AppId,
                                                _capability:
                                                    &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
        pub fn timeslice_expirations(&self,
                                     _capability:
                                         &dyn ProcessManagementCapability)
         -> usize {
            loop  { }
        }
    }
}
pub mod ipc {
    use crate::callback::{AppId, Callback};
    use crate::capabilities::MemoryAllocationCapability;
    use crate::driver::Driver;
    use crate::grant::Grant;
    use crate::mem::{AppSlice, Shared};
    use crate::process;
    use crate::returncode::ReturnCode;
    use crate::sched::Kernel;
    pub const DRIVER_NUM: usize = 0x10000;
    struct IPCData {
        shared_memory: [Option<AppSlice<Shared, u8>>; 8],
        client_callbacks: [Option<Callback>; 8],
        callback: Option<Callback>,
    }
    impl Default for IPCData {
        fn default() -> IPCData { loop  { } }
    }
    pub struct IPC {
        data: Grant<IPCData>,
    }
    impl IPC {
        pub fn new(kernel: &'static Kernel,
                   capability: &dyn MemoryAllocationCapability) -> IPC {
            loop  { }
        }
        pub unsafe fn schedule_callback(&self, appid: AppId, otherapp: AppId,
                                        cb_type: process::IPCType) {
            loop  { }
        }
    }
    impl Driver for IPC {
        fn subscribe(&self, subscribe_num: usize, callback: Option<Callback>,
                     app_id: AppId) -> ReturnCode {
            loop  { }
        }
        fn command(&self, target_id: usize, client_or_svc: usize, _: usize,
                   appid: AppId) -> ReturnCode {
            loop  { }
        }
        fn allow(&self, appid: AppId, target_id: usize,
                 slice: Option<AppSlice<Shared, u8>>) -> ReturnCode {
            loop  { }
        }
    }
}
pub mod syscall {
    use core::fmt::Write;
    use crate::process;
    pub enum Syscall {

        YIELD,

        SUBSCRIBE {
            driver_number: usize,
            subdriver_number: usize,
            callback_ptr: *mut (),
            appdata: usize,
        },

        COMMAND {
            driver_number: usize,
            subdriver_number: usize,
            arg0: usize,
            arg1: usize,
        },

        ALLOW {
            driver_number: usize,
            subdriver_number: usize,
            allow_address: *mut u8,
            allow_size: usize,
        },

        MEMOP {
            operand: usize,
            arg0: usize,
        },
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Syscall { }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Syscall {
        #[inline]
        fn clone(&self) -> Syscall { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Syscall {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Syscall {
        #[inline]
        fn eq(&self, other: &Syscall) -> bool { loop  { } }
        #[inline]
        fn ne(&self, other: &Syscall) -> bool { loop  { } }
    }
    pub enum ContextSwitchReason {

        SyscallFired {
            syscall: Syscall,
        },

        Fault,

        TimesliceExpired,

        Interrupted,
    }
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ContextSwitchReason {
        #[inline]
        fn eq(&self, other: &ContextSwitchReason) -> bool { loop  { } }
        #[inline]
        fn ne(&self, other: &ContextSwitchReason) -> bool { loop  { } }
    }
    pub trait UserspaceKernelBoundary {
        type
        StoredState: Default +
        Copy;
        unsafe fn initialize_new_process(&self, stack_pointer: *const usize,
                                         stack_size: usize,
                                         state: &mut Self::StoredState)
        -> Result<*const usize, ()>;
        unsafe fn set_syscall_return_value(&self, stack_pointer: *const usize,
                                           state: &mut Self::StoredState,
                                           return_value: isize);
        unsafe fn set_process_function(&self, stack_pointer: *const usize,
                                       remaining_stack_memory: usize,
                                       state: &mut Self::StoredState,
                                       callback: process::FunctionCall)
        -> Result<*mut usize, *mut usize>;
        unsafe fn switch_to_process(&self, stack_pointer: *const usize,
                                    state: &mut Self::StoredState)
        -> (*mut usize, ContextSwitchReason);
        unsafe fn fault_fmt(&self, writer: &mut dyn Write);
        unsafe fn process_detail_fmt(&self, stack_pointer: *const usize,
                                     state: &Self::StoredState,
                                     writer: &mut dyn Write);
    }
    pub fn arguments_to_syscall(syscall_number: u8, r0: usize, r1: usize,
                                r2: usize, r3: usize) -> Option<Syscall> {
        loop  { }
    }
}
mod callback {
    use core::fmt;
    use core::ptr::NonNull;
    use crate::process;
    use crate::sched::Kernel;
    pub struct AppId {
        crate kernel: &'static Kernel,
        idx: usize,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for AppId {
        #[inline]
        fn clone(&self) -> AppId { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for AppId { }
    impl PartialEq for AppId {
        fn eq(&self, other: &AppId) -> bool { loop  { } }
    }
    impl Eq for AppId { }
    impl fmt::Debug for AppId {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { loop  { } }
    }
    impl AppId {
        crate fn new(kernel: &'static Kernel, idx: usize) -> AppId {
            loop  { }
        }
        pub fn idx(&self) -> usize { loop  { } }
        pub fn get_editable_flash_range(&self) -> (usize, usize) { loop  { } }
    }
    pub struct CallbackId {
        pub driver_num: usize,
        pub subscribe_num: usize,
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for CallbackId { }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for CallbackId {
        #[inline]
        fn clone(&self) -> CallbackId { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for CallbackId {
        #[inline]
        fn eq(&self, other: &CallbackId) -> bool { loop  { } }
        #[inline]
        fn ne(&self, other: &CallbackId) -> bool { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for CallbackId {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    pub struct Callback {
        app_id: AppId,
        callback_id: CallbackId,
        appdata: usize,
        fn_ptr: NonNull<*mut ()>,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Callback {
        #[inline]
        fn clone(&self) -> Callback { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Callback { }
    impl Callback {
        crate fn new(app_id: AppId, callback_id: CallbackId, appdata: usize,
                     fn_ptr: NonNull<*mut ()>) -> Callback {
            loop  { }
        }
        pub fn schedule(&mut self, r0: usize, r1: usize, r2: usize) -> bool {
            loop  { }
        }
    }
}
mod driver {
    use crate::callback::{AppId, Callback};
    use crate::mem::{AppSlice, Shared};
    use crate::returncode::ReturnCode;
    pub trait Driver {
        #[allow(unused_variables)]
        fn subscribe(&self, minor_num: usize, callback: Option<Callback>,
                     app_id: AppId) -> ReturnCode {
            loop  { }
        }
        #[allow(unused_variables)]
        fn command(&self, minor_num: usize, r2: usize, r3: usize,
                   caller_id: AppId) -> ReturnCode {
            loop  { }
        }
        #[allow(unused_variables)]
        fn allow(&self, app: AppId, minor_num: usize,
                 slice: Option<AppSlice<Shared, u8>>) -> ReturnCode {
            loop  { }
        }
    }
}
mod grant {
    use core::marker::PhantomData;
    use core::mem::{align_of, size_of};
    use core::ops::{Deref, DerefMut};
    use core::ptr::{write, write_volatile, Unique};
    use crate::callback::AppId;
    use crate::process::Error;
    use crate::sched::Kernel;
    pub struct Grant<T: Default> {
        crate kernel: &'static Kernel,
        grant_num: usize,
        ptr: PhantomData<T>,
    }
    pub struct AppliedGrant<T> {
        appid: AppId,
        grant: *mut T,
        _phantom: PhantomData<T>,
    }
    impl <T> AppliedGrant<T> {
        pub fn enter<F, R>(self, fun: F) -> R where
         F: FnOnce(&mut Owned<T>, &mut Allocator) -> R, R: Copy {
            loop  { }
        }
    }
    pub struct Allocator {
        appid: AppId,
    }
    pub struct Owned<T: ?Sized> {
        data: Unique<T>,
        appid: AppId,
    }
    impl <T: ?Sized> Owned<T> {
        unsafe fn new(data: *mut T, appid: AppId) -> Owned<T> { loop  { } }
        pub fn appid(&self) -> AppId { loop  { } }
    }
    impl <T: ?Sized> Drop for Owned<T> {
        fn drop(&mut self) { loop  { } }
    }
    impl <T: ?Sized> Deref for Owned<T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { loop  { } }
    }
    impl <T: ?Sized> DerefMut for Owned<T> {
        fn deref_mut(&mut self) -> &mut T { loop  { } }
    }
    impl Allocator {
        pub fn alloc<T>(&mut self, data: T) -> Result<Owned<T>, Error> {
            loop  { }
        }
    }
    pub struct Borrowed<'a, T: 'a + ?Sized> {
        data: &'a mut T,
        appid: AppId,
    }
    impl <T: 'a + ?Sized> Borrowed<'a, T> {
        pub fn new(data: &'a mut T, appid: AppId) -> Borrowed<'a, T> {
            loop  { }
        }
        pub fn appid(&self) -> AppId { loop  { } }
    }
    impl <T: 'a + ?Sized> Deref for Borrowed<'a, T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { loop  { } }
    }
    impl <T: 'a + ?Sized> DerefMut for Borrowed<'a, T> {
        fn deref_mut(&mut self) -> &mut T { loop  { } }
    }
    impl <T: Default> Grant<T> {
        crate fn new(kernel: &'static Kernel, grant_index: usize)
         -> Grant<T> {
            loop  { }
        }
        pub fn grant(&self, appid: AppId) -> Option<AppliedGrant<T>> {
            loop  { }
        }
        pub fn enter<F, R>(&self, appid: AppId, fun: F) -> Result<R, Error>
         where F: FnOnce(&mut Borrowed<T>, &mut Allocator) -> R, R: Copy {
            loop  { }
        }
        pub fn each<F>(&self, fun: F) where F: Fn(&mut Owned<T>) { loop  { } }
        pub fn iter(&self) -> Iter<T> { loop  { } }
    }
    pub struct Iter<'a, T: 'a + Default> {
        grant: &'a Grant<T>,
        index: usize,
        len: usize,
    }
    impl <T: Default> Iterator for Iter<'a, T> {
        type
        Item
        =
        AppliedGrant<T>;
        fn next(&mut self) -> Option<Self::Item> { loop  { } }
    }
}
mod mem {
    use core::marker::PhantomData;
    use core::ops::{Deref, DerefMut};
    use core::ptr::Unique;
    use core::slice;
    use crate::callback::AppId;
    pub struct Private;
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Private {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    pub struct Shared;
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Shared {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    pub struct AppPtr<L, T> {
        ptr: Unique<T>,
        process: AppId,
        _phantom: PhantomData<L>,
    }
    impl <L, T> AppPtr<L, T> {
        unsafe fn new(ptr: *mut T, appid: AppId) -> AppPtr<L, T> { loop  { } }
    }
    impl <L, T> Deref for AppPtr<L, T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { loop  { } }
    }
    impl <L, T> DerefMut for AppPtr<L, T> {
        fn deref_mut(&mut self) -> &mut T { loop  { } }
    }
    impl <L, T> Drop for AppPtr<L, T> {
        fn drop(&mut self) { loop  { } }
    }
    pub struct AppSlice<L, T> {
        ptr: AppPtr<L, T>,
        len: usize,
    }
    impl <L, T> AppSlice<L, T> {
        crate fn new(ptr: *mut T, len: usize, appid: AppId)
         -> AppSlice<L, T> {
            loop  { }
        }
        pub fn len(&self) -> usize { loop  { } }
        pub fn ptr(&self) -> *const T { loop  { } }
        crate unsafe fn expose_to(&self, appid: AppId) -> bool { loop  { } }
        pub fn iter(&self) -> slice::Iter<T> { loop  { } }
        pub fn iter_mut(&mut self) -> slice::IterMut<T> { loop  { } }
        pub fn chunks(&self, size: usize) -> slice::Chunks<T> { loop  { } }
        pub fn chunks_mut(&mut self, size: usize) -> slice::ChunksMut<T> {
            loop  { }
        }
    }
    impl <L, T> AsRef<[T]> for AppSlice<L, T> {
        fn as_ref(&self) -> &[T] { loop  { } }
    }
    impl <L, T> AsMut<[T]> for AppSlice<L, T> {
        fn as_mut(&mut self) -> &mut [T] { loop  { } }
    }
}
mod memop {
    use crate::process::ProcessType;
    use crate::returncode::ReturnCode;
    crate fn memop(process: &dyn ProcessType, op_type: usize, r1: usize)
     -> ReturnCode {
        loop  { }
    }
}
mod platform {
    use crate::driver::Driver;
    use crate::syscall;
    pub mod mpu {
        use core::cmp;
        use core::fmt::Display;
        pub enum Permissions {
            ReadWriteExecute,
            ReadWriteOnly,
            ReadExecuteOnly,
            ReadOnly,
            ExecuteOnly,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Permissions { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Permissions {
            #[inline]
            fn clone(&self) -> Permissions { loop  { } }
        }
        pub struct Region {
            start_address: *const u8,
            size: usize,
        }
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Region { }
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Region {
            #[inline]
            fn clone(&self) -> Region { loop  { } }
        }
        impl Region {
            pub fn new(start_address: *const u8, size: usize) -> Region {
                loop  { }
            }
            pub fn start_address(&self) -> *const u8 { loop  { } }
            pub fn size(&self) -> usize { loop  { } }
        }
        pub struct MpuConfig;
        #[allow(unused_qualifications)]
        impl ::core::default::Default for MpuConfig {
            #[inline]
            fn default() -> MpuConfig { loop  { } }
        }
        impl Display for MpuConfig {
            fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
                loop  { }
            }
        }
        pub trait MPU {
            type
            MpuConfig: Default +
            Display
            =
            ();
            fn enable_mpu(&self) { loop  { } }
            fn disable_mpu(&self) { loop  { } }
            fn number_total_regions(&self) -> usize { loop  { } }
            #[allow(unused_variables)]
            fn allocate_region(&self, unallocated_memory_start: *const u8,
                               unallocated_memory_size: usize,
                               min_region_size: usize,
                               permissions: Permissions,
                               config: &mut Self::MpuConfig)
             -> Option<Region> {
                loop  { }
            }
            #[allow(unused_variables)]
            fn allocate_app_memory_region(&self,
                                          unallocated_memory_start: *const u8,
                                          unallocated_memory_size: usize,
                                          min_memory_size: usize,
                                          initial_app_memory_size: usize,
                                          initial_kernel_memory_size: usize,
                                          permissions: Permissions,
                                          config: &mut Self::MpuConfig)
             -> Option<(*const u8, usize)> {
                loop  { }
            }
            #[allow(unused_variables)]
            fn update_app_memory_region(&self, app_memory_break: *const u8,
                                        kernel_memory_break: *const u8,
                                        permissions: Permissions,
                                        config: &mut Self::MpuConfig)
             -> Result<(), ()> {
                loop  { }
            }
            #[allow(unused_variables)]
            fn configure_mpu(&self, config: &Self::MpuConfig) { loop  { } }
        }
        impl MPU for () { }
    }
    crate mod systick {
        pub trait SysTick {
            fn set_timer(&self, us: u32);
            fn greater_than(&self, us: u32)
            -> bool;
            fn overflowed(&self)
            -> bool;
            fn reset(&self);
            fn enable(&self, with_interrupt: bool);
        }
        impl SysTick for () {
            fn reset(&self) { loop  { } }
            fn set_timer(&self, _: u32) { loop  { } }
            fn enable(&self, _: bool) { loop  { } }
            fn overflowed(&self) -> bool { loop  { } }
            fn greater_than(&self, _: u32) -> bool { loop  { } }
        }
    }
    pub trait Platform {
        fn with_driver<F, R>(&self, driver_num: usize, f: F)
        -> R
        where
        F: FnOnce(Option<&dyn Driver>)
        ->
        R;
    }
    pub trait Chip {
        type
        MPU: mpu::MPU;
        type
        UserspaceKernelBoundary: syscall::UserspaceKernelBoundary;
        type
        SysTick: systick::SysTick;
        fn service_pending_interrupts(&self) { loop  { } }
        fn has_pending_interrupts(&self) -> bool { loop  { } }
        fn mpu(&self) -> &Self::MPU { loop  { } }
        fn systick(&self) -> &Self::SysTick { loop  { } }
        fn userspace_kernel_boundary(&self)
         -> &Self::UserspaceKernelBoundary {
            loop  { }
        }
        fn sleep(&self) { loop  { } }
        unsafe fn atomic<F, R>(&self, _: F) -> R where F: FnOnce() -> R {
            loop  { }
        }
    }
    pub trait ClockInterface {
        fn is_enabled(&self) -> bool { loop  { } }
        fn enable(&self) { loop  { } }
        fn disable(&self) { loop  { } }
    }
    pub struct NoClockControl {
    }
    impl ClockInterface for NoClockControl {
        fn is_enabled(&self) -> bool { loop  { } }
        fn enable(&self) { loop  { } }
        fn disable(&self) { loop  { } }
    }
    pub static mut NO_CLOCK_CONTROL: NoClockControl = NoClockControl{};
}
mod process {
use core::cell::Cell;
use core::fmt::Write;
use core::ptr::write_volatile;
use core::{mem, ptr, slice, str};

use crate::callback::{AppId, CallbackId};
use crate::capabilities::ProcessManagementCapability;
use crate::common::cells::MapCell;
use crate::common::{Queue, RingBuffer};
use crate::mem::{AppSlice, Shared};
use crate::platform::mpu::{self, MPU};
use crate::platform::Chip;
use crate::returncode::ReturnCode;
use crate::sched::Kernel;
use crate::syscall::{self, Syscall, UserspaceKernelBoundary};
use crate::tbfheader;
use core::cmp::max;

pub fn load_processes<C: Chip>(
    kernel: &'static Kernel,
    chip: &'static C,
    start_of_flash: *const u8,
    app_memory: &mut [u8],
    procs: &'static mut [Option<&'static dyn ProcessType>],
    fault_response: FaultResponse,
    _capability: &dyn ProcessManagementCapability,
) {
    let mut apps_in_flash_ptr = start_of_flash;
    let mut app_memory_ptr = app_memory.as_mut_ptr();
    let mut app_memory_size = app_memory.len();
    for i in 0..procs.len() {
        unsafe {
            let (process, flash_offset, memory_offset) = Process::create(
                kernel,
                chip,
                apps_in_flash_ptr,
                app_memory_ptr,
                app_memory_size,
                fault_response,
                i,
            );

            if process.is_none() {
                if flash_offset == 0 && memory_offset == 0 {
                    break;
                }
            } else {
                procs[i] = process;
            }

            apps_in_flash_ptr = apps_in_flash_ptr.add(flash_offset);
            app_memory_ptr = app_memory_ptr.add(memory_offset);
            app_memory_size -= memory_offset;
        }
    }
}


pub trait ProcessType {
    fn appid(&self) -> AppId { loop { } }

    fn enqueue_task(&self, task: Task) -> bool { loop { } }

    fn dequeue_task(&self) -> Option<Task> { loop { } }

    fn remove_pending_callbacks(&self, callback_id: CallbackId) { loop { } }

    fn get_state(&self) -> State { loop { } }

    fn set_yielded_state(&self) { loop { } }

    fn stop(&self) { loop { } }

    fn resume(&self) { loop { } }

    fn set_fault_state(&self) { loop { } }

    fn get_process_name(&self) -> &'static str { loop { } }

    fn brk(&self, new_break: *const u8) -> Result<*const u8, Error> { loop { } }

    fn sbrk(&self, increment: isize) -> Result<*const u8, Error> { loop { } }

    fn mem_start(&self) -> *const u8 { loop { } }

    fn mem_end(&self) -> *const u8 { loop { } }

    fn flash_start(&self) -> *const u8 { loop { } }

    fn flash_end(&self) -> *const u8 { loop { } }

    fn kernel_memory_break(&self) -> *const u8 { loop { } }

    fn number_writeable_flash_regions(&self) -> usize { loop { } }

    fn get_writeable_flash_region(&self, region_index: usize) -> (u32, u32) { loop { } }

    fn update_stack_start_pointer(&self, stack_pointer: *const u8) { loop { } }

    fn update_heap_start_pointer(&self, heap_pointer: *const u8) { loop { } }

    fn allow(
        &self,
        buf_start_addr: *const u8,
        size: usize,
    ) -> Result<Option<AppSlice<Shared, u8>>, ReturnCode> { loop { } }

    fn flash_non_protected_start(&self) -> *const u8 { loop { } }

    fn setup_mpu(&self) { loop { } }

    fn add_mpu_region(
        &self,
        unallocated_memory_start: *const u8,
        unallocated_memory_size: usize,
        min_region_size: usize,
    ) -> Option<mpu::Region> { loop { } }

    unsafe fn alloc(&self, size: usize, align: usize) -> Option<&mut [u8]> { loop { } }

    unsafe fn free(&self, _: *mut u8) { loop { } }

    unsafe fn grant_ptr(&self, grant_num: usize) -> *mut *mut u8 { loop { } }

    unsafe fn set_syscall_return_value(&self, return_value: isize) { loop { } }

    unsafe fn set_process_function(&self, callback: FunctionCall) { loop { } }

    unsafe fn switch_to(&self) -> Option<syscall::ContextSwitchReason> { loop { } }

    unsafe fn fault_fmt(&self, writer: &mut dyn Write) { loop { } }
    unsafe fn process_detail_fmt(&self, writer: &mut dyn Write) { loop { } }

    fn debug_syscall_count(&self) -> usize { loop { } }

    fn debug_dropped_callback_count(&self) -> usize { loop { } }

    fn debug_restart_count(&self) -> usize { loop { } }

    fn debug_timeslice_expiration_count(&self) -> usize { loop { } }

    fn debug_timeslice_expired(&self) { loop { } }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NoSuchApp,
    OutOfMemory,
    AddressOutOfBounds,
    KernelError, // This likely indicates a bug in the kernel and that some
}

impl From<Error> for ReturnCode {
    fn from(err: Error) -> ReturnCode { loop { } }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    Running,

    Yielded,

    StoppedRunning,

    StoppedYielded,

    StoppedFaulted,

    Fault,

    Unstarted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FaultResponse {
    Panic,

    Restart,

    Stop,
}

#[derive(Copy, Clone, Debug)]
pub enum IPCType {
    Service,
    Client,
}

#[derive(Copy, Clone)]
pub enum Task {
    FunctionCall(FunctionCall),
    IPC((AppId, IPCType)),
}

#[derive(Copy, Clone, Debug)]
pub enum FunctionCallSource {
    Kernel, // For functions coming directly from the kernel, such as `init_fn`.
    Driver(CallbackId),
}

#[derive(Copy, Clone, Debug)]
pub struct FunctionCall {
    pub source: FunctionCallSource,
    pub argument0: usize,
    pub argument1: usize,
    pub argument2: usize,
    pub argument3: usize,
    pub pc: usize,
}

struct ProcessDebug {
    app_heap_start_pointer: Option<*const u8>,

    app_stack_start_pointer: Option<*const u8>,

    min_stack_pointer: *const u8,

    syscall_count: usize,

    last_syscall: Option<Syscall>,

    dropped_callback_count: usize,

    restart_count: usize,

    timeslice_expiration_count: usize,
}

pub struct Process<'a, C: 'static + Chip> {
    app_idx: usize,

    kernel: &'static Kernel,

    chip: &'static C,

    memory: &'static mut [u8],

    kernel_memory_break: Cell<*const u8>,

    original_kernel_memory_break: *const u8,

    app_break: Cell<*const u8>,
    original_app_break: *const u8,

    allow_high_water_mark: Cell<*const u8>,

    current_stack_pointer: Cell<*const u8>,
    original_stack_pointer: *const u8,

    flash: &'static [u8],

    header: tbfheader::TbfHeader,

    state: Cell<State>,

    fault_response: FaultResponse,

    mpu_config: MapCell<<<C as Chip>::MPU as MPU>::MpuConfig>,

    mpu_regions: [Cell<Option<mpu::Region>>; 6],

    tasks: MapCell<RingBuffer<'a, Task>>,

    process_name: &'static str,

    debug: MapCell<ProcessDebug>,
}

impl<C: Chip> ProcessType for Process<'a, C> {
    fn appid(&self) -> AppId { loop { } }

    fn enqueue_task(&self, task: Task) -> bool { loop { } }

    fn remove_pending_callbacks(&self, callback_id: CallbackId) { loop { } }

    fn get_state(&self) -> State { loop { } }

    fn set_yielded_state(&self) { loop { } }

    fn stop(&self) { loop { } }

    fn resume(&self) { loop { } }

    fn set_fault_state(&self) { loop { } }

    fn dequeue_task(&self) -> Option<Task> { loop { } }

    fn mem_start(&self) -> *const u8 { loop { } }

    fn mem_end(&self) -> *const u8 { loop { } }

    fn flash_start(&self) -> *const u8 { loop { } }

    fn flash_non_protected_start(&self) -> *const u8 { loop { } }

    fn flash_end(&self) -> *const u8 { loop { } }

    fn kernel_memory_break(&self) -> *const u8 { loop { } }

    fn number_writeable_flash_regions(&self) -> usize { loop { } }

    fn get_writeable_flash_region(&self, region_index: usize) -> (u32, u32) { loop { } }

    fn update_stack_start_pointer(&self, stack_pointer: *const u8) { loop { } }

    fn update_heap_start_pointer(&self, heap_pointer: *const u8) { loop { } }

    fn setup_mpu(&self) { loop { } }

    fn add_mpu_region(
        &self,
        unallocated_memory_start: *const u8,
        unallocated_memory_size: usize,
        min_region_size: usize,
    ) -> Option<mpu::Region> { loop { } }

    fn sbrk(&self, increment: isize) -> Result<*const u8, Error> { loop { } }

    fn brk(&self, new_break: *const u8) -> Result<*const u8, Error> { loop { } }

    fn allow(
        &self,
        buf_start_addr: *const u8,
        size: usize,
    ) -> Result<Option<AppSlice<Shared, u8>>, ReturnCode> { loop { } }

    unsafe fn alloc(&self, size: usize, align: usize) -> Option<&mut [u8]> { loop { } }

    unsafe fn free(&self, _: *mut u8) { loop { } }

    #[allow(clippy::cast_ptr_alignment)]
    unsafe fn grant_ptr(&self, grant_num: usize) -> *mut *mut u8 { loop { } }

    fn get_process_name(&self) -> &'static str { loop { } }

    unsafe fn set_syscall_return_value(&self, return_value: isize) { loop { } }

    unsafe fn set_process_function(&self, callback: FunctionCall) { loop { } }

    unsafe fn switch_to(&self) -> Option<syscall::ContextSwitchReason> { loop { } }

    fn debug_syscall_count(&self) -> usize { loop { } }

    fn debug_dropped_callback_count(&self) -> usize { loop { } }

    fn debug_restart_count(&self) -> usize { loop { } }

    fn debug_timeslice_expiration_count(&self) -> usize { loop { } }

    fn debug_timeslice_expired(&self) { loop { } }

    unsafe fn fault_fmt(&self, writer: &mut dyn Write) { loop { } }

    unsafe fn process_detail_fmt(&self, writer: &mut dyn Write) {
        let flash_end = self.flash.as_ptr().add(self.flash.len()) as usize;
        let flash_start = self.flash.as_ptr() as usize;
        let flash_protected_size = self.header.get_protected_size() as usize;
        let flash_app_start = flash_start + flash_protected_size;
        let flash_app_size = flash_end - flash_app_start;
        let flash_init_fn = flash_start + self.header.get_init_function_offset() as usize;

        let sram_end = self.memory.as_ptr().add(self.memory.len()) as usize;
        let sram_grant_start = self.kernel_memory_break.get() as usize;
        let sram_heap_end = self.app_break.get() as usize;
        let sram_heap_start: Option<usize> = self.debug.map_or(None, |debug| {
            debug.app_heap_start_pointer.map(|p| p as usize)
        });
        let sram_stack_start: Option<usize> = self.debug.map_or(None, |debug| {
            debug.app_stack_start_pointer.map(|p| p as usize)
        });
        let sram_stack_bottom =
            self.debug
                .map_or(ptr::null(), |debug| debug.min_stack_pointer) as usize;
        let sram_start = self.memory.as_ptr() as usize;

        let sram_grant_size = sram_end - sram_grant_start;
        let sram_grant_allocated = sram_end - sram_grant_start;

        let events_queued = self.tasks.map_or(0, |tasks| tasks.len());
        let syscall_count = self.debug.map_or(0, |debug| debug.syscall_count);
        let last_syscall = self.debug.map(|debug| debug.last_syscall);
        let dropped_callback_count = self.debug.map_or(0, |debug| debug.dropped_callback_count);
        let restart_count = self.debug.map_or(0, |debug| debug.restart_count);

        let _ = writer.write_fmt(format_args!(
            "\
             App: {}   -   [{:?}]\
             \r\n Events Queued: {}   Syscall Count: {}   Dropped Callback Count: {}\
             \n Restart Count: {}\n",
            self.process_name,
            self.state.get(),
            events_queued,
            syscall_count,
            dropped_callback_count,
            restart_count,
        ));

        let _ = match last_syscall {
            Some(syscall) => writer.write_fmt(format_args!(" Last Syscall: {:?}", syscall)),
            None => writer.write_str(" Last Syscall: None"),
        };

        let _ = writer.write_fmt(format_args!(
            "\
             \r\n\
             \r\n ╔═══════════╤══════════════════════════════════════════╗\
             \r\n ║  Address  │ Region Name    Used | Allocated (bytes)  ║\
             \r\n ╚{:#010X}═╪══════════════════════════════════════════╝\
             \r\n             │ ▼ Grant      {:6} | {:6}{}\
             \r\n  {:#010X} ┼───────────────────────────────────────────\
             \r\n             │ Unused\
             \r\n  {:#010X} ┼───────────────────────────────────────────",
            sram_end,
            sram_grant_size,
            sram_grant_allocated,
            exceeded_check(sram_grant_size, sram_grant_allocated),
            sram_grant_start,
            sram_heap_end,
        ));

        match sram_heap_start {
            Some(sram_heap_start) => {
                let sram_heap_size = sram_heap_end - sram_heap_start;
                let sram_heap_allocated = sram_grant_start - sram_heap_start;

                let _ = writer.write_fmt(format_args!(
                    "\
                     \r\n             │ ▲ Heap       {:6} | {:6}{}     S\
                     \r\n  {:#010X} ┼─────────────────────────────────────────── R",
                    sram_heap_size,
                    sram_heap_allocated,
                    exceeded_check(sram_heap_size, sram_heap_allocated),
                    sram_heap_start,
                ));
            }
            None => {
                let _ = writer.write_str(
                    "\
                     \r\n             │ ▲ Heap            ? |      ?               S\
                     \r\n  ?????????? ┼─────────────────────────────────────────── R",
                );
            }
        }

        match (sram_heap_start, sram_stack_start) {
            (Some(sram_heap_start), Some(sram_stack_start)) => {
                let sram_data_size = sram_heap_start - sram_stack_start;
                let sram_data_allocated = sram_data_size as usize;

                let _ = writer.write_fmt(format_args!(
                    "\
                     \r\n             │ Data         {:6} | {:6}               A",
                    sram_data_size, sram_data_allocated,
                ));
            }
            _ => {
                let _ = writer.write_str(
                    "\
                     \r\n             │ Data              ? |      ?               A",
                );
            }
        }

        match sram_stack_start {
            Some(sram_stack_start) => {
                let sram_stack_size = sram_stack_start - sram_stack_bottom;
                let sram_stack_allocated = sram_stack_start - sram_start;

                let _ = writer.write_fmt(format_args!(
                    "\
                     \r\n  {:#010X} ┼─────────────────────────────────────────── M\
                     \r\n             │ ▼ Stack      {:6} | {:6}{}",
                    sram_stack_start,
                    sram_stack_size,
                    sram_stack_allocated,
                    exceeded_check(sram_stack_size, sram_stack_allocated),
                ));
            }
            None => {
                let _ = writer.write_str(
                    "\
                     \r\n  ?????????? ┼─────────────────────────────────────────── M\
                     \r\n             │ ▼ Stack           ? |      ?",
                );
            }
        }

        let _ = writer.write_fmt(format_args!(
            "\
             \r\n  {:#010X} ┼───────────────────────────────────────────\
             \r\n             │ Unused\
             \r\n  {:#010X} ┴───────────────────────────────────────────\
             \r\n             .....\
             \r\n  {:#010X} ┬─────────────────────────────────────────── F\
             \r\n             │ App Flash    {:6}                        L\
             \r\n  {:#010X} ┼─────────────────────────────────────────── A\
             \r\n             │ Protected    {:6}                        S\
             \r\n  {:#010X} ┴─────────────────────────────────────────── H\
             \r\n",
            sram_stack_bottom,
            sram_start,
            flash_end,
            flash_app_size,
            flash_app_start,
            flash_protected_size,
            flash_start
        ));

        self.mpu_config.map(|config| {
            let _ = writer.write_fmt(format_args!("{}", config));
        });

        let _ = writer.write_fmt(format_args!(
            "\
             \r\nTo debug, run `make debug RAM_START={:#x} FLASH_INIT={:#x}`\
             \r\nin the app's folder and open the .lst file.\r\n\r\n",
            sram_start, flash_init_fn
        ));
    }
}

fn exceeded_check(size: usize, allocated: usize) -> &'static str { loop { } }

impl<C: 'static + Chip> Process<'a, C> {
    #[allow(clippy::cast_ptr_alignment)]
    crate unsafe fn create(
        kernel: &'static Kernel,
        chip: &'static C,
        app_flash_address: *const u8,
        remaining_app_memory: *mut u8,
        remaining_app_memory_size: usize,
        fault_response: FaultResponse,
        index: usize,
    ) -> (Option<&'static dyn ProcessType>, usize, usize) {
        if let Some(tbf_header) = tbfheader::parse_and_validate_tbf_header(app_flash_address) {
            let app_flash_size = tbf_header.get_total_size() as usize;

            if !tbf_header.is_app() || !tbf_header.enabled() {
                return (None, app_flash_size, 0);
            }

            let mut min_app_ram_size = tbf_header.get_minimum_app_ram_size() as usize;
            let process_name = tbf_header.get_package_name();
            let init_fn =
                app_flash_address.offset(tbf_header.get_init_function_offset() as isize) as usize;

            let mut mpu_config: <<C as Chip>::MPU as MPU>::MpuConfig = Default::default();

            if let None = chip.mpu().allocate_region(
                app_flash_address,
                app_flash_size,
                app_flash_size,
                mpu::Permissions::ReadExecuteOnly,
                &mut mpu_config,
            ) {
                return (None, app_flash_size, 0);
            }

            let grant_ptr_size = mem::size_of::<*const usize>();
            let grant_ptrs_num = kernel.get_grant_count_and_finalize();
            let grant_ptrs_offset = grant_ptrs_num * grant_ptr_size;

            let callback_size = mem::size_of::<Task>();
            let callback_len = 10;
            let callbacks_offset = callback_len * callback_size;

            let process_struct_offset = mem::size_of::<Process<C>>();

            let initial_kernel_memory_size =
                grant_ptrs_offset + callbacks_offset + process_struct_offset;
            let initial_app_memory_size = 3 * 1024;

            if min_app_ram_size < initial_app_memory_size {
                min_app_ram_size = initial_app_memory_size;
            }

            let min_total_memory_size = min_app_ram_size + initial_kernel_memory_size;

            let (memory_start, memory_size) = match chip.mpu().allocate_app_memory_region(
                remaining_app_memory as *const u8,
                remaining_app_memory_size,
                min_total_memory_size,
                initial_app_memory_size,
                initial_kernel_memory_size,
                mpu::Permissions::ReadWriteOnly,
                &mut mpu_config,
            ) {
                Some((memory_start, memory_size)) => (memory_start, memory_size),
                None => {
                    return (None, app_flash_size, 0);
                }
            };

            let memory_padding_size = (memory_start as usize) - (remaining_app_memory as usize);

            let app_memory = slice::from_raw_parts_mut(memory_start as *mut u8, memory_size);

            let initial_stack_pointer = memory_start.add(initial_app_memory_size);
            let initial_sbrk_pointer = memory_start.add(initial_app_memory_size);

            let mut kernel_memory_break = app_memory.as_mut_ptr().add(app_memory.len());

            kernel_memory_break = kernel_memory_break.offset(-(grant_ptrs_offset as isize));

            let opts =
                slice::from_raw_parts_mut(kernel_memory_break as *mut *const usize, grant_ptrs_num);
            for opt in opts.iter_mut() {
                *opt = ptr::null()
            }

            kernel_memory_break = kernel_memory_break.offset(-(callbacks_offset as isize));

            let callback_buf =
                slice::from_raw_parts_mut(kernel_memory_break as *mut Task, callback_len);
            let tasks = RingBuffer::new(callback_buf);

            kernel_memory_break = kernel_memory_break.offset(-(process_struct_offset as isize));
            let process_struct_memory_location = kernel_memory_break;

            let app_heap_start_pointer = None;
            let app_stack_start_pointer = None;

            let mut process: &mut Process<C> =
                &mut *(process_struct_memory_location as *mut Process<'static, C>);

            process.app_idx = index;
            process.kernel = kernel;
            process.chip = chip;
            process.memory = app_memory;
            process.header = tbf_header;
            process.kernel_memory_break = Cell::new(kernel_memory_break);
            process.original_kernel_memory_break = kernel_memory_break;
            process.app_break = Cell::new(initial_sbrk_pointer);
            process.original_app_break = initial_sbrk_pointer;
            process.allow_high_water_mark = Cell::new(remaining_app_memory);
            process.current_stack_pointer = Cell::new(initial_stack_pointer);
            process.original_stack_pointer = initial_stack_pointer;

            process.flash = slice::from_raw_parts(app_flash_address, app_flash_size);

            // process.stored_state = Cell::new(Default::default());
            process.state = Cell::new(State::Unstarted);
            process.fault_response = fault_response;

            process.mpu_config = MapCell::new(mpu_config);
            process.mpu_regions = [
                Cell::new(None),
                Cell::new(None),
                Cell::new(None),
                Cell::new(None),
                Cell::new(None),
                Cell::new(None),
            ];
            process.tasks = MapCell::new(tasks);
            process.process_name = process_name;

            process.debug = MapCell::new(ProcessDebug {
                app_heap_start_pointer: app_heap_start_pointer,
                app_stack_start_pointer: app_stack_start_pointer,
                min_stack_pointer: initial_stack_pointer,
                syscall_count: 0,
                last_syscall: None,
                dropped_callback_count: 0,
                restart_count: 0,
                timeslice_expiration_count: 0,
            });

            let flash_protected_size = process.header.get_protected_size() as usize;
            let flash_app_start = app_flash_address as usize + flash_protected_size;

            process.tasks.map(|tasks| {
                tasks.enqueue(Task::FunctionCall(FunctionCall {
                    source: FunctionCallSource::Kernel,
                    pc: init_fn,
                    argument0: flash_app_start,
                    argument1: process.memory.as_ptr() as usize,
                    argument2: process.memory.len() as usize,
                    argument3: process.app_break.get() as usize,
                }));
            });

            /*
            let mut stored_state = process.stored_state.get();
            match chip.userspace_kernel_boundary().initialize_new_process(
                process.sp(),
                process.sp() as usize - process.memory.as_ptr() as usize,
                &mut stored_state,
            ) {
                Ok(new_stack_pointer) => {
                    process
                        .current_stack_pointer
                        .set(new_stack_pointer as *mut u8);
                    process.debug_set_max_stack_depth();
                    process.stored_state.set(stored_state);
                }
                Err(_) => {
                    return (None, app_flash_size, 0);
                }
            };
             */

            kernel.increment_work();

            return (
                Some(process),
                app_flash_size,
                memory_padding_size + memory_size,
            );
        }
        (None, 0, 0)
    }

    #[allow(clippy::cast_ptr_alignment)]
    fn sp(&self) -> *const usize { loop { } }

    fn in_app_owned_memory(&self, buf_start_addr: *const u8, size: usize) -> bool { loop { } }

    #[allow(clippy::cast_ptr_alignment)]
    unsafe fn grant_ptrs_reset(&self) { loop { } }

    fn debug_set_max_stack_depth(&self) { loop { } }
}
}
mod returncode {
    pub enum ReturnCode {

        SuccessWithValue {
            value: usize,
        },

        SUCCESS,

        FAIL,

        EBUSY,

        EALREADY,

        EOFF,

        ERESERVE,

        EINVAL,

        ESIZE,

        ECANCEL,

        ENOMEM,

        ENOSUPPORT,

        ENODEVICE,

        EUNINSTALLED,

        ENOACK,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ReturnCode {
        #[inline]
        fn clone(&self) -> ReturnCode { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ReturnCode { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ReturnCode {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ReturnCode {
        #[inline]
        fn eq(&self, other: &ReturnCode) -> bool { loop  { } }
        #[inline]
        fn ne(&self, other: &ReturnCode) -> bool { loop  { } }
    }
    impl From<ReturnCode> for isize {
        fn from(original: ReturnCode) -> isize { loop  { } }
    }
    impl From<ReturnCode> for usize {
        fn from(original: ReturnCode) -> usize { loop  { } }
    }
}
mod sched {
    use core::cell::Cell;
    use core::ptr::NonNull;
    use crate::callback::{Callback, CallbackId};
    use crate::capabilities;
    use crate::common::cells::NumericCellExt;
    use crate::common::dynamic_deferred_call::DynamicDeferredCall;
    use crate::grant::Grant;
    use crate::ipc;
    use crate::memop;
    use crate::platform::mpu::MPU;
    use crate::platform::systick::SysTick;
    use crate::platform::{Chip, Platform};
    use crate::process::{self, Task};
    use crate::returncode::ReturnCode;
    use crate::syscall::{ContextSwitchReason, Syscall};
    const KERNEL_TICK_DURATION_US: u32 = 10000;
    const MIN_QUANTA_THRESHOLD_US: u32 = 500;
    pub struct Kernel {
        work: Cell<usize>,
        processes: &'static [Option<&'static dyn process::ProcessType>],
        grant_counter: Cell<usize>,
        grants_finalized: Cell<bool>,
    }
    impl Kernel {
        pub fn new(processes:
                       &'static [Option<&'static dyn process::ProcessType>])
         -> Kernel {
            loop  { }
        }
        crate fn increment_work(&self) { loop  { } }
        crate fn decrement_work(&self) { loop  { } }
        fn processes_blocked(&self) -> bool { loop  { } }
        crate fn process_map_or<F,
                                R>(&self, default: R, process_index: usize,
                                   closure: F) -> R where
         F: FnOnce(&dyn process::ProcessType) -> R {
            loop  { }
        }
        crate fn process_each<F>(&self, closure: F) where
         F: Fn(&dyn process::ProcessType) {
            loop  { }
        }
        pub fn process_each_capability<F>(&'static self,
                                          _capability:
                                              &dyn capabilities::ProcessManagementCapability,
                                          closure: F) where
         F: Fn(usize, &dyn process::ProcessType) {
            loop  { }
        }
        crate fn process_until<F>(&self, closure: F) -> ReturnCode where
         F: Fn(&dyn process::ProcessType) -> ReturnCode {
            loop  { }
        }
        crate fn number_of_process_slots(&self) -> usize { loop  { } }
        pub fn create_grant<T: Default>(&'static self,
                                        _capability:
                                            &dyn capabilities::MemoryAllocationCapability)
         -> Grant<T> {
            loop  { }
        }
        crate fn get_grant_count_and_finalize(&self) -> usize { loop  { } }
        pub fn hardfault_all_apps<C: capabilities::ProcessManagementCapability>(&self,
                                                                                _c:
                                                                                    &C) {
            loop  { }
        }
        pub fn kernel_loop<P: Platform,
                           C: Chip>(&'static self, platform: &P, chip: &C,
                                    ipc: Option<&ipc::IPC>,
                                    _capability:
                                        &dyn capabilities::MainLoopCapability) {
            loop  { }
        }
        unsafe fn do_process<P: Platform,
                             C: Chip>(&self, platform: &P, chip: &C,
                                      process: &dyn process::ProcessType,
                                      ipc: Option<&crate::ipc::IPC>) {
            loop  { }
        }
    }
}
mod tbfheader {
    use core::{mem, slice, str};
    #[repr(C)]
    crate struct TbfHeaderV2Base {
        version: u16,
        header_size: u16,
        total_size: u32,
        flags: u32,
        checksum: u32,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TbfHeaderV2Base {
        #[inline]
        fn clone(&self) -> TbfHeaderV2Base { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Base { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2Base {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[repr(u16)]
    #[allow(dead_code)]
    crate enum TbfHeaderTypes {
        TbfHeaderMain = 1,
        TbfHeaderWriteableFlashRegions = 2,
        TbfHeaderPackageName = 3,
        Unused = 5,
    }
    #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::clone::Clone for TbfHeaderTypes {
        #[inline]
        fn clone(&self) -> TbfHeaderTypes { loop  { } }
    }
    #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::marker::Copy for TbfHeaderTypes { }
    #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for TbfHeaderTypes {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[repr(C)]
    crate struct TbfHeaderTlv {
        tipe: TbfHeaderTypes,
        length: u16,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TbfHeaderTlv {
        #[inline]
        fn clone(&self) -> TbfHeaderTlv { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderTlv { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderTlv {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[repr(C)]
    crate struct TbfHeaderV2Main {
        init_fn_offset: u32,
        protected_size: u32,
        minimum_ram_size: u32,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TbfHeaderV2Main {
        #[inline]
        fn clone(&self) -> TbfHeaderV2Main { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Main { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2Main {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    #[repr(C)]
    crate struct TbfHeaderV2WriteableFlashRegion {
        writeable_flash_region_offset: u32,
        writeable_flash_region_size: u32,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TbfHeaderV2WriteableFlashRegion {
        #[inline]
        fn clone(&self) -> TbfHeaderV2WriteableFlashRegion { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2WriteableFlashRegion { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2WriteableFlashRegion {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    crate struct TbfHeaderV2 {
        base: &'static TbfHeaderV2Base,
        main: Option<&'static TbfHeaderV2Main>,
        package_name: Option<&'static str>,
        writeable_regions: Option<&'static [TbfHeaderV2WriteableFlashRegion]>,
    }
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for TbfHeaderV2 {
        #[inline]
        fn clone(&self) -> TbfHeaderV2 { loop  { } }
    }
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2 { }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2 {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    crate enum TbfHeader {
        TbfHeaderV2(TbfHeaderV2),
        Padding(&'static TbfHeaderV2Base),
    }
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeader {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            loop  { }
        }
    }
    impl TbfHeader {
        crate fn is_app(&self) -> bool { loop  { } }
        crate fn enabled(&self) -> bool { loop  { } }
        crate fn get_total_size(&self) -> u32 { loop  { } }
        crate fn get_minimum_app_ram_size(&self) -> u32 { loop  { } }
        crate fn get_protected_size(&self) -> u32 { loop  { } }
        crate fn get_init_function_offset(&self) -> u32 { loop  { } }
        crate fn get_package_name(&self) -> &'static str { loop  { } }
        crate fn number_writeable_flash_regions(&self) -> usize { loop  { } }
        crate fn get_writeable_flash_region(&self, index: usize)
         -> (u32, u32) {
            loop  { }
        }
    }
    #[allow(clippy :: cast_ptr_alignment)]
    crate unsafe fn parse_and_validate_tbf_header(address: *const u8)
     -> Option<TbfHeader> {
        loop  { }
    }
}
pub use crate::callback::{AppId, Callback};
pub use crate::driver::Driver;
pub use crate::grant::Grant;
pub use crate::mem::{AppPtr, AppSlice, Private, Shared};
pub use crate::platform::systick::SysTick;
pub use crate::platform::{mpu, Chip, Platform};
pub use crate::platform::{ClockInterface, NoClockControl, NO_CLOCK_CONTROL};
pub use crate::returncode::ReturnCode;
pub use crate::sched::Kernel;
pub mod procs {
    pub use crate::process::{load_processes, FaultResponse, FunctionCall,
                             Process, ProcessType};
}
