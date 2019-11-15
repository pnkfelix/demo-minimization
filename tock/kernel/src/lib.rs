#![feature(derive_clone_copy, compiler_builtins_lib, fmt_internals, core_panic, derive_eq)]
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
        pub fn has_tasks() -> bool {
            DEFERRED_CALL.load(Ordering::Relaxed) != 0
        }
        pub struct DeferredCall<T>(T);
        impl <T: Into<usize> + TryFrom<usize> + Copy> DeferredCall<T> {
            pub const unsafe fn new(task: T) -> Self { DeferredCall(task) }
            pub fn set(&self) {
                let val = DEFERRED_CALL.load(Ordering::Relaxed);
                let new_val = val | (1 << self.0.into());
                DEFERRED_CALL.store(new_val, Ordering::Relaxed);
            }
            pub fn next_pending() -> Option<T> {
                let val = DEFERRED_CALL.load(Ordering::Relaxed);
                if val == 0 {
                    None
                } else {
                    let bit = val.trailing_zeros() as usize;
                    let new_val = val & !(1 << bit);
                    DEFERRED_CALL.store(new_val, Ordering::Relaxed);
                    bit.try_into().ok()
                }
            }
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
            fn default() -> DynamicDeferredCallClientState {
                DynamicDeferredCallClientState{scheduled: Cell::new(false),
                                               client: OptionalCell::empty(),}
            }
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
                DynamicDeferredCall{client_states,
                                    handle_counter: Cell::new(0),
                                    call_pending: Cell::new(false),}
            }
            pub unsafe fn set_global_instance(ddc:
                                                  &'static DynamicDeferredCall)
             -> bool {
                (*DYNAMIC_DEFERRED_CALL.get_or_insert(ddc)) as *const _ ==
                    ddc as *const _
            }
            pub unsafe fn call_global_instance() -> bool {
                DYNAMIC_DEFERRED_CALL.map(|ddc| ddc.call()).is_some()
            }
            pub unsafe fn call_global_instance_while<F: Fn() -> bool>(f: F)
             -> bool {
                DYNAMIC_DEFERRED_CALL.map(move |ddc|
                                              ddc.call_while(f)).is_some()
            }
            pub unsafe fn global_instance_calls_pending() -> Option<bool> {
                DYNAMIC_DEFERRED_CALL.map(|ddc| ddc.has_pending())
            }
            pub fn set(&self, handle: DeferredCallHandle) -> Option<bool> {
                let DeferredCallHandle(client_pos) = handle;
                let client_state = &self.client_states[client_pos];
                if let (call_set, true) =
                       (&client_state.scheduled,
                        client_state.client.is_some()) {
                    if call_set.get() {
                        Some(false)
                    } else {
                        call_set.set(true);
                        self.call_pending.set(true);
                        Some(true)
                    }
                } else { None }
            }
            pub fn register(&self,
                            ddc_client:
                                &'static dyn DynamicDeferredCallClient)
             -> Option<DeferredCallHandle> {
                let current_counter = self.handle_counter.get();
                if current_counter < self.client_states.len() {
                    let client_state = &self.client_states[current_counter];
                    client_state.scheduled.set(false);
                    client_state.client.set(ddc_client);
                    self.handle_counter.set(current_counter + 1);
                    Some(DeferredCallHandle(current_counter))
                } else { None }
            }
            pub fn has_pending(&self) -> bool { self.call_pending.get() }
            pub(self) fn call(&self) { self.call_while(|| true) }
            pub(self) fn call_while<F: Fn() -> bool>(&self, f: F) {
                if self.call_pending.get() {
                    self.call_pending.set(false);
                    self.client_states.iter().enumerate().filter(|(_i,
                                                                   client_state)|
                                                                     client_state.scheduled.get()).filter_map(|(i,
                                                                                                                client_state)|
                                                                                                                  {
                                                                                                                      client_state.client.map(|c|
                                                                                                                                                  (i,
                                                                                                                                                   &client_state.scheduled,
                                                                                                                                                   *c))
                                                                                                                  }).take_while(|_|
                                                                                                                                    f()).for_each(|(i,
                                                                                                                                                    call_reqd,
                                                                                                                                                    client)|
                                                                                                                                                      {
                                                                                                                                                          call_reqd.set(false);
                                                                                                                                                          client.call(DeferredCallHandle(i));
                                                                                                                                                      });
                }
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
            fn clone(&self) -> DeferredCallHandle {
                { let _: ::core::clone::AssertParamIsClone<usize>; *self }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DeferredCallHandle {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    DeferredCallHandle(ref __self_0_0) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DeferredCallHandle");
                        let _ = debug_trait_builder.field(&&(*__self_0_0));
                        debug_trait_builder.finish()
                    }
                }
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
            fn next(&mut self) -> Option<&'a T> {
                match self.cur {
                    Some(res) => { self.cur = res.next().0.get(); Some(res) }
                    None => None,
                }
            }
        }
        impl <T: ?Sized + ListNode<'a, T>> List<'a, T> {
            pub const fn new() -> List<'a, T> {
                List{head: ListLink(Cell::new(None)),}
            }
            pub fn head(&self) -> Option<&'a T> { self.head.0.get() }
            pub fn push_head(&self, node: &'a T) {
                node.next().0.set(self.head.0.get());
                self.head.0.set(Some(node));
            }
            pub fn push_tail(&self, node: &'a T) {
                node.next().0.set(None);
                match self.iter().last() {
                    Some(last) => last.next().0.set(Some(node)),
                    None => self.push_head(node),
                }
            }
            pub fn pop_head(&self) -> Option<&'a T> {
                let remove = self.head.0.get();
                match remove {
                    Some(node) => self.head.0.set(node.next().0.get()),
                    None => self.head.0.set(None),
                }
                remove
            }
            pub fn iter(&self) -> ListIterator<'a, T> {
                ListIterator{cur: self.head.0.get(),}
            }
        }
    }
    pub mod math {
        use core::convert::{From, Into};
        use core::intrinsics as int;
        pub fn sqrtf32(num: f32) -> f32 { unsafe { int::sqrtf32(num) } }
        extern "C" {
            fn __errno() -> &'static mut i32;
        }
        pub fn get_errno() -> i32 {
            unsafe {
                let errnoaddr = __errno();
                let ret = *errnoaddr;
                *errnoaddr = 0;
                ret
            }
        }
        pub fn closest_power_of_two(mut num: u32) -> u32 {
            num -= 1;
            num |= num >> 1;
            num |= num >> 2;
            num |= num >> 4;
            num |= num >> 8;
            num |= num >> 16;
            num += 1;
            num
        }
        pub struct PowerOfTwo(u32);
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for PowerOfTwo { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for PowerOfTwo {
            #[inline]
            fn clone(&self) -> PowerOfTwo {
                { let _: ::core::clone::AssertParamIsClone<u32>; *self }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for PowerOfTwo {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    PowerOfTwo(ref __self_0_0) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("PowerOfTwo");
                        let _ = debug_trait_builder.field(&&(*__self_0_0));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for PowerOfTwo {
            #[inline]
            fn eq(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        (*__self_0_0) == (*__self_1_0),
                    },
                }
            }
            #[inline]
            fn ne(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        (*__self_0_0) != (*__self_1_0),
                    },
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialOrd for PowerOfTwo {
            #[inline]
            fn partial_cmp(&self, other: &PowerOfTwo)
             -> ::core::option::Option<::core::cmp::Ordering> {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        match ::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                   &(*__self_1_0))
                            {
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                            =>
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal),
                            cmp => cmp,
                        },
                    },
                }
            }
            #[inline]
            fn lt(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Greater)
                            == ::core::cmp::Ordering::Less,
                    },
                }
            }
            #[inline]
            fn le(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Greater)
                            != ::core::cmp::Ordering::Greater,
                    },
                }
            }
            #[inline]
            fn gt(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Less)
                            == ::core::cmp::Ordering::Greater,
                    },
                }
            }
            #[inline]
            fn ge(&self, other: &PowerOfTwo) -> bool {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Less)
                            != ::core::cmp::Ordering::Less,
                    },
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for PowerOfTwo {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () {
                { let _: ::core::cmp::AssertParamIsEq<u32>; }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Ord for PowerOfTwo {
            #[inline]
            fn cmp(&self, other: &PowerOfTwo) -> ::core::cmp::Ordering {
                match *other {
                    PowerOfTwo(ref __self_1_0) =>
                    match *self {
                        PowerOfTwo(ref __self_0_0) =>
                        match ::core::cmp::Ord::cmp(&(*__self_0_0),
                                                    &(*__self_1_0)) {
                            ::core::cmp::Ordering::Equal =>
                            ::core::cmp::Ordering::Equal,
                            cmp => cmp,
                        },
                    },
                }
            }
        }
        impl PowerOfTwo {
            pub fn exp<R>(self) -> R where R: From<u32> { From::from(self.0) }
            pub fn floor<F: Into<u32>>(f: F) -> PowerOfTwo {
                PowerOfTwo(log_base_two(f.into()))
            }
            pub fn ceiling<F: Into<u32>>(f: F) -> PowerOfTwo {
                PowerOfTwo(log_base_two(closest_power_of_two(f.into())))
            }
            pub fn zero() -> PowerOfTwo { PowerOfTwo(0) }
            pub fn as_num<F: From<u32>>(self) -> F { (1 << self.0).into() }
        }
        pub fn log_base_two(num: u32) -> u32 {
            if num == 0 { 0 } else { 31 - num.leading_zeros() }
        }
        pub fn log_base_two_u64(num: u64) -> u32 {
            if num == 0 { 0 } else { 63 - num.leading_zeros() }
        }
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
                let registers = peripheral_hardware.get_registers();
                let clock = peripheral_hardware.get_clock();
                peripheral_hardware.before_peripheral_access(clock,
                                                             registers);
                PeripheralManager{registers, peripheral_hardware, clock,}
            }
        }
        impl <H, C> Drop for PeripheralManager<'a, H, C> where H: 'a +
         PeripheralManagement<C>, C: 'a + ClockInterface {
            fn drop(&mut self) {
                self.peripheral_hardware.after_peripheral_access(self.clock,
                                                                 self.registers);
            }
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
            pub fn new(ring: &'a mut [T]) -> RingBuffer<'a, T> {
                RingBuffer{head: 0, tail: 0, ring: ring,}
            }
            pub fn available_len(&self) -> usize {
                self.ring.len().saturating_sub(1 + queue::Queue::len(self))
            }
            pub fn as_slices(&'a self) -> (Option<&'a [T]>, Option<&'a [T]>) {
                if self.head < self.tail {
                    (Some(&self.ring[self.head..self.tail]), None)
                } else if self.head > self.tail {
                    let (left, right) = self.ring.split_at(self.head);
                    (Some(right),
                     if self.tail == 0 {
                         None
                     } else { Some(&left[..self.tail]) })
                } else { (None, None) }
            }
        }
        impl <T: Copy> queue::Queue<T> for RingBuffer<'a, T> {
            fn has_elements(&self) -> bool { self.head != self.tail }
            fn is_full(&self) -> bool {
                self.head == ((self.tail + 1) % self.ring.len())
            }
            fn len(&self) -> usize {
                if self.tail > self.head {
                    self.tail - self.head
                } else if self.tail < self.head {
                    (self.ring.len() - self.head) + self.tail
                } else { 0 }
            }
            fn enqueue(&mut self, val: T) -> bool {
                if ((self.tail + 1) % self.ring.len()) == self.head {
                    false
                } else {
                    self.ring[self.tail] = val;
                    self.tail = (self.tail + 1) % self.ring.len();
                    true
                }
            }
            fn dequeue(&mut self) -> Option<T> {
                if self.has_elements() {
                    let val = self.ring[self.head];
                    self.head = (self.head + 1) % self.ring.len();
                    Some(val)
                } else { None }
            }
            fn empty(&mut self) { self.head = 0; self.tail = 0; }
            fn retain<F>(&mut self, mut f: F) where F: FnMut(&T) -> bool {
                let len = self.ring.len();
                let mut src = self.head;
                let mut dst = self.head;
                while src != self.tail {
                    if f(&self.ring[src]) {
                        if src != dst { self.ring[dst] = self.ring[src]; }
                        dst = (dst + 1) % len;
                    }
                    src = (src + 1) % len;
                }
                self.tail = dst;
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
                match *self {
                    StaticRef { ptr: ref __self_0_0 } => {
                        let mut debug_trait_builder =
                            f.debug_struct("StaticRef");
                        let _ =
                            debug_trait_builder.field("ptr", &&(*__self_0_0));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl <T> StaticRef<T> {
            pub const unsafe fn new(ptr: *const T) -> StaticRef<T> {
                StaticRef{ptr: ptr,}
            }
        }
        impl <T> Clone for StaticRef<T> {
            fn clone(&self) -> Self { StaticRef{ptr: self.ptr,} }
        }
        impl <T> Copy for StaticRef<T> { }
        impl <T> Deref for StaticRef<T> {
            type
            Target
            =
            T;
            fn deref(&self) -> &'static T { unsafe { &*self.ptr } }
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
        panic_begin(nop);
        panic_banner(writer, panic_info);
        flush(writer);
        panic_process_info(processes, writer);
        panic_blink_forever(leds)
    }
    pub unsafe fn panic_begin(nop: &dyn Fn()) {
        for _ in 0..200000 { nop(); }
    }
    pub unsafe fn panic_banner<W: Write>(writer: &mut W,
                                         panic_info: &PanicInfo) {
        if let Some(location) = panic_info.location() {
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1(&["\r\n\nKernel panic at ",
                                                                  ":",
                                                                  ":\r\n\t\""],
                                                                &match (&location.file(),
                                                                        &location.line())
                                                                     {
                                                                     (arg0,
                                                                      arg1) =>
                                                                     [::core::fmt::ArgumentV1::new(arg0,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg1,
                                                                                                   ::core::fmt::Display::fmt)],
                                                                 }));
        } else {
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1(&["\r\n\nKernel panic:\r\n\t\""],
                                                                &match () {
                                                                     () => [],
                                                                 }));
        }
        if let Some(args) = panic_info.message() {
            let _ = write(writer, *args);
        }
        let _ = writer.write_str("\"\r\n");
        let _ =
            writer.write_fmt(::core::fmt::Arguments::new_v1(&["\tKernel version ",
                                                              "\r\n"],
                                                            &match (&::core::option::Option::None::<&'static str>.unwrap_or("unknown"),)
                                                                 {
                                                                 (arg0,) =>
                                                                 [::core::fmt::ArgumentV1::new(arg0,
                                                                                               ::core::fmt::Display::fmt)],
                                                             }));
    }
    pub unsafe fn panic_process_info<W: Write>(procs:
                                                   &'static [Option<&'static dyn ProcessType>],
                                               writer: &mut W) {
        if !procs.is_empty() {
            procs[0].as_ref().map(|process| { process.fault_fmt(writer); });
        }
        let _ =
            writer.write_fmt(::core::fmt::Arguments::new_v1(&["\r\n---| App Status |---\r\n"],
                                                            &match () {
                                                                 () => [],
                                                             }));
        for idx in 0..procs.len() {
            procs[idx].as_ref().map(|process|
                                        {
                                            process.process_detail_fmt(writer);
                                        });
        }
    }
    pub fn panic_blink_forever<L: hil::led::Led>(leds: &mut [&mut L]) -> ! {
        leds.iter_mut().for_each(|led| led.init());
        loop  {
            for _ in 0..1000000 { leds.iter_mut().for_each(|led| led.on()); }
            for _ in 0..100000 { leds.iter_mut().for_each(|led| led.off()); }
            for _ in 0..1000000 { leds.iter_mut().for_each(|led| led.on()); }
            for _ in 0..500000 { leds.iter_mut().for_each(|led| led.off()); }
        }
    }
    pub static mut DEBUG_GPIOS:
               (Option<&'static dyn hil::gpio::Pin>,
                Option<&'static dyn hil::gpio::Pin>,
                Option<&'static dyn hil::gpio::Pin>) =
        (None, None, None);
    pub unsafe fn assign_gpios(gpio0: Option<&'static dyn hil::gpio::Pin>,
                               gpio1: Option<&'static dyn hil::gpio::Pin>,
                               gpio2: Option<&'static dyn hil::gpio::Pin>) {
        DEBUG_GPIOS.0 = gpio0;
        DEBUG_GPIOS.1 = gpio1;
        DEBUG_GPIOS.2 = gpio2;
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
        match ptr::read(&DEBUG_WRITER) {
            Some(x) => x,
            None => {
                ::core::panicking::panic(&("Must call `set_debug_writer_wrapper` in board initialization.",
                                           "/Users/felixklock/Dev/Mozilla/issue65774/demo-minimization/tock/kernel/src/debug.rs",
                                           228u32, 17u32))
            }
        }
    }
    pub unsafe fn set_debug_writer_wrapper(debug_writer:
                                               &'static mut DebugWriterWrapper) {
        DEBUG_WRITER = Some(debug_writer);
    }
    impl DebugWriterWrapper {
        pub fn new(dw: &'static DebugWriter) -> DebugWriterWrapper {
            DebugWriterWrapper{dw: MapCell::new(dw),}
        }
    }
    impl DebugWriter {
        pub fn new(uart: &'static dyn hil::uart::Transmit,
                   out_buffer: &'static mut [u8],
                   internal_buffer: &'static mut RingBuffer<'static, u8>)
         -> DebugWriter {
            DebugWriter{uart: uart,
                        output_buffer: TakeCell::new(out_buffer),
                        internal_buffer: TakeCell::new(internal_buffer),
                        count: Cell::new(0),}
        }
        fn increment_count(&self) { self.count.increment(); }
        fn get_count(&self) -> usize { self.count.get() }
        fn publish_str(&self) {
            self.internal_buffer.map(|ring_buffer|
                                         {
                                             if let Some(out_buffer) =
                                                    self.output_buffer.take()
                                                {
                                                 let mut count = 0;
                                                 for dst in
                                                     out_buffer.iter_mut() {
                                                     match ring_buffer.dequeue()
                                                         {
                                                         Some(src) => {
                                                             *dst = src;
                                                             count += 1;
                                                         }
                                                         None => { break ; }
                                                     }
                                                 }
                                                 if count != 0 {
                                                     let (_rval, opt) =
                                                         self.uart.transmit_buffer(out_buffer,
                                                                                   count);
                                                     self.output_buffer.put(opt);
                                                 }
                                             }
                                         });
        }
        fn extract(&self) -> Option<&mut RingBuffer<'static, u8>> {
            self.internal_buffer.take()
        }
    }
    impl hil::uart::TransmitClient for DebugWriter {
        fn transmitted_buffer(&self, buffer: &'static mut [u8],
                              _tx_len: usize, _rcode: ReturnCode) {
            self.output_buffer.replace(buffer);
            if self.internal_buffer.map_or(false, |buf| buf.has_elements()) {
                self.publish_str();
            }
        }
        fn transmitted_word(&self, _rcode: ReturnCode) { }
    }
    impl DebugWriterWrapper {
        fn increment_count(&self) {
            self.dw.map(|dw| { dw.increment_count(); });
        }
        fn get_count(&self) -> usize {
            self.dw.map_or(0, |dw| dw.get_count())
        }
        fn publish_str(&self) { self.dw.map(|dw| { dw.publish_str(); }); }
        fn extract(&self) -> Option<&mut RingBuffer<'static, u8>> {
            self.dw.map_or(None, |dw| dw.extract())
        }
    }
    impl Write for DebugWriterWrapper {
        fn write_str(&mut self, s: &str) -> Result {
            const FULL_MSG: &[u8] = b"\n*** DEBUG BUFFER FULL ***\n";
            self.dw.map(|dw|
                            {
                                dw.internal_buffer.map(|ring_buffer|
                                                           {
                                                               let bytes =
                                                                   s.as_bytes();
                                                               let available_len_for_msg =
                                                                   ring_buffer.available_len().saturating_sub(FULL_MSG.len());
                                                               if available_len_for_msg
                                                                      >=
                                                                      bytes.len()
                                                                  {
                                                                   for &b in
                                                                       bytes {
                                                                       ring_buffer.enqueue(b);
                                                                   }
                                                               } else {
                                                                   for &b in
                                                                       &bytes[..available_len_for_msg]
                                                                       {
                                                                       ring_buffer.enqueue(b);
                                                                   }
                                                                   for &b in
                                                                       FULL_MSG
                                                                       {
                                                                       ring_buffer.enqueue(b);
                                                                   }
                                                               }
                                                           });
                            });
            Ok(())
        }
    }
    pub fn begin_debug_fmt(args: Arguments) {
        unsafe {
            let writer = get_debug_writer();
            let _ = write(writer, args);
            let _ = writer.write_str("\r\n");
            writer.publish_str();
        }
    }
    pub fn begin_debug_verbose_fmt(args: Arguments,
                                   file_line: &(&'static str, u32)) {
        unsafe {
            let writer = get_debug_writer();
            writer.increment_count();
            let count = writer.get_count();
            let (file, line) = *file_line;
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1(&["TOCK_DEBUG(",
                                                                  "): ", ":",
                                                                  ": "],
                                                                &match (&count,
                                                                        &file,
                                                                        &line)
                                                                     {
                                                                     (arg0,
                                                                      arg1,
                                                                      arg2) =>
                                                                     [::core::fmt::ArgumentV1::new(arg0,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg1,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg2,
                                                                                                   ::core::fmt::Display::fmt)],
                                                                 }));
            let _ = write(writer, args);
            let _ = writer.write_str("\r\n");
            writer.publish_str();
        }
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
    pub unsafe fn flush<W: Write>(writer: &mut W) {
        let debug_writer = get_debug_writer();
        if let Some(ring_buffer) = debug_writer.extract() {
            if ring_buffer.has_elements() {
                let _ =
                    writer.write_str("\r\n---| Debug buffer not empty. Flushing. May repeat some of last message(s):\r\n");
                let (left, right) = ring_buffer.as_slices();
                if let Some(slice) = left {
                    let s = str::from_utf8_unchecked(slice);
                    let _ = writer.write_str(s);
                }
                if let Some(slice) = right {
                    let s = str::from_utf8_unchecked(slice);
                    let _ = writer.write_str(s);
                }
            }
        }
    }
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
            fn eq(&self, other: &RadioChannel) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for RadioChannel {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&RadioChannel::DataChannel0,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel0");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel1,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel1");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel2,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel2");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel3,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel3");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel4,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel4");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel5,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel5");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel6,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel6");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel7,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel7");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel8,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel8");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel9,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel9");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel10,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel10");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel11,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel11");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel12,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel12");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel13,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel13");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel14,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel14");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel15,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel15");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel16,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel16");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel17,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel17");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel18,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel18");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel19,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel19");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel20,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel20");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel21,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel21");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel22,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel22");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel23,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel23");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel24,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel24");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel25,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel25");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel26,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel26");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel27,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel27");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel28,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel28");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel29,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel29");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel30,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel30");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel31,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel31");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel32,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel32");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel33,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel33");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel34,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel34");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel35,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel35");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::DataChannel36,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataChannel36");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::AdvertisingChannel37,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("AdvertisingChannel37");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::AdvertisingChannel38,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("AdvertisingChannel38");
                        debug_trait_builder.finish()
                    }
                    (&RadioChannel::AdvertisingChannel39,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("AdvertisingChannel39");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for RadioChannel { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for RadioChannel {
            #[inline]
            fn clone(&self) -> RadioChannel { { *self } }
        }
        impl RadioChannel {
            pub fn get_channel_index(&self) -> u32 {
                match *self {
                    RadioChannel::DataChannel0 => 0,
                    RadioChannel::DataChannel1 => 1,
                    RadioChannel::DataChannel2 => 2,
                    RadioChannel::DataChannel3 => 3,
                    RadioChannel::DataChannel4 => 4,
                    RadioChannel::DataChannel5 => 5,
                    RadioChannel::DataChannel6 => 6,
                    RadioChannel::DataChannel7 => 7,
                    RadioChannel::DataChannel8 => 8,
                    RadioChannel::DataChannel9 => 9,
                    RadioChannel::DataChannel10 => 10,
                    RadioChannel::DataChannel11 => 11,
                    RadioChannel::DataChannel12 => 12,
                    RadioChannel::DataChannel13 => 13,
                    RadioChannel::DataChannel14 => 14,
                    RadioChannel::DataChannel15 => 15,
                    RadioChannel::DataChannel16 => 16,
                    RadioChannel::DataChannel17 => 17,
                    RadioChannel::DataChannel18 => 18,
                    RadioChannel::DataChannel19 => 19,
                    RadioChannel::DataChannel20 => 20,
                    RadioChannel::DataChannel21 => 21,
                    RadioChannel::DataChannel22 => 22,
                    RadioChannel::DataChannel23 => 23,
                    RadioChannel::DataChannel24 => 24,
                    RadioChannel::DataChannel25 => 25,
                    RadioChannel::DataChannel26 => 26,
                    RadioChannel::DataChannel27 => 27,
                    RadioChannel::DataChannel28 => 28,
                    RadioChannel::DataChannel29 => 29,
                    RadioChannel::DataChannel30 => 30,
                    RadioChannel::DataChannel31 => 31,
                    RadioChannel::DataChannel32 => 32,
                    RadioChannel::DataChannel33 => 33,
                    RadioChannel::DataChannel34 => 34,
                    RadioChannel::DataChannel35 => 35,
                    RadioChannel::DataChannel36 => 36,
                    RadioChannel::AdvertisingChannel37 => 37,
                    RadioChannel::AdvertisingChannel38 => 38,
                    RadioChannel::AdvertisingChannel39 => 39,
                }
            }
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
            fn clone(&self) -> CrcAlg { { *self } }
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
                match (&*self,) {
                    (&InterruptMode::RisingEdge,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("RisingEdge");
                        debug_trait_builder.finish()
                    }
                    (&InterruptMode::FallingEdge,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("FallingEdge");
                        debug_trait_builder.finish()
                    }
                    (&InterruptMode::HighLevel,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("HighLevel");
                        debug_trait_builder.finish()
                    }
                    (&InterruptMode::LowLevel,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("LowLevel");
                        debug_trait_builder.finish()
                    }
                }
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
                match (&*self,) {
                    (&Continue::More,) => {
                        let mut debug_trait_builder = f.debug_tuple("More");
                        debug_trait_builder.finish()
                    }
                    (&Continue::Done,) => {
                        let mut debug_trait_builder = f.debug_tuple("Done");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Continue {
            #[inline]
            fn eq(&self, other: &Continue) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
            fn clone(&self) -> Error { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&Error::CommandComplete,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("CommandComplete");
                        debug_trait_builder.finish()
                    }
                    (&Error::FlashError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("FlashError");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
                match (&*self,) {
                    (&FloatingState::PullUp,) => {
                        let mut debug_trait_builder = f.debug_tuple("PullUp");
                        debug_trait_builder.finish()
                    }
                    (&FloatingState::PullDown,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("PullDown");
                        debug_trait_builder.finish()
                    }
                    (&FloatingState::PullNone,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("PullNone");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        pub enum InterruptEdge { RisingEdge, FallingEdge, EitherEdge, }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for InterruptEdge {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&InterruptEdge::RisingEdge,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("RisingEdge");
                        debug_trait_builder.finish()
                    }
                    (&InterruptEdge::FallingEdge,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("FallingEdge");
                        debug_trait_builder.finish()
                    }
                    (&InterruptEdge::EitherEdge,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("EitherEdge");
                        debug_trait_builder.finish()
                    }
                }
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
                match (&*self,) {
                    (&Configuration::LowPower,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("LowPower");
                        debug_trait_builder.finish()
                    }
                    (&Configuration::Input,) => {
                        let mut debug_trait_builder = f.debug_tuple("Input");
                        debug_trait_builder.finish()
                    }
                    (&Configuration::Output,) => {
                        let mut debug_trait_builder = f.debug_tuple("Output");
                        debug_trait_builder.finish()
                    }
                    (&Configuration::InputOutput,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("InputOutput");
                        debug_trait_builder.finish()
                    }
                    (&Configuration::Function,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Function");
                        debug_trait_builder.finish()
                    }
                    (&Configuration::Other,) => {
                        let mut debug_trait_builder = f.debug_tuple("Other");
                        debug_trait_builder.finish()
                    }
                }
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
            fn is_input(&self) -> bool {
                match self.configuration() {
                    Configuration::Input | Configuration::InputOutput => true,
                    _ => false,
                }
            }
            fn is_output(&self) -> bool {
                match self.configuration() {
                    Configuration::Output | Configuration::InputOutput =>
                    true,
                    _ => false,
                }
            }
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
                InterruptValueWrapper{value: Cell::new(0),
                                      client: OptionalCell::empty(),
                                      source: pin,}
            }
            pub fn finalize(&'static self) -> &'static Self {
                self.source.set_client(self);
                self
            }
        }
        impl InterruptWithValue for InterruptValueWrapper {
            fn set_value(&self, value: u32) { self.value.set(value); }
            fn value(&self) -> u32 { self.value.get() }
            fn set_client(&self, client: &'static dyn ClientWithValue) {
                self.client.replace(client);
            }
            fn is_pending(&self) -> bool { self.source.is_pending() }
            fn enable_interrupts(&self, edge: InterruptEdge) -> ReturnCode {
                self.source.enable_interrupts(edge);
                ReturnCode::SUCCESS
            }
            fn disable_interrupts(&self) { self.source.disable_interrupts(); }
        }
        impl Input for InterruptValueWrapper {
            fn read(&self) -> bool { self.source.read() }
        }
        impl Configure for InterruptValueWrapper {
            fn configuration(&self) -> Configuration {
                self.source.configuration()
            }
            fn make_output(&self) -> Configuration {
                self.source.make_output()
            }
            fn disable_output(&self) -> Configuration {
                self.source.disable_output()
            }
            fn make_input(&self) -> Configuration { self.source.make_input() }
            fn disable_input(&self) -> Configuration {
                self.source.disable_input()
            }
            fn deactivate_to_low_power(&self) {
                self.source.deactivate_to_low_power();
            }
            fn set_floating_state(&self, state: FloatingState) {
                self.source.set_floating_state(state);
            }
            fn floating_state(&self) -> FloatingState {
                self.source.floating_state()
            }
            fn is_input(&self) -> bool { self.source.is_input() }
            fn is_output(&self) -> bool { self.source.is_input() }
        }
        impl Output for InterruptValueWrapper {
            fn set(&self) { self.source.set(); }
            fn clear(&self) { self.source.clear(); }
            fn toggle(&self) -> bool { self.source.toggle() }
        }
        impl InterruptValuePin for InterruptValueWrapper { }
        impl Pin for InterruptValueWrapper { }
        impl Client for InterruptValueWrapper {
            fn fired(&self) { self.client.map(|c| c.fired(self.value())); }
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
            fn clone(&self) -> Error { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&Error::AddressNak,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("AddressNak");
                        debug_trait_builder.finish()
                    }
                    (&Error::DataNak,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("DataNak");
                        debug_trait_builder.finish()
                    }
                    (&Error::ArbitrationLost,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ArbitrationLost");
                        debug_trait_builder.finish()
                    }
                    (&Error::Overrun,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Overrun");
                        debug_trait_builder.finish()
                    }
                    (&Error::CommandComplete,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("CommandComplete");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
        impl Display for Error {
            fn fmt(&self, fmt: &mut Formatter) -> Result {
                let display_str =
                    match *self {
                        Error::AddressNak => "I2C Address Not Acknowledged",
                        Error::DataNak => "I2C Data Not Acknowledged",
                        Error::ArbitrationLost => "I2C Bus Arbitration Lost",
                        Error::Overrun => "I2C receive overrun",
                        Error::CommandComplete => "I2C Command Completed",
                    };
                fmt.write_fmt(::core::fmt::Arguments::new_v1(&[""],
                                                             &match (&display_str,)
                                                                  {
                                                                  (arg0,) =>
                                                                  [::core::fmt::ArgumentV1::new(arg0,
                                                                                                ::core::fmt::Display::fmt)],
                                                              }))
            }
        }
        pub enum SlaveTransmissionType { Write, Read, }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for SlaveTransmissionType { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for SlaveTransmissionType {
            #[inline]
            fn clone(&self) -> SlaveTransmissionType { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for SlaveTransmissionType {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&SlaveTransmissionType::Write,) => {
                        let mut debug_trait_builder = f.debug_tuple("Write");
                        debug_trait_builder.finish()
                    }
                    (&SlaveTransmissionType::Read,) => {
                        let mut debug_trait_builder = f.debug_tuple("Read");
                        debug_trait_builder.finish()
                    }
                }
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
            pub fn new(p: &'a mut dyn gpio::Pin) -> LedHigh {
                LedHigh{pin: p,}
            }
        }
        impl LedLow<'a> {
            pub fn new(p: &'a mut dyn gpio::Pin) -> LedLow { LedLow{pin: p,} }
        }
        impl Led for LedHigh<'a> {
            fn init(&mut self) { self.pin.make_output(); }
            fn on(&mut self) { self.pin.set(); }
            fn off(&mut self) { self.pin.clear(); }
            fn toggle(&mut self) { self.pin.toggle(); }
            fn read(&self) -> bool { self.pin.read() }
        }
        impl Led for LedLow<'a> {
            fn init(&mut self) { self.pin.make_output(); }
            fn on(&mut self) { self.pin.clear(); }
            fn off(&mut self) { self.pin.set(); }
            fn toggle(&mut self) { self.pin.toggle(); }
            fn read(&self) -> bool { !self.pin.read() }
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
                match (&*self,) {
                    (&Continue::More,) => {
                        let mut debug_trait_builder = f.debug_tuple("More");
                        debug_trait_builder.finish()
                    }
                    (&Continue::Done,) => {
                        let mut debug_trait_builder = f.debug_tuple("Done");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Continue {
            #[inline]
            fn eq(&self, other: &Continue) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
            fn read_light_intensity(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
        }
        pub trait AmbientLightClient {
            fn callback(&self, lux: usize);
        }
        pub trait NineDof {
            fn set_client(&self, client: &'static dyn NineDofClient);
            fn read_accelerometer(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
            fn read_magnetometer(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
            fn read_gyroscope(&self) -> ReturnCode { ReturnCode::ENODEVICE }
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
            fn clone(&self) -> DataOrder { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DataOrder {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&DataOrder::MSBFirst,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("MSBFirst");
                        debug_trait_builder.finish()
                    }
                    (&DataOrder::LSBFirst,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("LSBFirst");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        pub enum ClockPolarity { IdleLow, IdleHigh, }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPolarity { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPolarity {
            #[inline]
            fn clone(&self) -> ClockPolarity { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for ClockPolarity {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&ClockPolarity::IdleLow,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("IdleLow");
                        debug_trait_builder.finish()
                    }
                    (&ClockPolarity::IdleHigh,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("IdleHigh");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for ClockPolarity {
            #[inline]
            fn eq(&self, other: &ClockPolarity) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
        pub enum ClockPhase { SampleLeading, SampleTrailing, }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPhase { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPhase {
            #[inline]
            fn clone(&self) -> ClockPhase { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for ClockPhase {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&ClockPhase::SampleLeading,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("SampleLeading");
                        debug_trait_builder.finish()
                    }
                    (&ClockPhase::SampleTrailing,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("SampleTrailing");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for ClockPhase {
            #[inline]
            fn eq(&self, other: &ClockPhase) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
                match *self {
                    Freq16MHz => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Freq16MHz");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl Frequency for Freq16MHz {
            fn frequency() -> u32 { 16000000 }
        }
        pub struct Freq32KHz;
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq32KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    Freq32KHz => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Freq32KHz");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl Frequency for Freq32KHz {
            fn frequency() -> u32 { 32768 }
        }
        pub struct Freq16KHz;
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq16KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    Freq16KHz => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Freq16KHz");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl Frequency for Freq16KHz {
            fn frequency() -> u32 { 16000 }
        }
        pub struct Freq1KHz;
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Freq1KHz {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    Freq1KHz => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Freq1KHz");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl Frequency for Freq1KHz {
            fn frequency() -> u32 { 1000 }
        }
        pub trait Alarm<'a, W = u32>: Time<W> {
            fn set_alarm(&self, tics: W);
            fn get_alarm(&self)
            -> W;
            fn set_client(&'a self, client: &'a dyn AlarmClient);
            fn is_enabled(&self)
            -> bool;
            fn enable(&self) { self.set_alarm(self.get_alarm()) }
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
            fn is_oneshot(&self) -> bool { self.interval().is_none() }
            fn is_repeating(&self) -> bool { self.interval().is_some() }
            fn time_remaining(&self)
            -> Option<W>;
            fn is_enabled(&self) -> bool { self.time_remaining().is_some() }
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
            fn clone(&self) -> StopBits { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for StopBits {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&StopBits::One,) => {
                        let mut debug_trait_builder = f.debug_tuple("One");
                        debug_trait_builder.finish()
                    }
                    (&StopBits::Two,) => {
                        let mut debug_trait_builder = f.debug_tuple("Two");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for StopBits {
            #[inline]
            fn eq(&self, other: &StopBits) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
        pub enum Parity { None = 0, Odd = 1, Even = 2, }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Parity { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Parity {
            #[inline]
            fn clone(&self) -> Parity { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Parity {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&Parity::None,) => {
                        let mut debug_trait_builder = f.debug_tuple("None");
                        debug_trait_builder.finish()
                    }
                    (&Parity::Odd,) => {
                        let mut debug_trait_builder = f.debug_tuple("Odd");
                        debug_trait_builder.finish()
                    }
                    (&Parity::Even,) => {
                        let mut debug_trait_builder = f.debug_tuple("Even");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Parity {
            #[inline]
            fn eq(&self, other: &Parity) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
        pub enum Width { Six = 6, Seven = 7, Eight = 8, }
                #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Width { }
                #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Width {
            #[inline]
            fn clone(&self) -> Width { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Width {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&Width::Six,) => {
                        let mut debug_trait_builder = f.debug_tuple("Six");
                        debug_trait_builder.finish()
                    }
                    (&Width::Seven,) => {
                        let mut debug_trait_builder = f.debug_tuple("Seven");
                        debug_trait_builder.finish()
                    }
                    (&Width::Eight,) => {
                        let mut debug_trait_builder = f.debug_tuple("Eight");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Width {
            #[inline]
            fn eq(&self, other: &Width) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
            fn clone(&self) -> Parameters {
                {
                    let _: ::core::clone::AssertParamIsClone<u32>;
                    let _: ::core::clone::AssertParamIsClone<Width>;
                    let _: ::core::clone::AssertParamIsClone<Parity>;
                    let _: ::core::clone::AssertParamIsClone<StopBits>;
                    let _: ::core::clone::AssertParamIsClone<bool>;
                    *self
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Parameters {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    Parameters {
                    baud_rate: ref __self_0_0,
                    width: ref __self_0_1,
                    parity: ref __self_0_2,
                    stop_bits: ref __self_0_3,
                    hw_flow_control: ref __self_0_4 } => {
                        let mut debug_trait_builder =
                            f.debug_struct("Parameters");
                        let _ =
                            debug_trait_builder.field("baud_rate",
                                                      &&(*__self_0_0));
                        let _ =
                            debug_trait_builder.field("width",
                                                      &&(*__self_0_1));
                        let _ =
                            debug_trait_builder.field("parity",
                                                      &&(*__self_0_2));
                        let _ =
                            debug_trait_builder.field("stop_bits",
                                                      &&(*__self_0_3));
                        let _ =
                            debug_trait_builder.field("hw_flow_control",
                                                      &&(*__self_0_4));
                        debug_trait_builder.finish()
                    }
                }
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
            fn clone(&self) -> Error { { *self } }
        }
                #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for Error {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&Error::None,) => {
                        let mut debug_trait_builder = f.debug_tuple("None");
                        debug_trait_builder.finish()
                    }
                    (&Error::ParityError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ParityError");
                        debug_trait_builder.finish()
                    }
                    (&Error::FramingError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("FramingError");
                        debug_trait_builder.finish()
                    }
                    (&Error::OverrunError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("OverrunError");
                        debug_trait_builder.finish()
                    }
                    (&Error::RepeatCallError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("RepeatCallError");
                        debug_trait_builder.finish()
                    }
                    (&Error::ResetError,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ResetError");
                        debug_trait_builder.finish()
                    }
                    (&Error::Aborted,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Aborted");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
                #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                {
                    let __self_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*self)
                        } as isize;
                    let __arg_1_vi =
                        unsafe {
                            ::core::intrinsics::discriminant_value(&*other)
                        } as isize;
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
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
            fn transmitted_word(&self, _rval: ReturnCode) { }
            fn transmitted_buffer(&self, tx_buffer: &'static mut [u8],
                                  tx_len: usize, rval: ReturnCode);
        }
        pub trait ReceiveClient {
            fn received_word(&self, _word: u32, _rval: ReturnCode,
                             _error: Error) {
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
                match (&*self,) {
                    (&CtrlSetupResult::Ok,) => {
                        let mut debug_trait_builder = f.debug_tuple("Ok");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::OkSetAddress,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("OkSetAddress");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrBadLength,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrBadLength");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrNoParse,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrNoParse");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrNonstandardRequest,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrNonstandardRequest");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrUnrecognizedDescriptorType,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrUnrecognizedDescriptorType");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrUnrecognizedRequestType,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrUnrecognizedRequestType");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrNoDeviceQualifier,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrNoDeviceQualifier");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrInvalidDeviceIndex,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrInvalidDeviceIndex");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrInvalidConfigurationIndex,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrInvalidConfigurationIndex");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrInvalidInterfaceIndex,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrInvalidInterfaceIndex");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrInvalidStringIndex,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrInvalidStringIndex");
                        debug_trait_builder.finish()
                    }
                    (&CtrlSetupResult::ErrGeneric,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("ErrGeneric");
                        debug_trait_builder.finish()
                    }
                }
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
        pub fn new(kernel: &'static Kernel) -> KernelInfo {
            KernelInfo{kernel: kernel,}
        }
        pub fn number_loaded_processes(&self,
                                       _capability:
                                           &dyn ProcessManagementCapability)
         -> usize {
            let count: Cell<usize> = Cell::new(0);
            self.kernel.process_each(|_| count.increment());
            count.get()
        }
        pub fn number_active_processes(&self,
                                       _capability:
                                           &dyn ProcessManagementCapability)
         -> usize {
            let count: Cell<usize> = Cell::new(0);
            self.kernel.process_each(|process|
                                         match process.get_state() {
                                             process::State::Running =>
                                             count.increment(),
                                             process::State::Yielded =>
                                             count.increment(),
                                             _ => { }
                                         });
            count.get()
        }
        pub fn number_inactive_processes(&self,
                                         _capability:
                                             &dyn ProcessManagementCapability)
         -> usize {
            let count: Cell<usize> = Cell::new(0);
            self.kernel.process_each(|process|
                                         match process.get_state() {
                                             process::State::Running => { }
                                             process::State::Yielded => { }
                                             _ => count.increment(),
                                         });
            count.get()
        }
        pub fn process_name(&self, app: AppId,
                            _capability: &dyn ProcessManagementCapability)
         -> &'static str {
            self.kernel.process_map_or("unknown", app.idx(),
                                       |process| process.get_process_name())
        }
        pub fn number_app_syscalls(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           process.debug_syscall_count())
        }
        pub fn number_app_dropped_callbacks(&self, app: AppId,
                                            _capability:
                                                &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           {
                                               process.debug_dropped_callback_count()
                                           })
        }
        pub fn number_app_restarts(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           process.debug_restart_count())
        }
        pub fn number_app_timeslice_expirations(&self, app: AppId,
                                                _capability:
                                                    &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           {
                                               process.debug_timeslice_expiration_count()
                                           })
        }
        pub fn timeslice_expirations(&self,
                                     _capability:
                                         &dyn ProcessManagementCapability)
         -> usize {
            let count: Cell<usize> = Cell::new(0);
            self.kernel.process_each(|proc|
                                         {
                                             count.add(proc.debug_timeslice_expiration_count());
                                         });
            count.get()
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
        fn default() -> IPCData {
            IPCData{shared_memory:
                        [None, None, None, None, None, None, None, None],
                    client_callbacks:
                        [None, None, None, None, None, None, None, None],
                    callback: None,}
        }
    }
    pub struct IPC {
        data: Grant<IPCData>,
    }
    impl IPC {
        pub fn new(kernel: &'static Kernel,
                   capability: &dyn MemoryAllocationCapability) -> IPC {
            IPC{data: kernel.create_grant(capability),}
        }
        pub unsafe fn schedule_callback(&self, appid: AppId, otherapp: AppId,
                                        cb_type: process::IPCType) {
            self.data.enter(appid,
                            |mydata, _|
                                {
                                    let callback =
                                        match cb_type {
                                            process::IPCType::Service =>
                                            mydata.callback,
                                            process::IPCType::Client => {
                                                *mydata.client_callbacks.get(otherapp.idx()).unwrap_or(&None)
                                            }
                                        };
                                    callback.map_or((),
                                                    |mut callback|
                                                        {
                                                            self.data.enter(otherapp,
                                                                            |otherdata,
                                                                             _|
                                                                                {
                                                                                    if appid.idx()
                                                                                           >=
                                                                                           otherdata.shared_memory.len()
                                                                                       {
                                                                                        return;
                                                                                    }
                                                                                    match otherdata.shared_memory[appid.idx()]
                                                                                        {
                                                                                        Some(ref slice)
                                                                                        =>
                                                                                        {
                                                                                            slice.expose_to(appid);
                                                                                            callback.schedule(otherapp.idx()
                                                                                                                  +
                                                                                                                  1,
                                                                                                              slice.len(),
                                                                                                              slice.ptr()
                                                                                                                  as
                                                                                                                  usize);
                                                                                        }
                                                                                        None
                                                                                        =>
                                                                                        {
                                                                                            callback.schedule(otherapp.idx()
                                                                                                                  +
                                                                                                                  1,
                                                                                                              0,
                                                                                                              0);
                                                                                        }
                                                                                    }
                                                                                }).unwrap_or(());
                                                        });
                                }).unwrap_or(());
        }
    }
    impl Driver for IPC {
        fn subscribe(&self, subscribe_num: usize, callback: Option<Callback>,
                     app_id: AppId) -> ReturnCode {
            match subscribe_num {
                0 =>
                self.data.enter(app_id,
                                |data, _|
                                    {
                                        data.callback = callback;
                                        ReturnCode::SUCCESS
                                    }).unwrap_or(ReturnCode::EBUSY),
                svc_id => {
                    if svc_id - 1 >= 8 {
                        ReturnCode::EINVAL
                    } else {
                        self.data.enter(app_id,
                                        |data, _|
                                            {
                                                data.client_callbacks[svc_id -
                                                                          1] =
                                                    callback;
                                                ReturnCode::SUCCESS
                                            }).unwrap_or(ReturnCode::EBUSY)
                    }
                }
            }
        }
        fn command(&self, target_id: usize, client_or_svc: usize, _: usize,
                   appid: AppId) -> ReturnCode {
            let cb_type =
                if client_or_svc == 0 {
                    process::IPCType::Service
                } else { process::IPCType::Client };
            self.data.kernel.process_map_or(ReturnCode::EINVAL, target_id - 1,
                                            |target|
                                                {
                                                    let ret =
                                                        target.enqueue_task(process::Task::IPC((appid,
                                                                                                cb_type)));
                                                    match ret {
                                                        true =>
                                                        ReturnCode::SUCCESS,
                                                        false =>
                                                        ReturnCode::FAIL,
                                                    }
                                                })
        }
        fn allow(&self, appid: AppId, target_id: usize,
                 slice: Option<AppSlice<Shared, u8>>) -> ReturnCode {
            if target_id == 0 {
                match slice {
                    Some(slice_data) => {
                        let ret =
                            self.data.kernel.process_until(|p|
                                                               {
                                                                   let s =
                                                                       p.get_process_name().as_bytes();
                                                                   if s.len()
                                                                          ==
                                                                          slice_data.len()
                                                                          &&
                                                                          s.iter().zip(slice_data.iter()).all(|(c1,
                                                                                                                c2)|
                                                                                                                  c1
                                                                                                                      ==
                                                                                                                      c2)
                                                                      {
                                                                       ReturnCode::SuccessWithValue{value:
                                                                                                        (p.appid().idx()
                                                                                                             as
                                                                                                             usize)
                                                                                                            +
                                                                                                            1,}
                                                                   } else {
                                                                       ReturnCode::FAIL
                                                                   }
                                                               });
                        if ret != ReturnCode::FAIL { return ret; }
                    }
                    None => { }
                }
                return ReturnCode::EINVAL;
            }
            self.data.enter(appid,
                            |data, _|
                                {
                                    data.shared_memory.get_mut(target_id -
                                                                   1).map_or(ReturnCode::EINVAL,
                                                                             |smem|
                                                                                 {
                                                                                     *smem
                                                                                         =
                                                                                         slice;
                                                                                     ReturnCode::SUCCESS
                                                                                 })
                                }).unwrap_or(ReturnCode::EBUSY)
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
        fn clone(&self) -> Syscall {
            {
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<*mut ()>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<*mut u8>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Syscall {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Syscall::YIELD,) => {
                    let mut debug_trait_builder = f.debug_tuple("YIELD");
                    debug_trait_builder.finish()
                }
                (&Syscall::SUBSCRIBE {
                 driver_number: ref __self_0,
                 subdriver_number: ref __self_1,
                 callback_ptr: ref __self_2,
                 appdata: ref __self_3 },) => {
                    let mut debug_trait_builder = f.debug_struct("SUBSCRIBE");
                    let _ =
                        debug_trait_builder.field("driver_number",
                                                  &&(*__self_0));
                    let _ =
                        debug_trait_builder.field("subdriver_number",
                                                  &&(*__self_1));
                    let _ =
                        debug_trait_builder.field("callback_ptr",
                                                  &&(*__self_2));
                    let _ =
                        debug_trait_builder.field("appdata", &&(*__self_3));
                    debug_trait_builder.finish()
                }
                (&Syscall::COMMAND {
                 driver_number: ref __self_0,
                 subdriver_number: ref __self_1,
                 arg0: ref __self_2,
                 arg1: ref __self_3 },) => {
                    let mut debug_trait_builder = f.debug_struct("COMMAND");
                    let _ =
                        debug_trait_builder.field("driver_number",
                                                  &&(*__self_0));
                    let _ =
                        debug_trait_builder.field("subdriver_number",
                                                  &&(*__self_1));
                    let _ = debug_trait_builder.field("arg0", &&(*__self_2));
                    let _ = debug_trait_builder.field("arg1", &&(*__self_3));
                    debug_trait_builder.finish()
                }
                (&Syscall::ALLOW {
                 driver_number: ref __self_0,
                 subdriver_number: ref __self_1,
                 allow_address: ref __self_2,
                 allow_size: ref __self_3 },) => {
                    let mut debug_trait_builder = f.debug_struct("ALLOW");
                    let _ =
                        debug_trait_builder.field("driver_number",
                                                  &&(*__self_0));
                    let _ =
                        debug_trait_builder.field("subdriver_number",
                                                  &&(*__self_1));
                    let _ =
                        debug_trait_builder.field("allow_address",
                                                  &&(*__self_2));
                    let _ =
                        debug_trait_builder.field("allow_size",
                                                  &&(*__self_3));
                    debug_trait_builder.finish()
                }
                (&Syscall::MEMOP { operand: ref __self_0, arg0: ref __self_1
                 },) => {
                    let mut debug_trait_builder = f.debug_struct("MEMOP");
                    let _ =
                        debug_trait_builder.field("operand", &&(*__self_0));
                    let _ = debug_trait_builder.field("arg0", &&(*__self_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Syscall {
        #[inline]
        fn eq(&self, other: &Syscall) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&Syscall::SUBSCRIBE {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         callback_ptr: ref __self_2,
                         appdata: ref __self_3 }, &Syscall::SUBSCRIBE {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         callback_ptr: ref __arg_1_2,
                         appdata: ref __arg_1_3 }) =>
                        (*__self_0) == (*__arg_1_0) &&
                            (*__self_1) == (*__arg_1_1) &&
                            (*__self_2) == (*__arg_1_2) &&
                            (*__self_3) == (*__arg_1_3),
                        (&Syscall::COMMAND {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         arg0: ref __self_2,
                         arg1: ref __self_3 }, &Syscall::COMMAND {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         arg0: ref __arg_1_2,
                         arg1: ref __arg_1_3 }) =>
                        (*__self_0) == (*__arg_1_0) &&
                            (*__self_1) == (*__arg_1_1) &&
                            (*__self_2) == (*__arg_1_2) &&
                            (*__self_3) == (*__arg_1_3),
                        (&Syscall::ALLOW {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         allow_address: ref __self_2,
                         allow_size: ref __self_3 }, &Syscall::ALLOW {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         allow_address: ref __arg_1_2,
                         allow_size: ref __arg_1_3 }) =>
                        (*__self_0) == (*__arg_1_0) &&
                            (*__self_1) == (*__arg_1_1) &&
                            (*__self_2) == (*__arg_1_2) &&
                            (*__self_3) == (*__arg_1_3),
                        (&Syscall::MEMOP {
                         operand: ref __self_0, arg0: ref __self_1 },
                         &Syscall::MEMOP {
                         operand: ref __arg_1_0, arg0: ref __arg_1_1 }) =>
                        (*__self_0) == (*__arg_1_0) &&
                            (*__self_1) == (*__arg_1_1),
                        _ => true,
                    }
                } else { false }
            }
        }
        #[inline]
        fn ne(&self, other: &Syscall) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&Syscall::SUBSCRIBE {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         callback_ptr: ref __self_2,
                         appdata: ref __self_3 }, &Syscall::SUBSCRIBE {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         callback_ptr: ref __arg_1_2,
                         appdata: ref __arg_1_3 }) =>
                        (*__self_0) != (*__arg_1_0) ||
                            (*__self_1) != (*__arg_1_1) ||
                            (*__self_2) != (*__arg_1_2) ||
                            (*__self_3) != (*__arg_1_3),
                        (&Syscall::COMMAND {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         arg0: ref __self_2,
                         arg1: ref __self_3 }, &Syscall::COMMAND {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         arg0: ref __arg_1_2,
                         arg1: ref __arg_1_3 }) =>
                        (*__self_0) != (*__arg_1_0) ||
                            (*__self_1) != (*__arg_1_1) ||
                            (*__self_2) != (*__arg_1_2) ||
                            (*__self_3) != (*__arg_1_3),
                        (&Syscall::ALLOW {
                         driver_number: ref __self_0,
                         subdriver_number: ref __self_1,
                         allow_address: ref __self_2,
                         allow_size: ref __self_3 }, &Syscall::ALLOW {
                         driver_number: ref __arg_1_0,
                         subdriver_number: ref __arg_1_1,
                         allow_address: ref __arg_1_2,
                         allow_size: ref __arg_1_3 }) =>
                        (*__self_0) != (*__arg_1_0) ||
                            (*__self_1) != (*__arg_1_1) ||
                            (*__self_2) != (*__arg_1_2) ||
                            (*__self_3) != (*__arg_1_3),
                        (&Syscall::MEMOP {
                         operand: ref __self_0, arg0: ref __self_1 },
                         &Syscall::MEMOP {
                         operand: ref __arg_1_0, arg0: ref __arg_1_1 }) =>
                        (*__self_0) != (*__arg_1_0) ||
                            (*__self_1) != (*__arg_1_1),
                        _ => false,
                    }
                } else { true }
            }
        }
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
        fn eq(&self, other: &ContextSwitchReason) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ContextSwitchReason::SyscallFired {
                         syscall: ref __self_0 },
                         &ContextSwitchReason::SyscallFired {
                         syscall: ref __arg_1_0 }) =>
                        (*__self_0) == (*__arg_1_0),
                        _ => true,
                    }
                } else { false }
            }
        }
        #[inline]
        fn ne(&self, other: &ContextSwitchReason) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ContextSwitchReason::SyscallFired {
                         syscall: ref __self_0 },
                         &ContextSwitchReason::SyscallFired {
                         syscall: ref __arg_1_0 }) =>
                        (*__self_0) != (*__arg_1_0),
                        _ => false,
                    }
                } else { true }
            }
        }
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
        match syscall_number {
            0 => Some(Syscall::YIELD),
            1 =>
            Some(Syscall::SUBSCRIBE{driver_number: r0,
                                    subdriver_number: r1,
                                    callback_ptr: r2 as *mut (),
                                    appdata: r3,}),
            2 =>
            Some(Syscall::COMMAND{driver_number: r0,
                                  subdriver_number: r1,
                                  arg0: r2,
                                  arg1: r3,}),
            3 =>
            Some(Syscall::ALLOW{driver_number: r0,
                                subdriver_number: r1,
                                allow_address: r2 as *mut u8,
                                allow_size: r3,}),
            4 => Some(Syscall::MEMOP{operand: r0, arg0: r1,}),
            _ => None,
        }
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
        fn clone(&self) -> AppId {
            {
                let _: ::core::clone::AssertParamIsClone<&'static Kernel>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for AppId { }
    impl PartialEq for AppId {
        fn eq(&self, other: &AppId) -> bool { self.idx == other.idx }
    }
    impl Eq for AppId { }
    impl fmt::Debug for AppId {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(::core::fmt::Arguments::new_v1(&[""],
                                                       &match (&self.idx,) {
                                                            (arg0,) =>
                                                            [::core::fmt::ArgumentV1::new(arg0,
                                                                                          ::core::fmt::Display::fmt)],
                                                        }))
        }
    }
    impl AppId {
        crate fn new(kernel: &'static Kernel, idx: usize) -> AppId {
            AppId{kernel: kernel, idx: idx,}
        }
        pub fn idx(&self) -> usize { self.idx }
        pub fn get_editable_flash_range(&self) -> (usize, usize) {
            self.kernel.process_map_or((0, 0), self.idx,
                                       |process|
                                           {
                                               let start =
                                                   process.flash_non_protected_start()
                                                       as usize;
                                               let end =
                                                   process.flash_end() as
                                                       usize;
                                               (start, end)
                                           })
        }
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
        fn clone(&self) -> CallbackId {
            {
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for CallbackId {
        #[inline]
        fn eq(&self, other: &CallbackId) -> bool {
            match *other {
                CallbackId {
                driver_num: ref __self_1_0, subscribe_num: ref __self_1_1 } =>
                match *self {
                    CallbackId {
                    driver_num: ref __self_0_0, subscribe_num: ref __self_0_1
                    } =>
                    (*__self_0_0) == (*__self_1_0) &&
                        (*__self_0_1) == (*__self_1_1),
                },
            }
        }
        #[inline]
        fn ne(&self, other: &CallbackId) -> bool {
            match *other {
                CallbackId {
                driver_num: ref __self_1_0, subscribe_num: ref __self_1_1 } =>
                match *self {
                    CallbackId {
                    driver_num: ref __self_0_0, subscribe_num: ref __self_0_1
                    } =>
                    (*__self_0_0) != (*__self_1_0) ||
                        (*__self_0_1) != (*__self_1_1),
                },
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for CallbackId {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                CallbackId {
                driver_num: ref __self_0_0, subscribe_num: ref __self_0_1 } =>
                {
                    let mut debug_trait_builder =
                        f.debug_struct("CallbackId");
                    let _ =
                        debug_trait_builder.field("driver_num",
                                                  &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("subscribe_num",
                                                  &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> Callback {
            {
                let _: ::core::clone::AssertParamIsClone<AppId>;
                let _: ::core::clone::AssertParamIsClone<CallbackId>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<NonNull<*mut ()>>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Callback { }
    impl Callback {
        crate fn new(app_id: AppId, callback_id: CallbackId, appdata: usize,
                     fn_ptr: NonNull<*mut ()>) -> Callback {
            Callback{app_id, callback_id, appdata, fn_ptr,}
        }
        pub fn schedule(&mut self, r0: usize, r1: usize, r2: usize) -> bool {
            self.app_id.kernel.process_map_or(false, self.app_id.idx(),
                                              |process|
                                                  {
                                                      process.enqueue_task(process::Task::FunctionCall(process::FunctionCall{source:
                                                                                                                                 process::FunctionCallSource::Driver(self.callback_id),
                                                                                                                             argument0:
                                                                                                                                 r0,
                                                                                                                             argument1:
                                                                                                                                 r1,
                                                                                                                             argument2:
                                                                                                                                 r2,
                                                                                                                             argument3:
                                                                                                                                 self.appdata,
                                                                                                                             pc:
                                                                                                                                 self.fn_ptr.as_ptr()
                                                                                                                                     as
                                                                                                                                     usize,}))
                                                  })
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
            ReturnCode::ENOSUPPORT
        }
        #[allow(unused_variables)]
        fn command(&self, minor_num: usize, r2: usize, r3: usize,
                   caller_id: AppId) -> ReturnCode {
            ReturnCode::ENOSUPPORT
        }
        #[allow(unused_variables)]
        fn allow(&self, app: AppId, minor_num: usize,
                 slice: Option<AppSlice<Shared, u8>>) -> ReturnCode {
            ReturnCode::ENOSUPPORT
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
            let mut allocator = Allocator{appid: self.appid,};
            let mut root = unsafe { Owned::new(self.grant, self.appid) };
            fun(&mut root, &mut allocator)
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
        unsafe fn new(data: *mut T, appid: AppId) -> Owned<T> {
            Owned{data: Unique::new_unchecked(data), appid: appid,}
        }
        pub fn appid(&self) -> AppId { self.appid }
    }
    impl <T: ?Sized> Drop for Owned<T> {
        fn drop(&mut self) {
            unsafe {
                let data = self.data.as_ptr() as *mut u8;
                self.appid.kernel.process_map_or((), self.appid.idx(),
                                                 |process|
                                                     { process.free(data); });
            }
        }
    }
    impl <T: ?Sized> Deref for Owned<T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { unsafe { self.data.as_ref() } }
    }
    impl <T: ?Sized> DerefMut for Owned<T> {
        fn deref_mut(&mut self) -> &mut T { unsafe { self.data.as_mut() } }
    }
    impl Allocator {
        pub fn alloc<T>(&mut self, data: T) -> Result<Owned<T>, Error> {
            unsafe {
                self.appid.kernel.process_map_or(Err(Error::NoSuchApp),
                                                 self.appid.idx(),
                                                 |process|
                                                     {
                                                         process.alloc(size_of::<T>(),
                                                                       align_of::<T>()).map_or(Err(Error::OutOfMemory),
                                                                                               |arr|
                                                                                                   {
                                                                                                       let ptr =
                                                                                                           arr.as_mut_ptr()
                                                                                                               as
                                                                                                               *mut T;
                                                                                                       write(ptr,
                                                                                                             data);
                                                                                                       Ok(Owned::new(ptr,
                                                                                                                     self.appid))
                                                                                                   })
                                                     })
            }
        }
    }
    pub struct Borrowed<'a, T: 'a + ?Sized> {
        data: &'a mut T,
        appid: AppId,
    }
    impl <T: 'a + ?Sized> Borrowed<'a, T> {
        pub fn new(data: &'a mut T, appid: AppId) -> Borrowed<'a, T> {
            Borrowed{data: data, appid: appid,}
        }
        pub fn appid(&self) -> AppId { self.appid }
    }
    impl <T: 'a + ?Sized> Deref for Borrowed<'a, T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { self.data }
    }
    impl <T: 'a + ?Sized> DerefMut for Borrowed<'a, T> {
        fn deref_mut(&mut self) -> &mut T { self.data }
    }
    impl <T: Default> Grant<T> {
        crate fn new(kernel: &'static Kernel, grant_index: usize)
         -> Grant<T> {
            Grant{kernel: kernel, grant_num: grant_index, ptr: PhantomData,}
        }
        pub fn grant(&self, appid: AppId) -> Option<AppliedGrant<T>> {
            unsafe {
                appid.kernel.process_map_or(None, appid.idx(),
                                            |process|
                                                {
                                                    let cntr =
                                                        *(process.grant_ptr(self.grant_num)
                                                              as *mut *mut T);
                                                    if cntr.is_null() {
                                                        None
                                                    } else {
                                                        Some(AppliedGrant{appid:
                                                                              appid,
                                                                          grant:
                                                                              cntr,
                                                                          _phantom:
                                                                              PhantomData,})
                                                    }
                                                })
            }
        }
        pub fn enter<F, R>(&self, appid: AppId, fun: F) -> Result<R, Error>
         where F: FnOnce(&mut Borrowed<T>, &mut Allocator) -> R, R: Copy {
            unsafe {
                appid.kernel.process_map_or(Err(Error::NoSuchApp),
                                            appid.idx(),
                                            |process|
                                                {
                                                    let ctr_ptr =
                                                        process.grant_ptr(self.grant_num)
                                                            as *mut *mut T;
                                                    let new_grant =
                                                        if (*ctr_ptr).is_null()
                                                           {
                                                            process.alloc(size_of::<T>(),
                                                                          align_of::<T>()).map(|root_arr|
                                                                                                   {
                                                                                                       let root_ptr =
                                                                                                           root_arr.as_mut_ptr()
                                                                                                               as
                                                                                                               *mut T;
                                                                                                       write(root_ptr,
                                                                                                             Default::default());
                                                                                                       write_volatile(ctr_ptr,
                                                                                                                      root_ptr);
                                                                                                       root_ptr
                                                                                                   })
                                                        } else {
                                                            Some(*ctr_ptr)
                                                        };
                                                    new_grant.map_or(Err(Error::OutOfMemory),
                                                                     move
                                                                         |root_ptr|
                                                                         {
                                                                             let root_ptr =
                                                                                 root_ptr
                                                                                     as
                                                                                     *mut T;
                                                                             let mut root =
                                                                                 Borrowed::new(&mut *root_ptr,
                                                                                               appid);
                                                                             let mut allocator =
                                                                                 Allocator{appid:
                                                                                               appid,};
                                                                             let res =
                                                                                 fun(&mut root,
                                                                                     &mut allocator);
                                                                             Ok(res)
                                                                         })
                                                })
            }
        }
        pub fn each<F>(&self, fun: F) where F: Fn(&mut Owned<T>) {
            self.kernel.process_each(|process|
                                         unsafe {
                                             let root_ptr =
                                                 *(process.grant_ptr(self.grant_num)
                                                       as *mut *mut T);
                                             if !root_ptr.is_null() {
                                                 let mut root =
                                                     Owned::new(root_ptr,
                                                                process.appid());
                                                 fun(&mut root);
                                             }
                                         });
        }
        pub fn iter(&self) -> Iter<T> {
            Iter{grant: self,
                 index: 0,
                 len: self.kernel.number_of_process_slots(),}
        }
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
        fn next(&mut self) -> Option<Self::Item> {
            while self.index < self.len {
                let idx = self.index;
                self.index += 1;
                let res =
                    self.grant.grant(AppId::new(self.grant.kernel, idx));
                if res.is_some() { return res; }
            }
            None
        }
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
            match *self {
                Private => {
                    let mut debug_trait_builder = f.debug_tuple("Private");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    pub struct Shared;
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Shared {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Shared => {
                    let mut debug_trait_builder = f.debug_tuple("Shared");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    pub struct AppPtr<L, T> {
        ptr: Unique<T>,
        process: AppId,
        _phantom: PhantomData<L>,
    }
    impl <L, T> AppPtr<L, T> {
        unsafe fn new(ptr: *mut T, appid: AppId) -> AppPtr<L, T> {
            AppPtr{ptr: Unique::new_unchecked(ptr),
                   process: appid,
                   _phantom: PhantomData,}
        }
    }
    impl <L, T> Deref for AppPtr<L, T> {
        type
        Target
        =
        T;
        fn deref(&self) -> &T { unsafe { self.ptr.as_ref() } }
    }
    impl <L, T> DerefMut for AppPtr<L, T> {
        fn deref_mut(&mut self) -> &mut T { unsafe { self.ptr.as_mut() } }
    }
    impl <L, T> Drop for AppPtr<L, T> {
        fn drop(&mut self) {
            self.process.kernel.process_map_or((), self.process.idx(),
                                               |process|
                                                   unsafe {
                                                       process.free(self.ptr.as_ptr()
                                                                        as
                                                                        *mut u8)
                                                   })
        }
    }
    pub struct AppSlice<L, T> {
        ptr: AppPtr<L, T>,
        len: usize,
    }
    impl <L, T> AppSlice<L, T> {
        crate fn new(ptr: *mut T, len: usize, appid: AppId)
         -> AppSlice<L, T> {
            unsafe { AppSlice{ptr: AppPtr::new(ptr, appid), len: len,} }
        }
        pub fn len(&self) -> usize { self.len }
        pub fn ptr(&self) -> *const T { self.ptr.ptr.as_ptr() }
        crate unsafe fn expose_to(&self, appid: AppId) -> bool {
            if appid.idx() != self.ptr.process.idx() {
                self.ptr.process.kernel.process_map_or(false, appid.idx(),
                                                       |process|
                                                           {
                                                               process.add_mpu_region(self.ptr()
                                                                                          as
                                                                                          *const u8,
                                                                                      self.len(),
                                                                                      self.len()).is_some()
                                                           })
            } else { false }
        }
        pub fn iter(&self) -> slice::Iter<T> { self.as_ref().iter() }
        pub fn iter_mut(&mut self) -> slice::IterMut<T> {
            self.as_mut().iter_mut()
        }
        pub fn chunks(&self, size: usize) -> slice::Chunks<T> {
            self.as_ref().chunks(size)
        }
        pub fn chunks_mut(&mut self, size: usize) -> slice::ChunksMut<T> {
            self.as_mut().chunks_mut(size)
        }
    }
    impl <L, T> AsRef<[T]> for AppSlice<L, T> {
        fn as_ref(&self) -> &[T] {
            unsafe { slice::from_raw_parts(self.ptr.ptr.as_ref(), self.len) }
        }
    }
    impl <L, T> AsMut<[T]> for AppSlice<L, T> {
        fn as_mut(&mut self) -> &mut [T] {
            unsafe {
                slice::from_raw_parts_mut(self.ptr.ptr.as_mut(), self.len)
            }
        }
    }
}
mod memop {
    use crate::process::ProcessType;
    use crate::returncode::ReturnCode;
    crate fn memop(process: &dyn ProcessType, op_type: usize, r1: usize)
     -> ReturnCode {
        match op_type {
            0 => {
                process.brk(r1 as
                                *const u8).map(|_|
                                                   ReturnCode::SUCCESS).unwrap_or(ReturnCode::ENOMEM)
            }
            1 => {
                process.sbrk(r1 as
                                 isize).map(|addr|
                                                ReturnCode::SuccessWithValue{value:
                                                                                 addr
                                                                                     as
                                                                                     usize,}).unwrap_or(ReturnCode::ENOMEM)
            }
            2 =>
            ReturnCode::SuccessWithValue{value:
                                             process.mem_start() as usize,},
            3 =>
            ReturnCode::SuccessWithValue{value: process.mem_end() as usize,},
            4 =>
            ReturnCode::SuccessWithValue{value:
                                             process.flash_start() as usize,},
            5 =>
            ReturnCode::SuccessWithValue{value:
                                             process.flash_end() as usize,},
            6 =>
            ReturnCode::SuccessWithValue{value:
                                             process.kernel_memory_break() as
                                                 usize,},
            7 =>
            ReturnCode::SuccessWithValue{value:
                                             process.number_writeable_flash_regions(),},
            8 => {
                let flash_start = process.flash_start() as usize;
                let (offset, size) = process.get_writeable_flash_region(r1);
                if size == 0 {
                    ReturnCode::FAIL
                } else {
                    ReturnCode::SuccessWithValue{value:
                                                     flash_start +
                                                         offset as usize,}
                }
            }
            9 => {
                let flash_start = process.flash_start() as usize;
                let (offset, size) = process.get_writeable_flash_region(r1);
                if size == 0 {
                    ReturnCode::FAIL
                } else {
                    ReturnCode::SuccessWithValue{value:
                                                     flash_start +
                                                         offset as usize +
                                                         size as usize,}
                }
            }
            10 => {
                process.update_stack_start_pointer(r1 as *const u8);
                ReturnCode::SUCCESS
            }
            11 => {
                process.update_heap_start_pointer(r1 as *const u8);
                ReturnCode::SUCCESS
            }
            _ => ReturnCode::ENOSUPPORT,
        }
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
            fn clone(&self) -> Permissions { { *self } }
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
            fn clone(&self) -> Region {
                {
                    let _: ::core::clone::AssertParamIsClone<*const u8>;
                    let _: ::core::clone::AssertParamIsClone<usize>;
                    *self
                }
            }
        }
        impl Region {
            pub fn new(start_address: *const u8, size: usize) -> Region {
                Region{start_address: start_address, size: size,}
            }
            pub fn start_address(&self) -> *const u8 { self.start_address }
            pub fn size(&self) -> usize { self.size }
        }
        pub struct MpuConfig;
                #[allow(unused_qualifications)]
        impl ::core::default::Default for MpuConfig {
            #[inline]
            fn default() -> MpuConfig { MpuConfig{} }
        }
        impl Display for MpuConfig {
            fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
                Ok(())
            }
        }
        pub trait MPU {
            type
            MpuConfig: Default +
            Display
            =
            ();
            fn enable_mpu(&self) { }
            fn disable_mpu(&self) { }
            fn number_total_regions(&self) -> usize { 0 }
            #[allow(unused_variables)]
            fn allocate_region(&self, unallocated_memory_start: *const u8,
                               unallocated_memory_size: usize,
                               min_region_size: usize,
                               permissions: Permissions,
                               config: &mut Self::MpuConfig)
             -> Option<Region> {
                if min_region_size > unallocated_memory_size {
                    None
                } else {
                    Some(Region::new(unallocated_memory_start,
                                     min_region_size))
                }
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
                let memory_size =
                    cmp::max(min_memory_size,
                             initial_app_memory_size +
                                 initial_kernel_memory_size);
                if memory_size > unallocated_memory_size {
                    None
                } else { Some((unallocated_memory_start, memory_size)) }
            }
            #[allow(unused_variables)]
            fn update_app_memory_region(&self, app_memory_break: *const u8,
                                        kernel_memory_break: *const u8,
                                        permissions: Permissions,
                                        config: &mut Self::MpuConfig)
             -> Result<(), ()> {
                if (app_memory_break as usize) >
                       (kernel_memory_break as usize) {
                    Err(())
                } else { Ok(()) }
            }
            #[allow(unused_variables)]
            fn configure_mpu(&self, config: &Self::MpuConfig) { }
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
            fn reset(&self) { }
            fn set_timer(&self, _: u32) { }
            fn enable(&self, _: bool) { }
            fn overflowed(&self) -> bool { false }
            fn greater_than(&self, _: u32) -> bool { true }
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
        fn is_enabled(&self) -> bool { true }
        fn enable(&self) { }
        fn disable(&self) { }
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
    pub fn load_processes<C: Chip>(kernel: &'static Kernel, chip: &'static C,
                                   start_of_flash: *const u8,
                                   app_memory: &mut [u8],
                                   procs:
                                       &'static mut [Option<&'static dyn ProcessType>],
                                   fault_response: FaultResponse,
                                   _capability:
                                       &dyn ProcessManagementCapability) {
        let mut apps_in_flash_ptr = start_of_flash;
        let mut app_memory_ptr = app_memory.as_mut_ptr();
        let mut app_memory_size = app_memory.len();
        for i in 0..procs.len() {
            unsafe {
                let (process, flash_offset, memory_offset) =
                    Process::create(kernel, chip, apps_in_flash_ptr,
                                    app_memory_ptr, app_memory_size,
                                    fault_response, i);
                if process.is_none() {
                    if flash_offset == 0 && memory_offset == 0 { break ; }
                } else { procs[i] = process; }
                apps_in_flash_ptr = apps_in_flash_ptr.add(flash_offset);
                app_memory_ptr = app_memory_ptr.add(memory_offset);
                app_memory_size -= memory_offset;
            }
        }
    }
    pub trait ProcessType {
        fn appid(&self)
        -> AppId;
        fn enqueue_task(&self, task: Task)
        -> bool;
        fn dequeue_task(&self)
        -> Option<Task>;
        fn remove_pending_callbacks(&self, callback_id: CallbackId);
        fn get_state(&self)
        -> State;
        fn set_yielded_state(&self);
        fn stop(&self);
        fn resume(&self);
        fn set_fault_state(&self);
        fn get_process_name(&self)
        -> &'static str;
        fn brk(&self, new_break: *const u8)
        -> Result<*const u8, Error>;
        fn sbrk(&self, increment: isize)
        -> Result<*const u8, Error>;
        fn mem_start(&self)
        -> *const u8;
        fn mem_end(&self)
        -> *const u8;
        fn flash_start(&self)
        -> *const u8;
        fn flash_end(&self)
        -> *const u8;
        fn kernel_memory_break(&self)
        -> *const u8;
        fn number_writeable_flash_regions(&self)
        -> usize;
        fn get_writeable_flash_region(&self, region_index: usize)
        -> (u32, u32);
        fn update_stack_start_pointer(&self, stack_pointer: *const u8);
        fn update_heap_start_pointer(&self, heap_pointer: *const u8);
        fn allow(&self, buf_start_addr: *const u8, size: usize)
        -> Result<Option<AppSlice<Shared, u8>>, ReturnCode>;
        fn flash_non_protected_start(&self)
        -> *const u8;
        fn setup_mpu(&self);
        fn add_mpu_region(&self, unallocated_memory_start: *const u8,
                          unallocated_memory_size: usize,
                          min_region_size: usize)
        -> Option<mpu::Region>;
        unsafe fn alloc(&self, size: usize, align: usize)
        -> Option<&mut [u8]>;
        unsafe fn free(&self, _: *mut u8);
        unsafe fn grant_ptr(&self, grant_num: usize)
        -> *mut *mut u8;
        unsafe fn set_syscall_return_value(&self, return_value: isize);
        unsafe fn set_process_function(&self, callback: FunctionCall);
        unsafe fn switch_to(&self)
        -> Option<syscall::ContextSwitchReason>;
        unsafe fn fault_fmt(&self, writer: &mut dyn Write);
        unsafe fn process_detail_fmt(&self, writer: &mut dyn Write);
        fn debug_syscall_count(&self)
        -> usize;
        fn debug_dropped_callback_count(&self)
        -> usize;
        fn debug_restart_count(&self)
        -> usize;
        fn debug_timeslice_expiration_count(&self)
        -> usize;
        fn debug_timeslice_expired(&self);
    }
    pub enum Error {
        NoSuchApp,
        OutOfMemory,
        AddressOutOfBounds,
        KernelError,
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Error { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Error {
        #[inline]
        fn clone(&self) -> Error { { *self } }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Error {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Error::NoSuchApp,) => {
                    let mut debug_trait_builder = f.debug_tuple("NoSuchApp");
                    debug_trait_builder.finish()
                }
                (&Error::OutOfMemory,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("OutOfMemory");
                    debug_trait_builder.finish()
                }
                (&Error::AddressOutOfBounds,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("AddressOutOfBounds");
                    debug_trait_builder.finish()
                }
                (&Error::KernelError,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("KernelError");
                    debug_trait_builder.finish()
                }
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for Error {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Error {
        #[inline]
        fn eq(&self, other: &Error) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) { _ => true, }
                } else { false }
            }
        }
    }
    impl From<Error> for ReturnCode {
        fn from(err: Error) -> ReturnCode {
            match err {
                Error::OutOfMemory => ReturnCode::ENOMEM,
                Error::AddressOutOfBounds => ReturnCode::EINVAL,
                Error::NoSuchApp => ReturnCode::EINVAL,
                Error::KernelError => ReturnCode::FAIL,
            }
        }
    }
    pub enum State {

        Running,

        Yielded,

        StoppedRunning,

        StoppedYielded,

        StoppedFaulted,

        Fault,

        Unstarted,
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for State { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for State {
        #[inline]
        fn clone(&self) -> State { { *self } }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for State {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&State::Running,) => {
                    let mut debug_trait_builder = f.debug_tuple("Running");
                    debug_trait_builder.finish()
                }
                (&State::Yielded,) => {
                    let mut debug_trait_builder = f.debug_tuple("Yielded");
                    debug_trait_builder.finish()
                }
                (&State::StoppedRunning,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("StoppedRunning");
                    debug_trait_builder.finish()
                }
                (&State::StoppedYielded,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("StoppedYielded");
                    debug_trait_builder.finish()
                }
                (&State::StoppedFaulted,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("StoppedFaulted");
                    debug_trait_builder.finish()
                }
                (&State::Fault,) => {
                    let mut debug_trait_builder = f.debug_tuple("Fault");
                    debug_trait_builder.finish()
                }
                (&State::Unstarted,) => {
                    let mut debug_trait_builder = f.debug_tuple("Unstarted");
                    debug_trait_builder.finish()
                }
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for State {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for State {
        #[inline]
        fn eq(&self, other: &State) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) { _ => true, }
                } else { false }
            }
        }
    }
    pub enum FaultResponse {

        Panic,

        Restart,

        Stop,
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FaultResponse { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FaultResponse {
        #[inline]
        fn clone(&self) -> FaultResponse { { *self } }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for FaultResponse {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&FaultResponse::Panic,) => {
                    let mut debug_trait_builder = f.debug_tuple("Panic");
                    debug_trait_builder.finish()
                }
                (&FaultResponse::Restart,) => {
                    let mut debug_trait_builder = f.debug_tuple("Restart");
                    debug_trait_builder.finish()
                }
                (&FaultResponse::Stop,) => {
                    let mut debug_trait_builder = f.debug_tuple("Stop");
                    debug_trait_builder.finish()
                }
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for FaultResponse {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for FaultResponse {
        #[inline]
        fn eq(&self, other: &FaultResponse) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) { _ => true, }
                } else { false }
            }
        }
    }
    pub enum IPCType { Service, Client, }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for IPCType { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for IPCType {
        #[inline]
        fn clone(&self) -> IPCType { { *self } }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for IPCType {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&IPCType::Service,) => {
                    let mut debug_trait_builder = f.debug_tuple("Service");
                    debug_trait_builder.finish()
                }
                (&IPCType::Client,) => {
                    let mut debug_trait_builder = f.debug_tuple("Client");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    pub enum Task { FunctionCall(FunctionCall), IPC((AppId, IPCType)), }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Task { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Task {
        #[inline]
        fn clone(&self) -> Task {
            {
                let _: ::core::clone::AssertParamIsClone<FunctionCall>;
                let _: ::core::clone::AssertParamIsClone<(AppId, IPCType)>;
                *self
            }
        }
    }
    pub enum FunctionCallSource { Kernel, Driver(CallbackId), }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FunctionCallSource { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FunctionCallSource {
        #[inline]
        fn clone(&self) -> FunctionCallSource {
            { let _: ::core::clone::AssertParamIsClone<CallbackId>; *self }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for FunctionCallSource {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&FunctionCallSource::Kernel,) => {
                    let mut debug_trait_builder = f.debug_tuple("Kernel");
                    debug_trait_builder.finish()
                }
                (&FunctionCallSource::Driver(ref __self_0),) => {
                    let mut debug_trait_builder = f.debug_tuple("Driver");
                    let _ = debug_trait_builder.field(&&(*__self_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    pub struct FunctionCall {
        pub source: FunctionCallSource,
        pub argument0: usize,
        pub argument1: usize,
        pub argument2: usize,
        pub argument3: usize,
        pub pc: usize,
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FunctionCall { }
        #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FunctionCall {
        #[inline]
        fn clone(&self) -> FunctionCall {
            {
                let _: ::core::clone::AssertParamIsClone<FunctionCallSource>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<usize>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for FunctionCall {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                FunctionCall {
                source: ref __self_0_0,
                argument0: ref __self_0_1,
                argument1: ref __self_0_2,
                argument2: ref __self_0_3,
                argument3: ref __self_0_4,
                pc: ref __self_0_5 } => {
                    let mut debug_trait_builder =
                        f.debug_struct("FunctionCall");
                    let _ =
                        debug_trait_builder.field("source", &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("argument0",
                                                  &&(*__self_0_1));
                    let _ =
                        debug_trait_builder.field("argument1",
                                                  &&(*__self_0_2));
                    let _ =
                        debug_trait_builder.field("argument2",
                                                  &&(*__self_0_3));
                    let _ =
                        debug_trait_builder.field("argument3",
                                                  &&(*__self_0_4));
                    let _ = debug_trait_builder.field("pc", &&(*__self_0_5));
                    debug_trait_builder.finish()
                }
            }
        }
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
        stored_state: Cell<<<C as Chip>::UserspaceKernelBoundary as
                           UserspaceKernelBoundary>::StoredState>,
        state: Cell<State>,
        fault_response: FaultResponse,
        mpu_config: MapCell<<<C as Chip>::MPU as MPU>::MpuConfig>,
        mpu_regions: [Cell<Option<mpu::Region>>; 6],
        tasks: MapCell<RingBuffer<'a, Task>>,
        process_name: &'static str,
        debug: MapCell<ProcessDebug>,
    }
    impl <C: Chip> ProcessType for Process<'a, C> {
        fn appid(&self) -> AppId { AppId::new(self.kernel, self.app_idx) }
        fn enqueue_task(&self, task: Task) -> bool {
            if self.state.get() == State::Fault { return false; }
            self.kernel.increment_work();
            let ret = self.tasks.map_or(false, |tasks| tasks.enqueue(task));
            if ret == false {
                self.debug.map(|debug|
                                   { debug.dropped_callback_count += 1; });
            }
            ret
        }
        fn remove_pending_callbacks(&self, callback_id: CallbackId) {
            self.tasks.map(|tasks|
                               {
                                   tasks.retain(|task|
                                                    match task {
                                                        Task::FunctionCall(function_call)
                                                        =>
                                                        match function_call.source
                                                            {
                                                            FunctionCallSource::Kernel
                                                            => true,
                                                            FunctionCallSource::Driver(id)
                                                            =>
                                                            id != callback_id,
                                                        },
                                                        _ => true,
                                                    });
                               });
        }
        fn get_state(&self) -> State { self.state.get() }
        fn set_yielded_state(&self) {
            if self.state.get() == State::Running {
                self.state.set(State::Yielded);
                self.kernel.decrement_work();
            }
        }
        fn stop(&self) {
            match self.state.get() {
                State::Running => self.state.set(State::StoppedRunning),
                State::Yielded => self.state.set(State::StoppedYielded),
                _ => { }
            }
        }
        fn resume(&self) {
            match self.state.get() {
                State::StoppedRunning => self.state.set(State::Running),
                State::StoppedYielded => self.state.set(State::Yielded),
                _ => { }
            }
        }
        fn set_fault_state(&self) {
            self.state.set(State::Fault);
            match self.fault_response {
                FaultResponse::Panic => {
                    {
                        ::core::panicking::panic_fmt(::core::fmt::Arguments::new_v1(&["Process ",
                                                                                      " had a fault"],
                                                                                    &match (&self.process_name,)
                                                                                         {
                                                                                         (arg0,)
                                                                                         =>
                                                                                         [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                       ::core::fmt::Display::fmt)],
                                                                                     }),
                                                     &("/Users/felixklock/Dev/Mozilla/issue65774/demo-minimization/tock/kernel/src/process.rs",
                                                       568u32, 17u32))
                    };
                }
                FaultResponse::Restart => {
                    let tasks_len = self.tasks.map_or(0, |tasks| tasks.len());
                    for _ in 0..tasks_len { self.kernel.decrement_work(); }
                    self.tasks.map(|tasks| { tasks.empty(); });
                    self.debug.map(|debug|
                                       {
                                           debug.restart_count += 1;
                                           debug.syscall_count = 0;
                                           debug.last_syscall = None;
                                           debug.dropped_callback_count = 0;
                                       });
                    let app_flash_address = self.flash_start();
                    let init_fn =
                        unsafe {
                            app_flash_address.offset(self.header.get_init_function_offset()
                                                         as isize) as usize
                        };
                    self.state.set(State::Unstarted);
                    unsafe { self.grant_ptrs_reset(); }
                    self.kernel_memory_break.set(self.original_kernel_memory_break);
                    self.app_break.set(self.original_app_break);
                    self.current_stack_pointer.set(self.original_stack_pointer);
                    let flash_protected_size =
                        self.header.get_protected_size() as usize;
                    let flash_app_start =
                        app_flash_address as usize + flash_protected_size;
                    self.tasks.map(|tasks|
                                       {
                                           tasks.empty();
                                           tasks.enqueue(Task::FunctionCall(FunctionCall{source:
                                                                                             FunctionCallSource::Kernel,
                                                                                         pc:
                                                                                             init_fn,
                                                                                         argument0:
                                                                                             flash_app_start,
                                                                                         argument1:
                                                                                             self.memory.as_ptr()
                                                                                                 as
                                                                                                 usize,
                                                                                         argument2:
                                                                                             self.memory.len()
                                                                                                 as
                                                                                                 usize,
                                                                                         argument3:
                                                                                             self.app_break.get()
                                                                                                 as
                                                                                                 usize,}));
                                       });
                    self.kernel.increment_work();
                }
                FaultResponse::Stop => {
                    let tasks_len = self.tasks.map_or(0, |tasks| tasks.len());
                    for _ in 0..tasks_len { self.kernel.decrement_work(); }
                    self.tasks.map(|tasks| { tasks.empty(); });
                    unsafe { self.grant_ptrs_reset(); }
                    self.state.set(State::StoppedFaulted);
                }
            }
        }
        fn dequeue_task(&self) -> Option<Task> {
            self.tasks.map_or(None,
                              |tasks|
                                  {
                                      tasks.dequeue().map(|cb|
                                                              {
                                                                  self.kernel.decrement_work();
                                                                  cb
                                                              })
                                  })
        }
        fn mem_start(&self) -> *const u8 { self.memory.as_ptr() }
        fn mem_end(&self) -> *const u8 {
            unsafe { self.memory.as_ptr().add(self.memory.len()) }
        }
        fn flash_start(&self) -> *const u8 { self.flash.as_ptr() }
        fn flash_non_protected_start(&self) -> *const u8 {
            ((self.flash.as_ptr() as usize) +
                 self.header.get_protected_size() as usize) as *const u8
        }
        fn flash_end(&self) -> *const u8 {
            unsafe { self.flash.as_ptr().add(self.flash.len()) }
        }
        fn kernel_memory_break(&self) -> *const u8 {
            self.kernel_memory_break.get()
        }
        fn number_writeable_flash_regions(&self) -> usize {
            self.header.number_writeable_flash_regions()
        }
        fn get_writeable_flash_region(&self, region_index: usize)
         -> (u32, u32) {
            self.header.get_writeable_flash_region(region_index)
        }
        fn update_stack_start_pointer(&self, stack_pointer: *const u8) {
            if stack_pointer >= self.mem_start() &&
                   stack_pointer < self.mem_end() {
                self.debug.map(|debug|
                                   {
                                       debug.app_stack_start_pointer =
                                           Some(stack_pointer);
                                       debug.min_stack_pointer =
                                           stack_pointer;
                                   });
            }
        }
        fn update_heap_start_pointer(&self, heap_pointer: *const u8) {
            if heap_pointer >= self.mem_start() &&
                   heap_pointer < self.mem_end() {
                self.debug.map(|debug|
                                   {
                                       debug.app_heap_start_pointer =
                                           Some(heap_pointer);
                                   });
            }
        }
        fn setup_mpu(&self) {
            self.mpu_config.map(|config|
                                    {
                                        self.chip.mpu().configure_mpu(&config);
                                    });
        }
        fn add_mpu_region(&self, unallocated_memory_start: *const u8,
                          unallocated_memory_size: usize,
                          min_region_size: usize) -> Option<mpu::Region> {
            self.mpu_config.and_then(|mut config|
                                         {
                                             let new_region =
                                                 self.chip.mpu().allocate_region(unallocated_memory_start,
                                                                                 unallocated_memory_size,
                                                                                 min_region_size,
                                                                                 mpu::Permissions::ReadWriteOnly,
                                                                                 &mut config);
                                             if new_region.is_none() {
                                                 return None;
                                             }
                                             for region in
                                                 self.mpu_regions.iter() {
                                                 if region.get().is_none() {
                                                     region.set(new_region);
                                                     return new_region;
                                                 }
                                             }
                                             None
                                         })
        }
        fn sbrk(&self, increment: isize) -> Result<*const u8, Error> {
            let new_break = unsafe { self.app_break.get().offset(increment) };
            self.brk(new_break)
        }
        fn brk(&self, new_break: *const u8) -> Result<*const u8, Error> {
            self.mpu_config.map_or(Err(Error::KernelError),
                                   |mut config|
                                       {
                                           if new_break <
                                                  self.allow_high_water_mark.get()
                                                  ||
                                                  new_break >= self.mem_end()
                                              {
                                               Err(Error::AddressOutOfBounds)
                                           } else if new_break >
                                                         self.kernel_memory_break.get()
                                            {
                                               Err(Error::OutOfMemory)
                                           } else if let Err(_) =
                                                         self.chip.mpu().update_app_memory_region(new_break,
                                                                                                  self.kernel_memory_break.get(),
                                                                                                  mpu::Permissions::ReadWriteOnly,
                                                                                                  &mut config)
                                            {
                                               Err(Error::OutOfMemory)
                                           } else {
                                               let old_break =
                                                   self.app_break.get();
                                               self.app_break.set(new_break);
                                               self.chip.mpu().configure_mpu(&config);
                                               Ok(old_break)
                                           }
                                       })
        }
        fn allow(&self, buf_start_addr: *const u8, size: usize)
         -> Result<Option<AppSlice<Shared, u8>>, ReturnCode> {
            if buf_start_addr == ptr::null_mut() {
                Ok(None)
            } else if self.in_app_owned_memory(buf_start_addr, size) {
                let buf_end_addr = buf_start_addr.wrapping_add(size);
                let new_water_mark =
                    max(self.allow_high_water_mark.get(), buf_end_addr);
                self.allow_high_water_mark.set(new_water_mark);
                Ok(Some(AppSlice::new(buf_start_addr as *mut u8, size,
                                      self.appid())))
            } else { Err(ReturnCode::EINVAL) }
        }
        unsafe fn alloc(&self, size: usize, align: usize)
         -> Option<&mut [u8]> {
            self.mpu_config.and_then(|mut config|
                                         {
                                             let new_break_unaligned =
                                                 self.kernel_memory_break.get().offset(-(size
                                                                                             as
                                                                                             isize));
                                             let alignment_mask =
                                                 !(align - 1);
                                             let new_break =
                                                 (new_break_unaligned as usize
                                                      & alignment_mask) as
                                                     *const u8;
                                             if new_break <
                                                    self.app_break.get() {
                                                 None
                                             } else if let Err(_) =
                                                           self.chip.mpu().update_app_memory_region(self.app_break.get(),
                                                                                                    new_break,
                                                                                                    mpu::Permissions::ReadWriteOnly,
                                                                                                    &mut config)
                                              {
                                                 None
                                             } else {
                                                 self.kernel_memory_break.set(new_break);
                                                 Some(slice::from_raw_parts_mut(new_break
                                                                                    as
                                                                                    *mut u8,
                                                                                size))
                                             }
                                         })
        }
        unsafe fn free(&self, _: *mut u8) { }
        #[allow(clippy :: cast_ptr_alignment)]
        unsafe fn grant_ptr(&self, grant_num: usize) -> *mut *mut u8 {
            let grant_num = grant_num as isize;
            (self.mem_end() as *mut *mut u8).offset(-(grant_num + 1))
        }
        fn get_process_name(&self) -> &'static str { self.process_name }
        unsafe fn set_syscall_return_value(&self, return_value: isize) {
            let mut stored_state = self.stored_state.get();
            self.chip.userspace_kernel_boundary().set_syscall_return_value(self.sp(),
                                                                           &mut stored_state,
                                                                           return_value);
            self.stored_state.set(stored_state);
        }
        unsafe fn set_process_function(&self, callback: FunctionCall) {
            let remaining_stack_bytes =
                self.sp() as usize - self.memory.as_ptr() as usize;
            let mut stored_state = self.stored_state.get();
            match self.chip.userspace_kernel_boundary().set_process_function(self.sp(),
                                                                             remaining_stack_bytes,
                                                                             &mut stored_state,
                                                                             callback)
                {
                Ok(stack_bottom) => {
                    self.kernel.increment_work();
                    self.state.set(State::Running);
                    self.current_stack_pointer.set(stack_bottom as *mut u8);
                    self.debug_set_max_stack_depth();
                }
                Err(bad_stack_bottom) => {
                    self.debug.map(|debug|
                                       {
                                           let bad_stack_bottom =
                                               bad_stack_bottom as *const u8;
                                           if bad_stack_bottom <
                                                  debug.min_stack_pointer {
                                               debug.min_stack_pointer =
                                                   bad_stack_bottom;
                                           }
                                       });
                    self.set_fault_state();
                }
            }
            self.stored_state.set(stored_state);
        }
        unsafe fn switch_to(&self) -> Option<syscall::ContextSwitchReason> {
            let mut stored_state = self.stored_state.get();
            let (stack_pointer, switch_reason) =
                self.chip.userspace_kernel_boundary().switch_to_process(self.sp(),
                                                                        &mut stored_state);
            self.current_stack_pointer.set(stack_pointer as *const u8);
            self.stored_state.set(stored_state);
            self.debug.map(|debug|
                               {
                                   if self.current_stack_pointer.get() <
                                          debug.min_stack_pointer {
                                       debug.min_stack_pointer =
                                           self.current_stack_pointer.get();
                                   }
                                   if switch_reason ==
                                          syscall::ContextSwitchReason::TimesliceExpired
                                      {
                                       debug.timeslice_expiration_count += 1;
                                   }
                               });
            Some(switch_reason)
        }
        fn debug_syscall_count(&self) -> usize {
            self.debug.map_or(0, |debug| debug.syscall_count)
        }
        fn debug_dropped_callback_count(&self) -> usize {
            self.debug.map_or(0, |debug| debug.dropped_callback_count)
        }
        fn debug_restart_count(&self) -> usize {
            self.debug.map_or(0, |debug| debug.restart_count)
        }
        fn debug_timeslice_expiration_count(&self) -> usize {
            self.debug.map_or(0, |debug| debug.timeslice_expiration_count)
        }
        fn debug_timeslice_expired(&self) {
            self.debug.map(|debug| debug.timeslice_expiration_count += 1);
        }
        unsafe fn fault_fmt(&self, writer: &mut dyn Write) {
            self.chip.userspace_kernel_boundary().fault_fmt(writer);
        }
        unsafe fn process_detail_fmt(&self, writer: &mut dyn Write) {
            let flash_end =
                self.flash.as_ptr().add(self.flash.len()) as usize;
            let flash_start = self.flash.as_ptr() as usize;
            let flash_protected_size =
                self.header.get_protected_size() as usize;
            let flash_app_start = flash_start + flash_protected_size;
            let flash_app_size = flash_end - flash_app_start;
            let flash_init_fn =
                flash_start + self.header.get_init_function_offset() as usize;
            let sram_end =
                self.memory.as_ptr().add(self.memory.len()) as usize;
            let sram_grant_start = self.kernel_memory_break.get() as usize;
            let sram_heap_end = self.app_break.get() as usize;
            let sram_heap_start: Option<usize> =
                self.debug.map_or(None,
                                  |debug|
                                      {
                                          debug.app_heap_start_pointer.map(|p|
                                                                               p
                                                                                   as
                                                                                   usize)
                                      });
            let sram_stack_start: Option<usize> =
                self.debug.map_or(None,
                                  |debug|
                                      {
                                          debug.app_stack_start_pointer.map(|p|
                                                                                p
                                                                                    as
                                                                                    usize)
                                      });
            let sram_stack_bottom =
                self.debug.map_or(ptr::null(),
                                  |debug| debug.min_stack_pointer) as usize;
            let sram_start = self.memory.as_ptr() as usize;
            let sram_grant_size = sram_end - sram_grant_start;
            let sram_grant_allocated = sram_end - sram_grant_start;
            let events_queued = self.tasks.map_or(0, |tasks| tasks.len());
            let syscall_count =
                self.debug.map_or(0, |debug| debug.syscall_count);
            let last_syscall = self.debug.map(|debug| debug.last_syscall);
            let dropped_callback_count =
                self.debug.map_or(0, |debug| debug.dropped_callback_count);
            let restart_count =
                self.debug.map_or(0, |debug| debug.restart_count);
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1(&["App: ",
                                                                  "   -   [",
                                                                  "]\r\n Events Queued: ",
                                                                  "   Syscall Count: ",
                                                                  "   Dropped Callback Count: ",
                                                                  "\n Restart Count: ",
                                                                  "\n"],
                                                                &match (&self.process_name,
                                                                        &self.state.get(),
                                                                        &events_queued,
                                                                        &syscall_count,
                                                                        &dropped_callback_count,
                                                                        &restart_count)
                                                                     {
                                                                     (arg0,
                                                                      arg1,
                                                                      arg2,
                                                                      arg3,
                                                                      arg4,
                                                                      arg5) =>
                                                                     [::core::fmt::ArgumentV1::new(arg0,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg1,
                                                                                                   ::core::fmt::Debug::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg2,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg3,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg4,
                                                                                                   ::core::fmt::Display::fmt),
                                                                      ::core::fmt::ArgumentV1::new(arg5,
                                                                                                   ::core::fmt::Display::fmt)],
                                                                 }));
            let _ =
                match last_syscall {
                    Some(syscall) =>
                    writer.write_fmt(::core::fmt::Arguments::new_v1(&[" Last Syscall: "],
                                                                    &match (&syscall,)
                                                                         {
                                                                         (arg0,)
                                                                         =>
                                                                         [::core::fmt::ArgumentV1::new(arg0,
                                                                                                       ::core::fmt::Debug::fmt)],
                                                                     })),
                    None => writer.write_str(" Last Syscall: None"),
                };
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\n\r\n \u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2564}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\r\n \u{2551}  Address  \u{2502} Region Name    Used | Allocated (bytes)  \u{2551}\r\n \u{255a}",
                                                                            "\u{2550}\u{256a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\r\n             \u{2502} \u{25bc} Grant      ",
                                                                            " | ",
                                                                            "",
                                                                            "\r\n  ",
                                                                            " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n             \u{2502} Unused\r\n  ",
                                                                            " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"],
                                                                          &match (&sram_end,
                                                                                  &sram_grant_size,
                                                                                  &sram_grant_allocated,
                                                                                  &exceeded_check(sram_grant_size,
                                                                                                  sram_grant_allocated),
                                                                                  &sram_grant_start,
                                                                                  &sram_heap_end)
                                                                               {
                                                                               (arg0,
                                                                                arg1,
                                                                                arg2,
                                                                                arg3,
                                                                                arg4,
                                                                                arg5)
                                                                               =>
                                                                               [::core::fmt::ArgumentV1::new(arg0,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg1,
                                                                                                             ::core::fmt::Display::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg2,
                                                                                                             ::core::fmt::Display::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg3,
                                                                                                             ::core::fmt::Display::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg4,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg5,
                                                                                                             ::core::fmt::UpperHex::fmt)],
                                                                           },
                                                                          &[::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  0u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(2usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  0u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(3usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  0u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(4usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(5usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},}]));
            match sram_heap_start {
                Some(sram_heap_start) => {
                    let sram_heap_size = sram_heap_end - sram_heap_start;
                    let sram_heap_allocated =
                        sram_grant_start - sram_heap_start;
                    let _ =
                        writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\n             \u{2502} \u{25b2} Heap       ",
                                                                                    " | ",
                                                                                    "",
                                                                                    "     S\r\n  ",
                                                                                    " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} R"],
                                                                                  &match (&sram_heap_size,
                                                                                          &sram_heap_allocated,
                                                                                          &exceeded_check(sram_heap_size,
                                                                                                          sram_heap_allocated),
                                                                                          &sram_heap_start)
                                                                                       {
                                                                                       (arg0,
                                                                                        arg1,
                                                                                        arg2,
                                                                                        arg3)
                                                                                       =>
                                                                                       [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg1,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg2,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg3,
                                                                                                                     ::core::fmt::UpperHex::fmt)],
                                                                                   },
                                                                                  &[::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(2usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(3usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          12u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(10usize),},}]));
                }
                None => {
                    let _ =
                        writer.write_str("\
                     \r\n             │ ▲ Heap            ? |      ?               S\
                     \r\n  ?????????? ┼─────────────────────────────────────────── R");
                }
            }
            match (sram_heap_start, sram_stack_start) {
                (Some(sram_heap_start), Some(sram_stack_start)) => {
                    let sram_data_size = sram_heap_start - sram_stack_start;
                    let sram_data_allocated = sram_data_size as usize;
                    let _ =
                        writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\n             \u{2502} Data         ",
                                                                                    " | ",
                                                                                    "               A"],
                                                                                  &match (&sram_data_size,
                                                                                          &sram_data_allocated)
                                                                                       {
                                                                                       (arg0,
                                                                                        arg1)
                                                                                       =>
                                                                                       [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg1,
                                                                                                                     ::core::fmt::Display::fmt)],
                                                                                   },
                                                                                  &[::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},}]));
                }
                _ => {
                    let _ =
                        writer.write_str("\
                     \r\n             │ Data              ? |      ?               A");
                }
            }
            match sram_stack_start {
                Some(sram_stack_start) => {
                    let sram_stack_size =
                        sram_stack_start - sram_stack_bottom;
                    let sram_stack_allocated = sram_stack_start - sram_start;
                    let _ =
                        writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\n  ",
                                                                                    " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} M\r\n             \u{2502} \u{25bc} Stack      ",
                                                                                    " | ",
                                                                                    ""],
                                                                                  &match (&sram_stack_start,
                                                                                          &sram_stack_size,
                                                                                          &sram_stack_allocated,
                                                                                          &exceeded_check(sram_stack_size,
                                                                                                          sram_stack_allocated))
                                                                                       {
                                                                                       (arg0,
                                                                                        arg1,
                                                                                        arg2,
                                                                                        arg3)
                                                                                       =>
                                                                                       [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                     ::core::fmt::UpperHex::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg1,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg2,
                                                                                                                     ::core::fmt::Display::fmt),
                                                                                        ::core::fmt::ArgumentV1::new(arg3,
                                                                                                                     ::core::fmt::Display::fmt)],
                                                                                   },
                                                                                  &[::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          12u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(2usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                                    ::core::fmt::rt::v1::Argument{position:
                                                                                                                      ::core::fmt::rt::v1::Position::At(3usize),
                                                                                                                  format:
                                                                                                                      ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                          ' ',
                                                                                                                                                      align:
                                                                                                                                                          ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                                      flags:
                                                                                                                                                          0u32,
                                                                                                                                                      precision:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                                      width:
                                                                                                                                                          ::core::fmt::rt::v1::Count::Implied,},}]));
                }
                None => {
                    let _ =
                        writer.write_str("\
                     \r\n  ?????????? ┼─────────────────────────────────────────── M\
                     \r\n             │ ▼ Stack           ? |      ?");
                }
            }
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\n  ",
                                                                            " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n             \u{2502} Unused\r\n  ",
                                                                            " \u{2534}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\r\n             .....\r\n  ",
                                                                            " \u{252c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} F\r\n             \u{2502} App Flash    ",
                                                                            "                        L\r\n  ",
                                                                            " \u{253c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} A\r\n             \u{2502} Protected    ",
                                                                            "                        S\r\n  ",
                                                                            " \u{2534}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} H\r\n"],
                                                                          &match (&sram_stack_bottom,
                                                                                  &sram_start,
                                                                                  &flash_end,
                                                                                  &flash_app_size,
                                                                                  &flash_app_start,
                                                                                  &flash_protected_size,
                                                                                  &flash_start)
                                                                               {
                                                                               (arg0,
                                                                                arg1,
                                                                                arg2,
                                                                                arg3,
                                                                                arg4,
                                                                                arg5,
                                                                                arg6)
                                                                               =>
                                                                               [::core::fmt::ArgumentV1::new(arg0,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg1,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg2,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg3,
                                                                                                             ::core::fmt::Display::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg4,
                                                                                                             ::core::fmt::UpperHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg5,
                                                                                                             ::core::fmt::Display::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg6,
                                                                                                             ::core::fmt::UpperHex::fmt)],
                                                                           },
                                                                          &[::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(2usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(3usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  0u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(4usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(5usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  0u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(6usize),},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(6usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  12u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Is(10usize),},}]));
            self.chip.userspace_kernel_boundary().process_detail_fmt(self.sp(),
                                                                     &self.stored_state.get(),
                                                                     writer);
            self.mpu_config.map(|config|
                                    {
                                        let _ =
                                            writer.write_fmt(::core::fmt::Arguments::new_v1(&[""],
                                                                                            &match (&config,)
                                                                                                 {
                                                                                                 (arg0,)
                                                                                                 =>
                                                                                                 [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                               ::core::fmt::Display::fmt)],
                                                                                             }));
                                    });
            let _ =
                writer.write_fmt(::core::fmt::Arguments::new_v1_formatted(&["\r\nTo debug, run `make debug RAM_START=",
                                                                            " FLASH_INIT=",
                                                                            "`\r\nin the app\'s folder and open the .lst file.\r\n\r\n"],
                                                                          &match (&sram_start,
                                                                                  &flash_init_fn)
                                                                               {
                                                                               (arg0,
                                                                                arg1)
                                                                               =>
                                                                               [::core::fmt::ArgumentV1::new(arg0,
                                                                                                             ::core::fmt::LowerHex::fmt),
                                                                                ::core::fmt::ArgumentV1::new(arg1,
                                                                                                             ::core::fmt::LowerHex::fmt)],
                                                                           },
                                                                          &[::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(0usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  4u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,},},
                                                                            ::core::fmt::rt::v1::Argument{position:
                                                                                                              ::core::fmt::rt::v1::Position::At(1usize),
                                                                                                          format:
                                                                                                              ::core::fmt::rt::v1::FormatSpec{fill:
                                                                                                                                                  ' ',
                                                                                                                                              align:
                                                                                                                                                  ::core::fmt::rt::v1::Alignment::Unknown,
                                                                                                                                              flags:
                                                                                                                                                  4u32,
                                                                                                                                              precision:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,
                                                                                                                                              width:
                                                                                                                                                  ::core::fmt::rt::v1::Count::Implied,},}]));
        }
    }
    fn exceeded_check(size: usize, allocated: usize) -> &'static str {
        if size > allocated { " EXCEEDED!" } else { "          " }
    }
    impl <C: 'static + Chip> Process<'a, C> {
        #[allow(clippy :: cast_ptr_alignment)]
        crate unsafe fn create(kernel: &'static Kernel, chip: &'static C,
                               app_flash_address: *const u8,
                               remaining_app_memory: *mut u8,
                               remaining_app_memory_size: usize,
                               fault_response: FaultResponse, index: usize)
         -> (Option<&'static dyn ProcessType>, usize, usize) {
            if let Some(tbf_header) =
                   tbfheader::parse_and_validate_tbf_header(app_flash_address)
               {
                let app_flash_size = tbf_header.get_total_size() as usize;
                if !tbf_header.is_app() || !tbf_header.enabled() {
                    return (None, app_flash_size, 0);
                }
                let mut min_app_ram_size =
                    tbf_header.get_minimum_app_ram_size() as usize;
                let process_name = tbf_header.get_package_name();
                let init_fn =
                    app_flash_address.offset(tbf_header.get_init_function_offset()
                                                 as isize) as usize;
                let mut mpu_config: <<C as Chip>::MPU as MPU>::MpuConfig =
                    Default::default();
                if let None =
                       chip.mpu().allocate_region(app_flash_address,
                                                  app_flash_size,
                                                  app_flash_size,
                                                  mpu::Permissions::ReadExecuteOnly,
                                                  &mut mpu_config) {
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
                    grant_ptrs_offset + callbacks_offset +
                        process_struct_offset;
                let initial_app_memory_size = 3 * 1024;
                if min_app_ram_size < initial_app_memory_size {
                    min_app_ram_size = initial_app_memory_size;
                }
                let min_total_memory_size =
                    min_app_ram_size + initial_kernel_memory_size;
                let (memory_start, memory_size) =
                    match chip.mpu().allocate_app_memory_region(remaining_app_memory
                                                                    as
                                                                    *const u8,
                                                                remaining_app_memory_size,
                                                                min_total_memory_size,
                                                                initial_app_memory_size,
                                                                initial_kernel_memory_size,
                                                                mpu::Permissions::ReadWriteOnly,
                                                                &mut mpu_config)
                        {
                        Some((memory_start, memory_size)) =>
                        (memory_start, memory_size),
                        None => { return (None, app_flash_size, 0); }
                    };
                let memory_padding_size =
                    (memory_start as usize) - (remaining_app_memory as usize);
                let app_memory =
                    slice::from_raw_parts_mut(memory_start as *mut u8,
                                              memory_size);
                let initial_stack_pointer =
                    memory_start.add(initial_app_memory_size);
                let initial_sbrk_pointer =
                    memory_start.add(initial_app_memory_size);
                let mut kernel_memory_break =
                    app_memory.as_mut_ptr().add(app_memory.len());
                kernel_memory_break =
                    kernel_memory_break.offset(-(grant_ptrs_offset as isize));
                let opts =
                    slice::from_raw_parts_mut(kernel_memory_break as
                                                  *mut *const usize,
                                              grant_ptrs_num);
                for opt in opts.iter_mut() { *opt = ptr::null() }
                kernel_memory_break =
                    kernel_memory_break.offset(-(callbacks_offset as isize));
                let callback_buf =
                    slice::from_raw_parts_mut(kernel_memory_break as
                                                  *mut Task, callback_len);
                let tasks = RingBuffer::new(callback_buf);
                kernel_memory_break =
                    kernel_memory_break.offset(-(process_struct_offset as
                                                     isize));
                let process_struct_memory_location = kernel_memory_break;
                let app_heap_start_pointer = None;
                let app_stack_start_pointer = None;
                let mut process: &mut Process<C> =
                    &mut *(process_struct_memory_location as
                               *mut Process<'static, C>);
                process.app_idx = index;
                process.kernel = kernel;
                process.chip = chip;
                process.memory = app_memory;
                process.header = tbf_header;
                process.kernel_memory_break = Cell::new(kernel_memory_break);
                process.original_kernel_memory_break = kernel_memory_break;
                process.app_break = Cell::new(initial_sbrk_pointer);
                process.original_app_break = initial_sbrk_pointer;
                process.allow_high_water_mark =
                    Cell::new(remaining_app_memory);
                process.current_stack_pointer =
                    Cell::new(initial_stack_pointer);
                process.original_stack_pointer = initial_stack_pointer;
                process.flash =
                    slice::from_raw_parts(app_flash_address, app_flash_size);
                process.stored_state = Cell::new(Default::default());
                process.state = Cell::new(State::Unstarted);
                process.fault_response = fault_response;
                process.mpu_config = MapCell::new(mpu_config);
                process.mpu_regions =
                    [Cell::new(None), Cell::new(None), Cell::new(None),
                     Cell::new(None), Cell::new(None), Cell::new(None)];
                process.tasks = MapCell::new(tasks);
                process.process_name = process_name;
                process.debug =
                    MapCell::new(ProcessDebug{app_heap_start_pointer:
                                                  app_heap_start_pointer,
                                              app_stack_start_pointer:
                                                  app_stack_start_pointer,
                                              min_stack_pointer:
                                                  initial_stack_pointer,
                                              syscall_count: 0,
                                              last_syscall: None,
                                              dropped_callback_count: 0,
                                              restart_count: 0,
                                              timeslice_expiration_count:
                                                  0,});
                let flash_protected_size =
                    process.header.get_protected_size() as usize;
                let flash_app_start =
                    app_flash_address as usize + flash_protected_size;
                process.tasks.map(|tasks|
                                      {
                                          tasks.enqueue(Task::FunctionCall(FunctionCall{source:
                                                                                            FunctionCallSource::Kernel,
                                                                                        pc:
                                                                                            init_fn,
                                                                                        argument0:
                                                                                            flash_app_start,
                                                                                        argument1:
                                                                                            process.memory.as_ptr()
                                                                                                as
                                                                                                usize,
                                                                                        argument2:
                                                                                            process.memory.len()
                                                                                                as
                                                                                                usize,
                                                                                        argument3:
                                                                                            process.app_break.get()
                                                                                                as
                                                                                                usize,}));
                                      });
                let mut stored_state = process.stored_state.get();
                match chip.userspace_kernel_boundary().initialize_new_process(process.sp(),
                                                                              process.sp()
                                                                                  as
                                                                                  usize
                                                                                  -
                                                                                  process.memory.as_ptr()
                                                                                      as
                                                                                      usize,
                                                                              &mut stored_state)
                    {
                    Ok(new_stack_pointer) => {
                        process.current_stack_pointer.set(new_stack_pointer as
                                                              *mut u8);
                        process.debug_set_max_stack_depth();
                        process.stored_state.set(stored_state);
                    }
                    Err(_) => { return (None, app_flash_size, 0); }
                };
                kernel.increment_work();
                return (Some(process), app_flash_size,
                        memory_padding_size + memory_size);
            }
            (None, 0, 0)
        }
        #[allow(clippy :: cast_ptr_alignment)]
        fn sp(&self) -> *const usize {
            self.current_stack_pointer.get() as *const usize
        }
        fn in_app_owned_memory(&self, buf_start_addr: *const u8, size: usize)
         -> bool {
            let buf_end_addr = buf_start_addr.wrapping_add(size);
            buf_end_addr >= buf_start_addr &&
                buf_start_addr >= self.mem_start() &&
                buf_end_addr <= self.app_break.get()
        }
        #[allow(clippy :: cast_ptr_alignment)]
        unsafe fn grant_ptrs_reset(&self) {
            let grant_ptrs_num = self.kernel.get_grant_count_and_finalize();
            for grant_num in 0..grant_ptrs_num {
                let grant_num = grant_num as isize;
                let ctr_ptr =
                    (self.mem_end() as
                         *mut *mut usize).offset(-(grant_num + 1));
                write_volatile(ctr_ptr, ptr::null_mut());
            }
        }
        fn debug_set_max_stack_depth(&self) {
            self.debug.map(|debug|
                               {
                                   if self.current_stack_pointer.get() <
                                          debug.min_stack_pointer {
                                       debug.min_stack_pointer =
                                           self.current_stack_pointer.get();
                                   }
                               });
        }
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
        fn clone(&self) -> ReturnCode {
            { let _: ::core::clone::AssertParamIsClone<usize>; *self }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ReturnCode { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ReturnCode {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&ReturnCode::SuccessWithValue { value: ref __self_0 },) => {
                    let mut debug_trait_builder =
                        f.debug_struct("SuccessWithValue");
                    let _ = debug_trait_builder.field("value", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&ReturnCode::SUCCESS,) => {
                    let mut debug_trait_builder = f.debug_tuple("SUCCESS");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::FAIL,) => {
                    let mut debug_trait_builder = f.debug_tuple("FAIL");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::EBUSY,) => {
                    let mut debug_trait_builder = f.debug_tuple("EBUSY");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::EALREADY,) => {
                    let mut debug_trait_builder = f.debug_tuple("EALREADY");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::EOFF,) => {
                    let mut debug_trait_builder = f.debug_tuple("EOFF");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ERESERVE,) => {
                    let mut debug_trait_builder = f.debug_tuple("ERESERVE");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::EINVAL,) => {
                    let mut debug_trait_builder = f.debug_tuple("EINVAL");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ESIZE,) => {
                    let mut debug_trait_builder = f.debug_tuple("ESIZE");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ECANCEL,) => {
                    let mut debug_trait_builder = f.debug_tuple("ECANCEL");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ENOMEM,) => {
                    let mut debug_trait_builder = f.debug_tuple("ENOMEM");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ENOSUPPORT,) => {
                    let mut debug_trait_builder = f.debug_tuple("ENOSUPPORT");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ENODEVICE,) => {
                    let mut debug_trait_builder = f.debug_tuple("ENODEVICE");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::EUNINSTALLED,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("EUNINSTALLED");
                    debug_trait_builder.finish()
                }
                (&ReturnCode::ENOACK,) => {
                    let mut debug_trait_builder = f.debug_tuple("ENOACK");
                    debug_trait_builder.finish()
                }
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for ReturnCode {
        #[inline]
        fn eq(&self, other: &ReturnCode) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ReturnCode::SuccessWithValue { value: ref __self_0
                         }, &ReturnCode::SuccessWithValue {
                         value: ref __arg_1_0 }) =>
                        (*__self_0) == (*__arg_1_0),
                        _ => true,
                    }
                } else { false }
            }
        }
        #[inline]
        fn ne(&self, other: &ReturnCode) -> bool {
            {
                let __self_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*self) }
                        as isize;
                let __arg_1_vi =
                    unsafe { ::core::intrinsics::discriminant_value(&*other) }
                        as isize;
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        (&ReturnCode::SuccessWithValue { value: ref __self_0
                         }, &ReturnCode::SuccessWithValue {
                         value: ref __arg_1_0 }) =>
                        (*__self_0) != (*__arg_1_0),
                        _ => false,
                    }
                } else { true }
            }
        }
    }
    impl From<ReturnCode> for isize {
        fn from(original: ReturnCode) -> isize {
            match original {
                ReturnCode::SuccessWithValue { value } => value as isize,
                ReturnCode::SUCCESS => 0,
                ReturnCode::FAIL => -1,
                ReturnCode::EBUSY => -2,
                ReturnCode::EALREADY => -3,
                ReturnCode::EOFF => -4,
                ReturnCode::ERESERVE => -5,
                ReturnCode::EINVAL => -6,
                ReturnCode::ESIZE => -7,
                ReturnCode::ECANCEL => -8,
                ReturnCode::ENOMEM => -9,
                ReturnCode::ENOSUPPORT => -10,
                ReturnCode::ENODEVICE => -11,
                ReturnCode::EUNINSTALLED => -12,
                ReturnCode::ENOACK => -13,
            }
        }
    }
    impl From<ReturnCode> for usize {
        fn from(original: ReturnCode) -> usize {
            isize::from(original) as usize
        }
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
            Kernel{work: Cell::new(0),
                   processes: processes,
                   grant_counter: Cell::new(0),
                   grants_finalized: Cell::new(false),}
        }
        crate fn increment_work(&self) { self.work.increment(); }
        crate fn decrement_work(&self) { self.work.decrement(); }
        fn processes_blocked(&self) -> bool { self.work.get() == 0 }
        crate fn process_map_or<F,
                                R>(&self, default: R, process_index: usize,
                                   closure: F) -> R where
         F: FnOnce(&dyn process::ProcessType) -> R {
            if process_index > self.processes.len() { return default; }
            self.processes[process_index].map_or(default,
                                                 |process| closure(process))
        }
        crate fn process_each<F>(&self, closure: F) where
         F: Fn(&dyn process::ProcessType) {
            for process in self.processes.iter() {
                match process { Some(p) => { closure(*p); } None => { } }
            }
        }
        pub fn process_each_capability<F>(&'static self,
                                          _capability:
                                              &dyn capabilities::ProcessManagementCapability,
                                          closure: F) where
         F: Fn(usize, &dyn process::ProcessType) {
            for (i, process) in self.processes.iter().enumerate() {
                match process { Some(p) => { closure(i, *p); } None => { } }
            }
        }
        crate fn process_until<F>(&self, closure: F) -> ReturnCode where
         F: Fn(&dyn process::ProcessType) -> ReturnCode {
            for process in self.processes.iter() {
                match process {
                    Some(p) => {
                        let ret = closure(*p);
                        if ret != ReturnCode::FAIL { return ret; }
                    }
                    None => { }
                }
            }
            ReturnCode::FAIL
        }
        crate fn number_of_process_slots(&self) -> usize {
            self.processes.len()
        }
        pub fn create_grant<T: Default>(&'static self,
                                        _capability:
                                            &dyn capabilities::MemoryAllocationCapability)
         -> Grant<T> {
            if self.grants_finalized.get() {
                {
                    ::core::panicking::panic(&("Grants finalized. Cannot create a new grant.",
                                               "/Users/felixklock/Dev/Mozilla/issue65774/demo-minimization/tock/kernel/src/sched.rs",
                                               164u32, 13u32))
                };
            }
            let grant_index = self.grant_counter.get();
            self.grant_counter.increment();
            Grant::new(self, grant_index)
        }
        crate fn get_grant_count_and_finalize(&self) -> usize {
            self.grants_finalized.set(true);
            self.grant_counter.get()
        }
        pub fn hardfault_all_apps<C: capabilities::ProcessManagementCapability>(&self,
                                                                                _c:
                                                                                    &C) {
            for p in self.processes.iter() {
                p.map(|process| { process.set_fault_state(); });
            }
        }
        pub fn kernel_loop<P: Platform,
                           C: Chip>(&'static self, platform: &P, chip: &C,
                                    ipc: Option<&ipc::IPC>,
                                    _capability:
                                        &dyn capabilities::MainLoopCapability) {
            loop  {
                unsafe {
                    chip.service_pending_interrupts();
                    DynamicDeferredCall::call_global_instance_while(||
                                                                        !chip.has_pending_interrupts());
                    for p in self.processes.iter() {
                        p.map(|process|
                                  {
                                      self.do_process(platform, chip, process,
                                                      ipc);
                                  });
                        if chip.has_pending_interrupts() ||
                               DynamicDeferredCall::global_instance_calls_pending().unwrap_or(false)
                           {
                            break ;
                        }
                    }
                    chip.atomic(||
                                    {
                                        if !chip.has_pending_interrupts() &&
                                               !DynamicDeferredCall::global_instance_calls_pending().unwrap_or(false)
                                               && self.processes_blocked() {
                                            chip.sleep();
                                        }
                                    });
                };
            }
        }
        unsafe fn do_process<P: Platform,
                             C: Chip>(&self, platform: &P, chip: &C,
                                      process: &dyn process::ProcessType,
                                      ipc: Option<&crate::ipc::IPC>) {
            let appid = process.appid();
            let systick = chip.systick();
            systick.reset();
            systick.set_timer(KERNEL_TICK_DURATION_US);
            systick.enable(false);
            loop  {
                if chip.has_pending_interrupts() { break ; }
                if systick.overflowed() ||
                       !systick.greater_than(MIN_QUANTA_THRESHOLD_US) {
                    process.debug_timeslice_expired();
                    break ;
                }
                match process.get_state() {
                    process::State::Running => {
                        process.setup_mpu();
                        chip.mpu().enable_mpu();
                        systick.enable(true);
                        let context_switch_reason = process.switch_to();
                        systick.enable(false);
                        chip.mpu().disable_mpu();
                        match context_switch_reason {
                            Some(ContextSwitchReason::Fault) => {
                                process.set_fault_state();
                            }
                            Some(ContextSwitchReason::SyscallFired { syscall
                                 }) => {
                                match syscall {
                                    Syscall::MEMOP { operand, arg0 } => {
                                        let res =
                                            memop::memop(process, operand,
                                                         arg0);
                                        process.set_syscall_return_value(res.into());
                                    }
                                    Syscall::YIELD => {
                                        process.set_yielded_state();
                                        continue ;
                                    }
                                    Syscall::SUBSCRIBE {
                                    driver_number,
                                    subdriver_number,
                                    callback_ptr,
                                    appdata } => {
                                        let callback_id =
                                            CallbackId{driver_num:
                                                           driver_number,
                                                       subscribe_num:
                                                           subdriver_number,};
                                        process.remove_pending_callbacks(callback_id);
                                        let callback_ptr =
                                            NonNull::new(callback_ptr);
                                        let callback =
                                            callback_ptr.map(|ptr|
                                                                 {
                                                                     Callback::new(appid,
                                                                                   callback_id,
                                                                                   appdata,
                                                                                   ptr.cast())
                                                                 });
                                        let res =
                                            platform.with_driver(driver_number,
                                                                 |driver|
                                                                     match driver
                                                                         {
                                                                         Some(d)
                                                                         => {
                                                                             d.subscribe(subdriver_number,
                                                                                         callback,
                                                                                         appid)
                                                                         }
                                                                         None
                                                                         =>
                                                                         ReturnCode::ENODEVICE,
                                                                     });
                                        process.set_syscall_return_value(res.into());
                                    }
                                    Syscall::COMMAND {
                                    driver_number,
                                    subdriver_number,
                                    arg0,
                                    arg1 } => {
                                        let res =
                                            platform.with_driver(driver_number,
                                                                 |driver|
                                                                     match driver
                                                                         {
                                                                         Some(d)
                                                                         => {
                                                                             d.command(subdriver_number,
                                                                                       arg0,
                                                                                       arg1,
                                                                                       appid)
                                                                         }
                                                                         None
                                                                         =>
                                                                         ReturnCode::ENODEVICE,
                                                                     });
                                        process.set_syscall_return_value(res.into());
                                    }
                                    Syscall::ALLOW {
                                    driver_number,
                                    subdriver_number,
                                    allow_address,
                                    allow_size } => {
                                        let res =
                                            platform.with_driver(driver_number,
                                                                 |driver|
                                                                     {
                                                                         match driver
                                                                             {
                                                                             Some(d)
                                                                             =>
                                                                             {
                                                                                 match process.allow(allow_address,
                                                                                                     allow_size)
                                                                                     {
                                                                                     Ok(oslice)
                                                                                     =>
                                                                                     {
                                                                                         d.allow(appid,
                                                                                                 subdriver_number,
                                                                                                 oslice)
                                                                                     }
                                                                                     Err(err)
                                                                                     =>
                                                                                     err,
                                                                                 }
                                                                             }
                                                                             None
                                                                             =>
                                                                             ReturnCode::ENODEVICE,
                                                                         }
                                                                     });
                                        process.set_syscall_return_value(res.into());
                                    }
                                }
                            }
                            Some(ContextSwitchReason::TimesliceExpired) => {
                                break ;
                            }
                            Some(ContextSwitchReason::Interrupted) => {
                                break ;
                            }
                            None => { process.set_fault_state(); }
                        }
                    }
                    process::State::Yielded | process::State::Unstarted =>
                    match process.dequeue_task() {
                        None => break ,
                        Some(cb) =>
                        match cb {
                            Task::FunctionCall(ccb) => {
                                process.set_process_function(ccb);
                            }
                            Task::IPC((otherapp, ipc_type)) => {
                                ipc.map_or_else(||
                                                    {
                                                        if !false {
                                                            {
                                                                ::core::panicking::panic(&("Kernel consistency error: IPC Task with no IPC",
                                                                                           "/Users/felixklock/Dev/Mozilla/issue65774/demo-minimization/tock/kernel/src/sched.rs",
                                                                                           393u32,
                                                                                           37u32))
                                                            }
                                                        };
                                                    },
                                                |ipc|
                                                    {
                                                        ipc.schedule_callback(appid,
                                                                              otherapp,
                                                                              ipc_type);
                                                    });
                            }
                        },
                    },
                    process::State::Fault => {
                        {
                            ::core::panicking::panic(&("Attempted to schedule a faulty process",
                                                       "/Users/felixklock/Dev/Mozilla/issue65774/demo-minimization/tock/kernel/src/sched.rs",
                                                       407u32, 21u32))
                        };
                    }
                    process::State::StoppedRunning => { break ; }
                    process::State::StoppedYielded => { break ; }
                    process::State::StoppedFaulted => { break ; }
                }
            }
            systick.reset();
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
        fn clone(&self) -> TbfHeaderV2Base {
            {
                let _: ::core::clone::AssertParamIsClone<u16>;
                let _: ::core::clone::AssertParamIsClone<u16>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Base { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2Base {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TbfHeaderV2Base {
                version: ref __self_0_0,
                header_size: ref __self_0_1,
                total_size: ref __self_0_2,
                flags: ref __self_0_3,
                checksum: ref __self_0_4 } => {
                    let mut debug_trait_builder =
                        f.debug_struct("TbfHeaderV2Base");
                    let _ =
                        debug_trait_builder.field("version", &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("header_size",
                                                  &&(*__self_0_1));
                    let _ =
                        debug_trait_builder.field("total_size",
                                                  &&(*__self_0_2));
                    let _ =
                        debug_trait_builder.field("flags", &&(*__self_0_3));
                    let _ =
                        debug_trait_builder.field("checksum",
                                                  &&(*__self_0_4));
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> TbfHeaderTypes { { *self } }
    }
        #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::marker::Copy for TbfHeaderTypes { }
        #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for TbfHeaderTypes {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&TbfHeaderTypes::TbfHeaderMain,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("TbfHeaderMain");
                    debug_trait_builder.finish()
                }
                (&TbfHeaderTypes::TbfHeaderWriteableFlashRegions,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("TbfHeaderWriteableFlashRegions");
                    debug_trait_builder.finish()
                }
                (&TbfHeaderTypes::TbfHeaderPackageName,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("TbfHeaderPackageName");
                    debug_trait_builder.finish()
                }
                (&TbfHeaderTypes::Unused,) => {
                    let mut debug_trait_builder = f.debug_tuple("Unused");
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> TbfHeaderTlv {
            {
                let _: ::core::clone::AssertParamIsClone<TbfHeaderTypes>;
                let _: ::core::clone::AssertParamIsClone<u16>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderTlv { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderTlv {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TbfHeaderTlv { tipe: ref __self_0_0, length: ref __self_0_1 }
                => {
                    let mut debug_trait_builder =
                        f.debug_struct("TbfHeaderTlv");
                    let _ =
                        debug_trait_builder.field("tipe", &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("length", &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> TbfHeaderV2Main {
            {
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Main { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2Main {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TbfHeaderV2Main {
                init_fn_offset: ref __self_0_0,
                protected_size: ref __self_0_1,
                minimum_ram_size: ref __self_0_2 } => {
                    let mut debug_trait_builder =
                        f.debug_struct("TbfHeaderV2Main");
                    let _ =
                        debug_trait_builder.field("init_fn_offset",
                                                  &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("protected_size",
                                                  &&(*__self_0_1));
                    let _ =
                        debug_trait_builder.field("minimum_ram_size",
                                                  &&(*__self_0_2));
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> TbfHeaderV2WriteableFlashRegion {
            {
                let _: ::core::clone::AssertParamIsClone<u32>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2WriteableFlashRegion { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2WriteableFlashRegion {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TbfHeaderV2WriteableFlashRegion {
                writeable_flash_region_offset: ref __self_0_0,
                writeable_flash_region_size: ref __self_0_1 } => {
                    let mut debug_trait_builder =
                        f.debug_struct("TbfHeaderV2WriteableFlashRegion");
                    let _ =
                        debug_trait_builder.field("writeable_flash_region_offset",
                                                  &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("writeable_flash_region_size",
                                                  &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
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
        fn clone(&self) -> TbfHeaderV2 {
            {
                let _:
                        ::core::clone::AssertParamIsClone<&'static TbfHeaderV2Base>;
                let _:
                        ::core::clone::AssertParamIsClone<Option<&'static TbfHeaderV2Main>>;
                let _:
                        ::core::clone::AssertParamIsClone<Option<&'static str>>;
                let _:
                        ::core::clone::AssertParamIsClone<Option<&'static [TbfHeaderV2WriteableFlashRegion]>>;
                *self
            }
        }
    }
        #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2 { }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeaderV2 {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                TbfHeaderV2 {
                base: ref __self_0_0,
                main: ref __self_0_1,
                package_name: ref __self_0_2,
                writeable_regions: ref __self_0_3 } => {
                    let mut debug_trait_builder =
                        f.debug_struct("TbfHeaderV2");
                    let _ =
                        debug_trait_builder.field("base", &&(*__self_0_0));
                    let _ =
                        debug_trait_builder.field("main", &&(*__self_0_1));
                    let _ =
                        debug_trait_builder.field("package_name",
                                                  &&(*__self_0_2));
                    let _ =
                        debug_trait_builder.field("writeable_regions",
                                                  &&(*__self_0_3));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    crate enum TbfHeader {
        TbfHeaderV2(TbfHeaderV2),
        Padding(&'static TbfHeaderV2Base),
    }
        #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TbfHeader {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&TbfHeader::TbfHeaderV2(ref __self_0),) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("TbfHeaderV2");
                    let _ = debug_trait_builder.field(&&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&TbfHeader::Padding(ref __self_0),) => {
                    let mut debug_trait_builder = f.debug_tuple("Padding");
                    let _ = debug_trait_builder.field(&&(*__self_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    impl TbfHeader {
        crate fn is_app(&self) -> bool {
            match *self {
                TbfHeader::TbfHeaderV2(_) => true,
                TbfHeader::Padding(_) => false,
            }
        }
        crate fn enabled(&self) -> bool {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.base.flags & 0x00000001 == 1
                }
                TbfHeader::Padding(_) => false,
            }
        }
        crate fn get_total_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => hd.base.total_size,
                TbfHeader::Padding(hd) => hd.total_size,
            }
        }
        crate fn get_minimum_app_ram_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) =>
                hd.main.map_or(0, |m| m.minimum_ram_size),
                _ => 0,
            }
        }
        crate fn get_protected_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.main.map_or(0, |m| m.protected_size) +
                        (hd.base.header_size as u32)
                }
                _ => 0,
            }
        }
        crate fn get_init_function_offset(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.main.map_or(0, |m| m.init_fn_offset) +
                        (hd.base.header_size as u32)
                }
                _ => 0,
            }
        }
        crate fn get_package_name(&self) -> &'static str {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => hd.package_name.unwrap_or(""),
                _ => "",
            }
        }
        crate fn number_writeable_flash_regions(&self) -> usize {
            match *self {
                TbfHeader::TbfHeaderV2(hd) =>
                hd.writeable_regions.map_or(0, |wr| wr.len()),
                _ => 0,
            }
        }
        crate fn get_writeable_flash_region(&self, index: usize)
         -> (u32, u32) {
            match *self {
                TbfHeader::TbfHeaderV2(hd) =>
                hd.writeable_regions.map_or((0, 0),
                                            |wr|
                                                {
                                                    if wr.len() > index {
                                                        (wr[index].writeable_flash_region_offset,
                                                         wr[index].writeable_flash_region_size)
                                                    } else { (0, 0) }
                                                }),
                _ => (0, 0),
            }
        }
    }
    #[allow(clippy :: cast_ptr_alignment)]
    crate unsafe fn parse_and_validate_tbf_header(address: *const u8)
     -> Option<TbfHeader> {
        let version = *(address as *const u16);
        match version {
            2 => {
                let tbf_header_base = &*(address as *const TbfHeaderV2Base);
                if tbf_header_base.header_size as u32 >=
                       tbf_header_base.total_size ||
                       tbf_header_base.total_size > 0x010000000 {
                    return None;
                }
                let mut chunks = tbf_header_base.header_size as usize / 4;
                let leftover_bytes = tbf_header_base.header_size as usize % 4;
                if leftover_bytes != 0 { chunks += 1; }
                let mut checksum: u32 = 0;
                let header =
                    slice::from_raw_parts(address as *const u32, chunks);
                for (i, chunk) in header.iter().enumerate() {
                    if i == 3 {
                    } else if i == chunks - 1 && leftover_bytes != 0 {
                        checksum ^=
                            *chunk & (0xFFFFFFFF >> (4 - leftover_bytes));
                    } else { checksum ^= *chunk; }
                }
                if checksum != tbf_header_base.checksum { return None; }
                let mut offset = mem::size_of::<TbfHeaderV2Base>() as isize;
                let mut remaining_length =
                    tbf_header_base.header_size as usize - offset as usize;
                if remaining_length == 0 {
                    if checksum == tbf_header_base.checksum {
                        Some(TbfHeader::Padding(tbf_header_base))
                    } else { None }
                } else {
                    let mut main_pointer: Option<&TbfHeaderV2Main> = None;
                    let mut wfr_pointer:
                            Option<&'static [TbfHeaderV2WriteableFlashRegion]> =
                        None;
                    let mut app_name_str = "";
                    while remaining_length > mem::size_of::<TbfHeaderTlv>() {
                        let tbf_tlv_header =
                            &*(address.offset(offset) as *const TbfHeaderTlv);
                        remaining_length -= mem::size_of::<TbfHeaderTlv>();
                        offset += mem::size_of::<TbfHeaderTlv>() as isize;
                        if (tbf_tlv_header.tipe as u16) <
                               TbfHeaderTypes::Unused as u16 &&
                               (tbf_tlv_header.tipe as u16) > 0 {
                            match tbf_tlv_header.tipe {
                                TbfHeaderTypes::TbfHeaderMain => {
                                    if remaining_length >=
                                           mem::size_of::<TbfHeaderV2Main>()
                                           &&
                                           tbf_tlv_header.length as usize ==
                                               mem::size_of::<TbfHeaderV2Main>()
                                       {
                                        let tbf_main =
                                            &*(address.offset(offset) as
                                                   *const TbfHeaderV2Main);
                                        main_pointer = Some(tbf_main);
                                    }
                                }
                                TbfHeaderTypes::TbfHeaderWriteableFlashRegions
                                => {
                                    if tbf_tlv_header.length as usize %
                                           mem::size_of::<TbfHeaderV2WriteableFlashRegion>()
                                           == 0 {
                                        let number_regions =
                                            tbf_tlv_header.length as usize /
                                                mem::size_of::<TbfHeaderV2WriteableFlashRegion>();
                                        let region_start =
                                            &*(address.offset(offset) as
                                                   *const TbfHeaderV2WriteableFlashRegion);
                                        let regions =
                                            slice::from_raw_parts(region_start,
                                                                  number_regions);
                                        wfr_pointer = Some(regions);
                                    }
                                }
                                TbfHeaderTypes::TbfHeaderPackageName => {
                                    if remaining_length >=
                                           tbf_tlv_header.length as usize {
                                        let package_name_byte_array =
                                            slice::from_raw_parts(address.offset(offset),
                                                                  tbf_tlv_header.length
                                                                      as
                                                                      usize);
                                        let _ =
                                            str::from_utf8(package_name_byte_array).map(|name_str|
                                                                                            {
                                                                                                app_name_str
                                                                                                    =
                                                                                                    name_str;
                                                                                            });
                                    }
                                }
                                TbfHeaderTypes::Unused => { }
                            }
                        }
                        remaining_length -=
                            ((tbf_tlv_header.length) +
                                 ((4 - ((tbf_tlv_header.length) % 4)) % 4)) as
                                usize;
                        offset +=
                            ((tbf_tlv_header.length) +
                                 ((4 - ((tbf_tlv_header.length) % 4)) % 4)) as
                                isize;
                    }
                    let tbf_header =
                        TbfHeaderV2{base: tbf_header_base,
                                    main: main_pointer,
                                    package_name: Some(app_name_str),
                                    writeable_regions: wfr_pointer,};
                    Some(TbfHeader::TbfHeaderV2(tbf_header))
                }
            }
            _ => None,
        }
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
