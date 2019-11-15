#![feature(derive_clone_copy, compiler_builtins_lib, fmt_internals, core_panic, derive_eq)]
#![feature(prelude_import)]
//! Core Tock Kernel
//!
//! The kernel crate implements the core features of Tock as well as shared
//! code that many chips, capsules, and boards use. It also holds the Hardware
//! Interface Layer (HIL) definitions.
//!
//! Most `unsafe` code is in this kernel crate.

#![feature(core_intrinsics, ptr_internals, const_fn)]
#![feature(panic_info_message)]
#![feature(in_band_lifetimes, crate_visibility_modifier)]
#![feature(associated_type_defaults)]
#![warn(unreachable_pub)]
#![no_std]
#[prelude_import]
use core::prelude::v1::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;

pub mod capabilities {



    // Export only select items from the process module. To remove the name conflict
    // this cannot be called `process`, so we use a shortened version. These
    // functions and types are used by board files to setup the platform and setup
    // processes.
    //! Special restricted capabilities.
    //!
    //! Rust provides a mechanism for restricting certain operations to only be used
    //! by trusted code through the `unsafe` keyword. This is very useful, but
    //! doesn't provide very granular access: code can either access _all_ `unsafe`
    //! things, or none.
    //!
    //! Capabilities are the mechanism in Tock that provides more granular access.
    //! For sensitive operations (e.g. operations that could violate isolation)
    //! callers must have a particular capability. The type system ensures that the
    //! caller does in fact have the capability, and `unsafe` is used to ensure that
    //! callers cannot create the capability type themselves.
    //!
    //! Capabilities are passed to modules from trusted code (i.e. code that can
    //! call `unsafe`).
    //!
    //! Capabilities are expressed as `unsafe` traits. Only code that can use
    //! `unsafe` mechanisms can instantiate an object that provides an `unsafe`
    //! trait. Functions that require certain capabilities require that they are
    //! passed an object that provides the correct capability trait. The object
    //! itself does not have to be marked `unsafe`.
    //!
    //! Creating an object that expresses a capability is straightforward:
    //!
    //! ```
    //! use kernel::capabilities::ProcessManagementCapability;
    //!
    //! struct ProcessMgmtCap;
    //! unsafe impl ProcessManagementCapability for ProcessMgmtCap {}
    //! ```
    //!
    //! Now anything that has a ProcessMgmtCap can call any function that requires
    //! the `ProcessManagementCapability` capability.
    //!
    //! Requiring a certain capability is also straightforward:
    //!
    //! ```ignore
    //! pub fn manage_process<C: ProcessManagementCapability>(_c: &C) {
    //!    unsafe {
    //!        ...
    //!    }
    //! }
    //! ```
    //!
    //! Anything that calls `manage_process` must have a reference to some object
    //! that provides the `ProcessManagementCapability` trait, which proves that it
    //! has the correct capability.
    /// The `ProcessManagementCapability` allows the holder to control
    /// process execution, such as related to creating, restarting, and
    /// otherwise managing processes.
    pub unsafe trait ProcessManagementCapability { }
    /// The `MainLoopCapability` capability allows the holder to start executing
    /// the main scheduler loop in Tock.
    pub unsafe trait MainLoopCapability { }
    /// The `MemoryAllocationCapability` capability allows the holder to allocate
    /// memory, for example by creating grants.
    pub unsafe trait MemoryAllocationCapability { }
}
pub mod common {
    //! Common operations and types in Tock.
    //!
    //! These are data types and access mechanisms that are used throughout the Tock
    //! kernel. Mostly they simplify common operations and enable the other parts of
    //! the kernel (chips and capsules) to be intuitive, valid Rust. In some cases
    //! they provide safe wrappers around unsafe interface so that other kernel
    //! crates do not need to use unsafe code.
    /// Re-export the tock-register-interface library.
    pub mod registers {
        pub use tock_registers::registers::InMemoryRegister;
        pub use tock_registers::registers::RegisterLongName;
        pub use tock_registers::registers::{Field, FieldValue,
                                            LocalRegisterCopy};
        pub use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
        pub use tock_registers::{register_bitfields, register_structs};
    }
    pub mod deferred_call {
        //! Deferred call mechanism.
        //!
        //! This is a tool to allow chip peripherals to schedule "interrupts"
        //! in the chip scheduler if the hardware doesn't support interrupts where
        //! they are needed.
        use core::convert::Into;
        use core::convert::TryFrom;
        use core::convert::TryInto;
        use core::marker::Copy;
        use core::sync::atomic::AtomicUsize;
        use core::sync::atomic::Ordering;
        static DEFERRED_CALL: AtomicUsize = AtomicUsize::new(0);
        /// Are there any pending `DeferredCall`s?
        pub fn has_tasks() -> bool {
            DEFERRED_CALL.load(Ordering::Relaxed) != 0
        }
        /// Represents a way to generate an asynchronous call without a hardware
        /// interrupt. Supports up to 32 possible deferrable tasks.
        pub struct DeferredCall<T>(T);
        impl <T: Into<usize> + TryFrom<usize> + Copy> DeferredCall<T> {
            /// Creates a new DeferredCall
            ///
            /// Only create one per task, preferably in the module that it will be used
            /// in.
            pub const unsafe fn new(task: T) -> Self { DeferredCall(task) }
            /// Set the `DeferredCall` as pending
            pub fn set(&self) {
                let val = DEFERRED_CALL.load(Ordering::Relaxed);
                let new_val = val | (1 << self.0.into());
                DEFERRED_CALL.store(new_val, Ordering::Relaxed);
            }
            /// Gets and clears the next pending `DeferredCall`
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
        //! Hardware-independent kernel interface for deferred calls
        //!
        //! This allows any struct in the kernel which implements
        //! [DynamicDeferredCallClient](crate::common::dynamic_deferred_call::DynamicDeferredCallClient)
        //! to set and receive deferred calls.
        //!
        //! These can be used to implement long-running in-kernel algorithms
        //! or software devices that are supposed to work like hardware devices.
        //! Essentially, this allows the chip to handle more important interrupts,
        //! and lets a kernel component return the function call stack up to the scheduler,
        //! automatically being called again.
        //!
        //! Usage
        //! -----
        //!
        //! The `dynamic_deferred_call_clients` array size determines how many
        //! [DeferredCallHandle](crate::common::dynamic_deferred_call::DeferredCallHandle)s
        //! may be registered with the instance.
        //! When no more slots are available,
        //! `dynamic_deferred_call.register(some_client)` will return `None`.
        //!
        //! ```
        //! # use core::cell::Cell;
        //! # use kernel::common::cells::OptionalCell;
        //! # use kernel::static_init;
        //! use kernel::common::dynamic_deferred_call::{
        //!     DynamicDeferredCall,
        //!     DynamicDeferredCallClient,
        //!     DynamicDeferredCallClientState,
        //! };
        //!
        //! let dynamic_deferred_call_clients = unsafe { static_init!(
        //!     [DynamicDeferredCallClientState; 2],
        //!     Default::default()
        //! ) };
        //! let dynamic_deferred_call = unsafe { static_init!(
        //!     DynamicDeferredCall,
        //!     DynamicDeferredCall::new(dynamic_deferred_call_clients)
        //! ) };
        //! assert!(unsafe { DynamicDeferredCall::set_global_instance(dynamic_deferred_call) }, true);
        //!
        //! # struct SomeCapsule;
        //! # impl SomeCapsule {
        //! #     pub fn new(_ddc: &'static DynamicDeferredCall) -> Self { SomeCapsule }
        //! #     pub fn set_deferred_call_handle(
        //! #         &self,
        //! #         _handle: kernel::common::dynamic_deferred_call::DeferredCallHandle,
        //! #     ) { }
        //! # }
        //! # impl DynamicDeferredCallClient for SomeCapsule {
        //! #     fn call(
        //! #         &self,
        //! #         _handle: kernel::common::dynamic_deferred_call::DeferredCallHandle,
        //! #     ) { }
        //! # }
        //! #
        //! // Here you can register custom capsules, etc.
        //! // This could look like:
        //! let some_capsule = unsafe { static_init!(
        //!     SomeCapsule,
        //!     SomeCapsule::new(dynamic_deferred_call)
        //! ) };
        //! some_capsule.set_deferred_call_handle(
        //!     dynamic_deferred_call.register(some_capsule).expect("no deferred call slot available")
        //! );
        //! ```
        use crate::common::cells::OptionalCell;
        use core::cell::Cell;
        /// Kernel-global dynamic deferred call instance
        ///
        /// This gets called by the kernel scheduler automatically and is accessible
        /// through `unsafe` static functions on the `DynamicDeferredCall` struct
        static mut DYNAMIC_DEFERRED_CALL: Option<&'static DynamicDeferredCall>
               =
            None;
        /// Internal per-client state tracking for the [DynamicDeferredCall]
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
        /// Dynamic deferred call
        ///
        /// This struct manages and calls dynamically (at runtime) registered
        /// deferred calls from capsules and other kernel structures.
        ///
        /// It has a fixed number of possible clients, which
        /// is determined by the `clients`-array passed in with the constructor.
        pub struct DynamicDeferredCall {
            client_states: &'static [DynamicDeferredCallClientState],
            handle_counter: Cell<usize>,
            call_pending: Cell<bool>,
        }
        impl DynamicDeferredCall {
            /// Construct a new dynamic deferred call implementation
            ///
            /// This needs to be registered with the `set_global_instance` function immediately
            /// afterwards, and should not be changed anymore. Only the globally registered
            /// instance will receive calls from the kernel scheduler.
            ///
            /// The `clients` array can be initialized using the implementation of [Default]
            /// for the [DynamicDeferredCallClientState].
            pub fn new(client_states:
                           &'static [DynamicDeferredCallClientState])
             -> DynamicDeferredCall {
                DynamicDeferredCall{client_states,
                                    handle_counter: Cell::new(0),
                                    call_pending: Cell::new(false),}
            }
            /// Sets a global [DynamicDeferredCall] instance
            ///
            /// This is required before any deferred calls can be retrieved.
            /// It may be called only once. Returns `true` if the global instance
            /// was successfully registered.
            pub unsafe fn set_global_instance(ddc:
                                                  &'static DynamicDeferredCall)
             -> bool {
                (*DYNAMIC_DEFERRED_CALL.get_or_insert(ddc)) as *const _ ==
                    ddc as *const _
            }
            /// Call the globally registered instance
            ///
            /// Returns `true` if a global instance was registered and has been called.
            pub unsafe fn call_global_instance() -> bool {
                DYNAMIC_DEFERRED_CALL.map(|ddc| ddc.call()).is_some()
            }
            /// Call the globally registered instance while the supplied predicate
            /// returns `true`.
            ///
            /// Returns `true` if a global instance was registered and has been called.
            pub unsafe fn call_global_instance_while<F: Fn() -> bool>(f: F)
             -> bool {
                DYNAMIC_DEFERRED_CALL.map(move |ddc|
                                              ddc.call_while(f)).is_some()
            }
            /// Check if one or more dynamic deferred calls are pending in the
            /// globally registered instance
            ///
            /// Returns `None` if no global instance has been registered, or `Some(true)`
            /// if the registered instance has one or more pending deferred calls.
            pub unsafe fn global_instance_calls_pending() -> Option<bool> {
                DYNAMIC_DEFERRED_CALL.map(|ddc| ddc.has_pending())
            }
            /// Schedule a deferred call to be called
            ///
            /// The handle addresses the client that will be called.
            ///
            /// If no client for the handle is found (it was unregistered), this
            /// returns `None`. If a call is already scheduled, it returns
            /// `Some(false)`.
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
            /// Register a new client
            ///
            /// On success, a `Some(handle)` will be returned. This handle is later
            /// required to schedule a deferred call.
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
            /// Check if one or more deferred calls are pending
            ///
            /// Returns `true` if one or more deferred calls are pending.
            pub fn has_pending(&self) -> bool { self.call_pending.get() }
            /// Call all registered and to-be-scheduled deferred calls
            ///
            /// It may be called without holding the `DynamicDeferredCall` reference through
            /// `call_global_instance`.
            pub(self) fn call(&self) { self.call_while(|| true) }
            /// Call all registered and to-be-scheduled deferred calls while the supplied
            /// predicate returns `true`.
            ///
            /// It may be called without holding the `DynamicDeferredCall` reference through
            /// `call_global_instance_while`.
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
        /// Client for the
        /// [DynamicDeferredCall](crate::common::dynamic_deferred_call::DynamicDeferredCall)
        ///
        /// This trait needs to be implemented for some struct to receive
        /// deferred calls from a `DynamicDeferredCall`.
        pub trait DynamicDeferredCallClient {
            fn call(&self, handle: DeferredCallHandle);
        }
        /// Unique identifier for a deferred call registered with a
        /// [DynamicDeferredCall](crate::common::dynamic_deferred_call::DynamicDeferredCall)
        pub struct DeferredCallHandle(usize);
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for DeferredCallHandle { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for DeferredCallHandle {
            #[inline]
            fn clone(&self) -> DeferredCallHandle {
                { let _: ::core::clone::AssertParamIsClone<usize>; *self }
            }
        }
        #[automatically_derived]
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
        //! Linked list implementation.
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
        //! Helper functions for common mathematical operations.
        use core::convert::{From, Into};
        use core::intrinsics as int;
        /// Provide `sqrtf32` with the unsafe hidden.
        pub fn sqrtf32(num: f32) -> f32 { unsafe { int::sqrtf32(num) } }
        extern "C" {
            fn __errno() -> &'static mut i32;
        }
        /// Return errno value and zero it out.
        pub fn get_errno() -> i32 {
            unsafe {
                let errnoaddr = __errno();
                let ret = *errnoaddr;
                *errnoaddr = 0;
                ret
            }
        }
        /// Get closest power of two greater than the given number.
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for PowerOfTwo { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for PowerOfTwo {
            #[inline]
            fn clone(&self) -> PowerOfTwo {
                { let _: ::core::clone::AssertParamIsClone<u32>; *self }
            }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for PowerOfTwo {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () {
                { let _: ::core::cmp::AssertParamIsEq<u32>; }
            }
        }
        #[automatically_derived]
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
        /// Represents an integral power-of-two as an exponent
        impl PowerOfTwo {
            /// Returns the base-2 exponent as a numeric type
            pub fn exp<R>(self) -> R where R: From<u32> { From::from(self.0) }
            /// Converts a number two the nearest `PowerOfTwo` less-than-or-equal to it.
            pub fn floor<F: Into<u32>>(f: F) -> PowerOfTwo {
                PowerOfTwo(log_base_two(f.into()))
            }
            /// Converts a number two the nearest `PowerOfTwo` greater-than-or-equal to
            /// it.
            pub fn ceiling<F: Into<u32>>(f: F) -> PowerOfTwo {
                PowerOfTwo(log_base_two(closest_power_of_two(f.into())))
            }
            /// Creates a new `PowerOfTwo` representing the number zero.
            pub fn zero() -> PowerOfTwo { PowerOfTwo(0) }
            /// Converts a `PowerOfTwo` to a number.
            pub fn as_num<F: From<u32>>(self) -> F { (1 << self.0).into() }
        }
        /// Get log base 2 of a number.
        /// Note: this is the floor of the result. Also, an input of 0 results in an
        /// output of 0
        pub fn log_base_two(num: u32) -> u32 {
            if num == 0 { 0 } else { 31 - num.leading_zeros() }
        }
        /// Log base 2 of 64 bit unsigned integers.
        pub fn log_base_two_u64(num: u64) -> u32 {
            if num == 0 { 0 } else { 63 - num.leading_zeros() }
        }
    }
    pub mod peripherals {
        //! Peripheral Management
        //!
        //! Most peripherals are implemented as memory mapped I/O (MMIO).
        //! Intrinsically, this means that accessing a peripheral requires
        //! dereferencing a raw pointer that points to the peripheral's memory.
        //!
        //! Generally, Tock peripherals are modeled by two structures, such as:
        //!
        //! ```rust
        //! # use kernel::common::cells::VolatileCell;
        //! # use kernel::common::StaticRef;
        //! # struct ChipSpecificPeripheralClock {};
        //! /// The MMIO Structure.
        //! #[repr(C)]
        //! #[allow(dead_code)]
        //! pub struct PeripheralRegisters {
        //!     control: VolatileCell<u32>,
        //!     interrupt_mask: VolatileCell<u32>,
        //! }
        //!
        //! /// The Tock object that holds all information for this peripheral.
        //! pub struct PeripheralHardware<'a> {
        //!     mmio_address: StaticRef<PeripheralRegisters>,
        //!     clock: &'a ChipSpecificPeripheralClock,
        //! }
        //! ```
        //!
        //! The first structure mirrors the MMIO specification. The second structure
        //! holds a pointer to the actual address in memory. It also holds other
        //! information for the peripheral. Kernel traits will be implemented for this
        //! peripheral hardware structure. As the peripheral cannot derefence the raw
        //! MMIO pointer safely, Tock provides the PeripheralManager interface:
        //!
        //! ```rust
        //! # use kernel::common::cells::VolatileCell;
        //! # use kernel::common::peripherals::PeripheralManager;
        //! # use kernel::common::StaticRef;
        //! # use kernel::hil;
        //! # use kernel::ReturnCode;
        //! # struct PeripheralRegisters { control: VolatileCell<u32> };
        //! # struct PeripheralHardware { mmio_address: StaticRef<PeripheralRegisters> };
        //! impl hil::uart::Configure for PeripheralHardware {
        //!     fn configure(&self, params: hil::uart::Parameters) -> ReturnCode {
        //!         let peripheral = &PeripheralManager::new(self);
        //!         peripheral.registers.control.set(0x0);
        //!         //         ^^^^^^^^^-- This is type &PeripheralRegisters
        //!         ReturnCode::SUCCESS
        //!     }
        //! }
        //! # use kernel::common::peripherals::PeripheralManagement;
        //! # use kernel::NoClockControl;
        //! # impl PeripheralManagement<NoClockControl> for PeripheralHardware {
        //! #     type RegisterType = PeripheralRegisters;
        //!
        //! #     fn get_registers(&self) -> &PeripheralRegisters {
        //! #         &*self.mmio_address
        //! #     }
        //! #     fn get_clock(&self) -> &NoClockControl { unsafe { &kernel::NO_CLOCK_CONTROL } }
        //! #     fn before_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        //! #     fn after_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        //! # }
        //! ```
        //!
        //! Each peripheral must tell the kernel where its registers live in memory:
        //!
        //! ```rust
        //! # use kernel::common::peripherals::PeripheralManagement;
        //! # use kernel::common::StaticRef;
        //! # pub struct PeripheralRegisters {};
        //! # pub struct PeripheralHardware { mmio_address: StaticRef<PeripheralRegisters> };
        //! /// Teaching the kernel how to create PeripheralRegisters.
        //! use kernel::NoClockControl;
        //! impl PeripheralManagement<NoClockControl> for PeripheralHardware {
        //!     type RegisterType = PeripheralRegisters;
        //!
        //!     fn get_registers(&self) -> &PeripheralRegisters {
        //!         &*self.mmio_address
        //!     }
        //!     # fn get_clock(&self) -> &NoClockControl { unsafe { &kernel::NO_CLOCK_CONTROL } }
        //!     # fn before_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        //!     # fn after_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        //! }
        //! ```
        //!
        //! Note, this example kept the `mmio_address` in the `PeripheralHardware`
        //! structure, which is useful when there are multiple copies of the same
        //! peripheral (e.g. multiple UARTs). For single-instance peripherals, it's
        //! fine to simply return the address directly from `get_registers`.
        //!
        //! Peripheral Clocks
        //! -----------------
        //!
        //! To facilitate low-power operation, PeripheralManager captures the peripheral's
        //! clock upon instantiation. The intention is to exploit
        //! [Ownership Based Resource Management](https://doc.rust-lang.org/beta/nomicon/obrm.html)
        //! to capture peripheral power state.
        //!
        //! To enable this, peripherals must inform the kernel which clock they use,
        //! and when the clock should be enabled and disabled. Implementations of the
        //! `before/after_mmio_access` methods must take care to not access hardware
        //! without enabling clocks if needed if they use hardware for bookkeeping.
        //!
        //! ```rust
        //! use kernel::common::peripherals::PeripheralManagement;
        //! use kernel::common::StaticRef;
        //! use kernel::ClockInterface;
        //! // A dummy clock for this example.
        //! // Real peripherals that do not have clocks should use NoClockControl from this module.
        //! struct ExampleClock {};
        //! impl ClockInterface for ExampleClock {
        //!     fn is_enabled(&self) -> bool { true }
        //!     fn enable(&self) { }
        //!     fn disable(&self) { }
        //! }
        //!
        //! // Dummy hardware for this example.
        //! struct SpiRegisters {};
        //! struct SpiHw<'a> {
        //!     mmio_address: StaticRef<SpiRegisters>,
        //!     clock: &'a ExampleClock,
        //!     busy: bool,
        //! };
        //!
        //! /// Teaching the kernel which clock controls SpiHw.
        //! impl<'a> PeripheralManagement<ExampleClock> for SpiHw<'a> {
        //!     type RegisterType = SpiRegisters;
        //!
        //!     fn get_registers(&self) -> &SpiRegisters { &*self.mmio_address }
        //!
        //!     fn get_clock(&self) -> &ExampleClock { self.clock }
        //!
        //!     fn before_peripheral_access(&self, clock: &ExampleClock, _registers: &SpiRegisters) {
        //!         clock.enable();
        //!     }
        //!
        //!     fn after_peripheral_access(&self, clock: &ExampleClock, _registers: &SpiRegisters) {
        //!         if !self.busy {
        //!             clock.disable();
        //!         }
        //!     }
        //! }
        //! ```
        use crate::ClockInterface;
        /// A structure encapsulating a peripheral should implement this trait.
        pub trait PeripheralManagement<C> where C: ClockInterface {
            type
            RegisterType;
            /// How to get a reference to the physical hardware registers (the MMIO struct).
            fn get_registers(&self)
            -> &Self::RegisterType;
            /// Which clock feeds this peripheral.
            ///
            /// For peripherals with no clock, use `&::kernel::platform::NO_CLOCK_CONTROL`.
            fn get_clock(&self)
            -> &C;
            /// Called before peripheral access.
            ///
            /// Responsible for ensure the periphal can be safely accessed, e.g. that
            /// its clock is powered on.
            fn before_peripheral_access(&self, _: &C, _: &Self::RegisterType);
            /// Called after periphal access.
            ///
            /// Currently used primarily for power management to check whether the
            /// peripheral can be powered off.
            fn after_peripheral_access(&self, _: &C, _: &Self::RegisterType);
        }
        /// Structures encapsulating periphal hardware (those implementing the
        /// PeripheralManagement trait) should instantiate an instance of this
        /// method to accesss memory mapped registers.
        ///
        /// ```
        /// # use kernel::common::cells::VolatileCell;
        /// # use kernel::common::peripherals::PeripheralManager;
        /// # use kernel::common::StaticRef;
        /// # pub struct PeripheralRegisters { control: VolatileCell<u32> };
        /// # pub struct PeripheralHardware { mmio_address: StaticRef<PeripheralRegisters> };
        /// impl PeripheralHardware {
        ///     fn example(&self) {
        ///         let peripheral = &PeripheralManager::new(self);
        ///         peripheral.registers.control.set(0x1);
        ///     }
        /// }
        /// # use kernel::common::peripherals::PeripheralManagement;
        /// # use kernel::NoClockControl;
        /// # impl PeripheralManagement<NoClockControl> for PeripheralHardware {
        /// #     type RegisterType = PeripheralRegisters;
        ///
        /// #     fn get_registers(&self) -> &PeripheralRegisters {
        /// #         &*self.mmio_address
        /// #     }
        /// #     fn get_clock(&self) -> &NoClockControl { unsafe { &kernel::NO_CLOCK_CONTROL } }
        /// #     fn before_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        /// #     fn after_peripheral_access(&self, _c: &NoClockControl, _r: &Self::RegisterType) {}
        /// # }
        /// ```
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
        //! Interface for queue structure.
        pub trait Queue<T> {
            /// Returns true if there are any items in the queue, false otherwise.
            fn has_elements(&self)
            -> bool;
            /// Returns true if the queue is full, false otherwise.
            fn is_full(&self)
            -> bool;
            /// Returns how many elements are in the queue.
            fn len(&self)
            -> usize;
            /// Add a new element to the back of the queue.
            fn enqueue(&mut self, val: T)
            -> bool;
            /// Remove the element from the front of the queue.
            fn dequeue(&mut self)
            -> Option<T>;
            /// Remove all elements from the ring buffer.
            fn empty(&mut self);
            /// Retains only the elements that satisfy the predicate.
            fn retain<F>(&mut self, f: F)
            where
            F: FnMut(&T)
            ->
            bool;
        }
    }
    pub mod ring_buffer {
        //! Implementation of a ring buffer.
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
            /// Returns the number of elements that can be enqueued until the ring buffer is full.
            pub fn available_len(&self) -> usize {
                self.ring.len().saturating_sub(1 + queue::Queue::len(self))
            }
            /// Returns up to 2 slices that together form the contents of the ring buffer.
            ///
            /// Returns:
            /// - `(None, None)` if the buffer is empty.
            /// - `(Some(slice), None)` if the head is before the tail (therefore all the contents is
            /// contiguous).
            /// - `(Some(left), Some(right))` if the head is after the tail. In that case, the logical
            /// contents of the buffer is `[left, right].concat()` (although physically the "left" slice is
            /// stored after the "right" slice).
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
        //! Utility macros including `static_init!`.
        /// Allocates a global array of static size to initialize data structures.
        ///
        /// The global array is initially set to zero. When this macro is hit, it will
        /// initialize the array to the value given and return a `&'static mut`
        /// reference to it.
        ///
        /// If `std::mem::size_of<T>` ever becomes a `const` function then `static_init`
        /// will be optimized to save up to a word of memory for every use.
        ///
        /// # Safety
        ///
        /// As this macro will write directly to a global area without acquiring a lock
        /// or similar, calling this macro is inherently unsafe. The caller should take
        /// care to never call the code that initializes this buffer twice, as doing so
        /// will overwrite the value from first allocation without running its
        /// destructor.
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
        /// Allocates space in the kernel image for on-chip non-volatile storage.
        ///
        /// Storage volumes are placed after the kernel code and before relocated
        /// variables (those copied into RAM on boot). They are placed in
        /// a section called `.storage`.
        ///
        /// Non-volatile storage abstractions can then refer to the block of
        /// allocate flash in terms of the name of the volume. For example,
        ///
        /// `storage_volume(LOG, 32);`
        ///
        /// will allocate 32kB of space in the flash and define a symbol LOG
        /// at the start address of that flash region. The intention is that
        /// storage abstractions can then be passed this address and size to
        /// initialize their state. The linker script kernel_layout.ld makes
        /// sure that the .storage section is aligned on a 512-byte boundary
        /// and the next section is aligned as well.
        #[macro_export]
        macro_rules! storage_volume {
            ($ N : ident, $ kB : expr) =>
            {
                # [link_section = ".storage"] # [used] pub static $ N :
                [u8 ; $ kB * 1024] = [0x00 ; $ kB * 1024] ;
            } ;
        }
        /// Create an object with the given capability.
        ///
        /// ```ignore
        /// use kernel::capabilities::ProcessManagementCapability;
        /// use kernel;
        ///
        /// let process_mgmt_cap = create_capability!(ProcessManagementCapability);
        /// ```
        ///
        /// This helper macro can only be called in an `unsafe` block, and is used by
        /// trusted code to generate a capability that it can either use or pass to
        /// another module.
        #[macro_export]
        macro_rules! create_capability {
            ($ T : ty) =>
            { { struct Cap ; unsafe impl $ T for Cap { } Cap } ; } ;
        }
    }
    mod static_ref {
        //! Wrapper type for safe pointers to static memory.
        use core::ops::Deref;
        /// A pointer to statically allocated mutable data such as memory mapped I/O
        /// registers.
        ///
        /// This is a simple wrapper around a raw pointer that encapsulates an unsafe
        /// dereference in a safe manner. It serve the role of creating a `&'static T`
        /// given a raw address and acts similarly to `extern` definitions, except
        /// `StaticRef` is subject to module and crate boundaries, while `extern`
        /// definitions can be imported anywhere.
        pub struct StaticRef<T> {
            ptr: *const T,
        }
        #[automatically_derived]
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
            /// Create a new `StaticRef` from a raw pointer
            ///
            /// ## Safety
            ///
            /// Callers must pass in a reference to statically allocated memory which
            /// does not overlap with other values.
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
    /// Create a "fake" module inside of `common` for all of the Tock `Cell` types.
    ///
    /// To use `TakeCell`, for example, users should use:
    ///
    ///     use kernel::common::cells::TakeCell;
    pub mod cells {
        pub use tock_cells::map_cell::MapCell;
        pub use tock_cells::numeric_cell_ext::NumericCellExt;
        pub use tock_cells::optional_cell::OptionalCell;
        pub use tock_cells::take_cell::TakeCell;
        pub use tock_cells::volatile_cell::VolatileCell;
    }
}
pub mod component {
    //! Components extend the functionality of the Tock kernel through a
    //! simple factory method interface.
    /// A component encapsulates peripheral-specific and capsule-specific
    /// initialization for the Tock OS kernel in a factory method,
    /// which reduces repeated code and simplifies the boot sequence.
    ///
    /// The `Component` trait encapsulates all of the initialization and
    /// configuration of a kernel extension inside the `finalize()` function
    /// call. The `Output` type defines what type this component generates.
    /// Note that instantiating a component does not necessarily
    /// instantiate the underlying `Output` type; instead, it is typically
    /// instantiated in the `finalize()` method. If instantiating and
    /// initializing the `Output` type requires parameters, these should be
    /// passed in the component's `new()` function.
    pub trait Component {
        /// An optional type to specify the chip or board specific static memory
        /// that a component needs to setup the output object(s). This is the memory
        /// that `static_init!()` would normally setup, but generic components
        /// cannot setup static buffers for types which are chip-dependent, so those
        /// buffers have to be passed in manually, and the `StaticInput` type makes
        /// this possible.
        type
        StaticInput
        =
        ();
        /// The type (e.g., capsule, peripheral) that this implementation
        /// of Component produces via `finalize()`. This is typically a
        /// static reference (`&'static`).
        type
        Output;
        /// A factory method that returns an instance of the Output type of this
        /// Component implementation. This factory method may only be called once
        /// per Component instance. Used in the boot sequence to instantiate and
        /// initialize part of the Tock kernel. Some components need to use the
        /// `static_memory` argument to allow the board initialization code to pass
        /// in references to static memory that the component will use to setup the
        /// Output type object.
        unsafe fn finalize(&mut self, static_memory: Self::StaticInput)
        -> Self::Output;
    }
}
pub mod debug {
    //! Support for in-kernel debugging.
    //!
    //! For printing, this module uses an internal buffer to write the strings into.
    //! If you are writing and the buffer fills up, you can make the size of
    //! `output_buffer` larger.
    //!
    //! Before debug interfaces can be used, the board file must assign them hardware:
    //!
    //! ```ignore
    //! kernel::debug::assign_gpios(
    //!     Some(&sam4l::gpio::PA[13]),
    //!     Some(&sam4l::gpio::PA[15]),
    //!     None,
    //!     );
    //!
    //! let kc = static_init!(
    //!     capsules::console::App,
    //!     capsules::console::App::default());
    //! kernel::debug::assign_console_driver(Some(hail.console), kc);
    //! ```
    //!
    //! Example
    //! -------
    //!
    //! ```no_run
    //! # use kernel::{debug, debug_gpio, debug_verbose};
    //! # fn main() {
    //! # let i = 42;
    //! debug!("Yes the code gets here with value {}", i);
    //! debug_verbose!("got here"); // includes message count, file, and line
    //! debug_gpio!(0, toggle); // Toggles the first debug GPIO
    //! # }
    //! ```
    //!
    //! ```text
    //! Yes the code gets here with value 42
    //! TOCK_DEBUG(0): /tock/capsules/src/sensys.rs:24: got here
    //! ```
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
    /// Tock default panic routine.
    ///
    /// **NOTE:** The supplied `writer` must be synchronous.
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
    /// Generic panic entry.
    ///
    /// This opaque method should always be called at the beginning of a board's
    /// panic method to allow hooks for any core kernel cleanups that may be
    /// appropriate.
    pub unsafe fn panic_begin(nop: &dyn Fn()) {
        for _ in 0..200000 { nop(); }
    }
    /// Lightweight prints about the current panic and kernel version.
    ///
    /// **NOTE:** The supplied `writer` must be synchronous.
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
    /// More detailed prints about all processes.
    ///
    /// **NOTE:** The supplied `writer` must be synchronous.
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
    /// Blinks a recognizable pattern forever.
    ///
    /// If a multi-color LED is used for the panic pattern, it is
    /// advised to turn off other LEDs before calling this method.
    ///
    /// Generally, boards should blink red during panic if possible,
    /// otherwise choose the 'first' or most prominent LED. Some
    /// boards may find it appropriate to blink multiple LEDs (e.g.
    /// one on the top and one on the bottom), thus this method
    /// accepts an array, however most will only need one.
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
    /// In-kernel gpio debugging, accepts any GPIO HIL method
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
    /// Wrapper type that we need a mutable reference to for the core::fmt::Write
    /// interface.
    pub struct DebugWriterWrapper {
        dw: MapCell<&'static DebugWriter>,
    }
    /// Main type that we need an immutable reference to so we can share it with
    /// the UART provider and this debug module.
    pub struct DebugWriter {
        uart: &'static dyn hil::uart::Transmit<'static>,
        output_buffer: TakeCell<'static, [u8]>,
        internal_buffer: TakeCell<'static, RingBuffer<'static, u8>>,
        count: Cell<usize>,
    }
    /// Static variable that holds the kernel's reference to the debug tool. This is
    /// needed so the debug!() macros have a reference to the object to use.
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
    /// Function used by board main.rs to set a reference to the writer.
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
        /// Write as many of the bytes from the internal_buffer to the output
        /// mechanism as possible.
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
    /// Pass through functions.
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
    /// In-kernel `println()` debugging.
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
    /// In-kernel `println()` debugging with filename and line numbers.
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
    //! Public traits for interfaces between Tock components.
    pub mod adc {
        //! Interfaces for analog to digital converter peripherals.
        use crate::returncode::ReturnCode;
        /// Simple interface for reading an ADC sample on any channel.
        pub trait Adc {
            /// The chip-dependent type of an ADC channel.
            type
            Channel;
            /// Request a single ADC sample on a particular channel.
            /// Used for individual samples that have no timing requirements.
            /// All ADC samples will be the raw ADC value left-justified in the u16.
            fn sample(&self, channel: &Self::Channel)
            -> ReturnCode;
            /// Request repeated ADC samples on a particular channel.
            /// Callbacks will occur at the given frequency with low jitter and can be
            /// set to any frequency supported by the chip implementation. However
            /// callbacks may be limited based on how quickly the system can service
            /// individual samples, leading to missed samples at high frequencies.
            /// All ADC samples will be the raw ADC value left-justified in the u16.
            fn sample_continuous(&self, channel: &Self::Channel,
                                 frequency: u32)
            -> ReturnCode;
            /// Stop a sampling operation.
            /// Can be used to stop any simple or high-speed sampling operation. No
            /// further callbacks will occur.
            fn stop_sampling(&self)
            -> ReturnCode;
            /// Function to ask the ADC how many bits of resolution are in the samples
            /// it is returning.
            fn get_resolution_bits(&self)
            -> usize;
            /// Function to ask the ADC what reference voltage it used when taking the
            /// samples. This allows the user of this interface to calculate an actual
            /// voltage from the ADC reading.
            ///
            /// The returned reference voltage is in millivolts, or `None` if unknown.
            fn get_voltage_reference_mv(&self)
            -> Option<usize>;
        }
        /// Trait for handling callbacks from simple ADC calls.
        pub trait Client {
            /// Called when a sample is ready.
            fn sample_ready(&self, sample: u16);
        }
        /// Interface for continuously sampling at a given frequency on a channel.
        /// Requires the AdcSimple interface to have been implemented as well.
        pub trait AdcHighSpeed: Adc {
            /// Start sampling continuously into buffers.
            /// Samples are double-buffered, going first into `buffer1` and then into
            /// `buffer2`. A callback is performed to the client whenever either buffer
            /// is full, which expects either a second buffer to be sent via the
            /// `provide_buffer` call. Length fields correspond to the number of
            /// samples that should be collected in each buffer. If an error occurs,
            /// the buffers will be returned.
            ///
            /// All ADC samples will be the raw ADC value left-justified in the u16.
            fn sample_highspeed(&self, channel: &Self::Channel,
                                frequency: u32, buffer1: &'static mut [u16],
                                length1: usize, buffer2: &'static mut [u16],
                                length2: usize)
            ->
                (ReturnCode, Option<&'static mut [u16]>,
                 Option<&'static mut [u16]>);
            /// Provide a new buffer to fill with the ongoing `sample_continuous`
            /// configuration.
            /// Expected to be called in a `buffer_ready` callback. Note that if this
            /// is not called before the second buffer is filled, samples will be
            /// missed. Length field corresponds to the number of samples that should
            /// be collected in the buffer. If an error occurs, the buffer will be
            /// returned.
            ///
            /// All ADC samples will be the raw ADC value left-justified in the u16.
            fn provide_buffer(&self, buf: &'static mut [u16], length: usize)
            -> (ReturnCode, Option<&'static mut [u16]>);
            /// Reclaim ownership of buffers.
            /// Can only be called when the ADC is inactive, which occurs after a
            /// successful `stop_sampling`. Used to reclaim buffers after a sampling
            /// operation is complete. Returns success if the ADC was inactive, but
            /// there may still be no buffers that are `some` if the driver had already
            /// returned all buffers.
            ///
            /// All ADC samples will be the raw ADC value left-justified in the u16.
            fn retrieve_buffers(&self)
            ->
                (ReturnCode, Option<&'static mut [u16]>,
                 Option<&'static mut [u16]>);
        }
        /// Trait for handling callbacks from high-speed ADC calls.
        pub trait HighSpeedClient {
            /// Called when a buffer is full.
            /// The length provided will always be less than or equal to the length of
            /// the buffer. Expects an additional call to either provide another buffer
            /// or stop sampling
            fn samples_ready(&self, buf: &'static mut [u16], length: usize);
        }
    }
    pub mod analog_comparator {
        //! Interface for direct control of the analog comparators.
        use crate::returncode::ReturnCode;
        pub trait AnalogComparator {
            /// The chip-dependent type of an analog comparator channel.
            type
            Channel;
            /// Do a single comparison of two inputs, depending on the AC chosen. Output
            /// will be True (1) when one is higher than the other, and False (0)
            /// otherwise.  Specifically, the output is True when Vp > Vn (Vin positive
            /// > Vin negative), and False if Vp < Vn.
            fn comparison(&self, channel: &Self::Channel)
            -> bool;
            /// Start interrupt-based comparison for the chosen channel (e.g. channel 1
            /// for AC1). This will make it listen and send an interrupt as soon as
            /// Vp > Vn.
            fn start_comparing(&self, channel: &Self::Channel)
            -> ReturnCode;
            /// Stop interrupt-based comparison for the chosen channel.
            fn stop_comparing(&self, channel: &Self::Channel)
            -> ReturnCode;
        }
        pub trait Client {
            /// Fires when handle_interrupt is called, returning the channel on which
            /// the interrupt occurred.
            fn fired(&self, _: usize);
        }
    }
    pub mod ble_advertising {
        //! Bluetooth Low Energy HIL
        //!
        //! ```text
        //! Application
        //!
        //!           +------------------------------------------------+
        //!           | Applications                                   |
        //!           +------------------------------------------------+
        //!
        //! ```
        //!
        //! ```text
        //! Host
        //!
        //!           +------------------------------------------------+
        //!           | Generic Access Profile                         |
        //!           +------------------------------------------------+
        //!
        //!           +------------------------------------------------+
        //!           | Generic Attribute Profile                      |
        //!           +------------------------------------------------+
        //!
        //!           +--------------------+      +-------------------+
        //!           | Attribute Protocol |      | Security Manager  |
        //!           +--------------------+      +-------------------+
        //!
        //!           +-----------------------------------------------+
        //!           | Logical Link and Adaptation Protocol          |
        //!           +-----------------------------------------------+
        //!
        //! ```
        //!
        //! ```text
        //! Controller
        //!
        //!           +--------------------------------------------+
        //!           | Host Controller Interface                  |
        //!           +--------------------------------------------+
        //!
        //!           +------------------+      +------------------+
        //!           | Link Layer       |      | Direct Test Mode |
        //!           +------------------+      +------------------+
        //!
        //!           +--------------------------------------------+
        //!           | Physical Layer                             |
        //!           +--------------------------------------------+
        //!
        //! ```
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
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for RadioChannel { }
        #[automatically_derived]
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
        //! Interface for CRC computation.
        use crate::returncode::ReturnCode;
        /// CRC algorithms
        ///
        /// In all cases, input bytes are bit-reversed (i.e., consumed from LSB to MSB.)
        ///
        /// Algorithms prefixed with `Sam4L` are native to that chip and thus require
        /// no software post-processing on platforms using it.
        ///
        pub enum CrcAlg {

            /// Polynomial 0x04C11DB7, output reversed then inverted ("CRC-32")
            Crc32,

            /// Polynomial 0x1EDC6F41, output reversed then inverted ("CRC-32C" / "Castagnoli")
            Crc32C,

            /// Polynomial 0x1021, no output post-processing
            Sam4L16,

            /// Polynomial 0x04C11DB7, no output post-processing
            Sam4L32,

            /// Polynomial 0x1EDC6F41, no output post-processing
            Sam4L32C,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for CrcAlg { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for CrcAlg {
            #[inline]
            fn clone(&self) -> CrcAlg { { *self } }
        }
        pub trait CRC {
            /// Initiate a CRC calculation
            fn compute(&self, data: &[u8], _: CrcAlg)
            -> ReturnCode;
            /// Disable the CRC unit until compute() is next called
            fn disable(&self);
        }
        pub trait Client {
            /// Receive the successful result of a CRC calculation
            fn receive_result(&self, _: u32);
        }
    }
    pub mod dac {
        //! Interface for digital to analog converters.
        use crate::returncode::ReturnCode;
        /// Simple interface for using the DAC.
        pub trait DacChannel {
            /// Initialize and enable the DAC.
            fn initialize(&self)
            -> ReturnCode;
            /// Set the DAC output value.
            fn set_value(&self, value: usize)
            -> ReturnCode;
        }
    }
    pub mod eic {
        //! Interface for external interrupt controller.
        //!
        //! The External Interrupt Controller (EIC) allows pins to be configured as
        //! external interrupts. Each external interrupt has its own interrupt request
        //! and can be individually masked. Each external interrupt can generate an
        //! interrupt on rising or falling edge, or high or low level.
        //! Every interrupt pin can also be configured to be asynchronous, in order to
        //! wake-up the part from sleep modes where the CLK_SYNC clock has been disabled.
        //!
        //! A basic use case:
        //! A user button is configured for falling edge trigger and async mode.
        /// Enum for selecting which edge to trigger interrupts on.
        pub enum InterruptMode {
            RisingEdge,
            FallingEdge,
            HighLevel,
            LowLevel,
        }
        #[automatically_derived]
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
        /// Interface for EIC.
        pub trait ExternalInterruptController {
            /// The chip-dependent type of an EIC line. Number of lines available depends on the chip.
            type
            Line;
            /// Enables external interrupt on the given 'line'
            /// In asychronous mode, all edge interrupts will be
            /// interpreted as level interrupts and the filter is disabled.
            fn line_enable(&self, line: &Self::Line,
                           interrupt_mode: InterruptMode);
            /// Disables external interrupt on the given 'line'
            fn line_disable(&self, line: &Self::Line);
        }
        /// Interface for users of EIC. In order
        /// to execute interrupts, the user must implement
        /// this `Client` interface.
        pub trait Client {
            /// Called when an interrupt occurs.
            fn fired(&self);
        }
    }
    pub mod entropy {
        //! Interfaces for accessing an entropy source.
        //!
        //! An entropy source produces random bits that are computationally
        //! intractable to guess, even if the complete history of generated
        //! bits and state of the device can be observed. Entropy sources must
        //! generate bits from an underlying physical random process, such as
        //! thermal noise, radiation, avalanche noise, or circuit
        //! instability. These bits of entropy can be used to seed
        //! cryptographically strong random number generators. Because high-quality
        //! entropy is critical for security and these APIs provide entropy,
        //! it is important to understand all of these requirements before
        //! implementing these traits. Otherwise you may subvert the security
        //! of the operating system.
        //!
        //! _Entropy_: Entropy bits generated by this trait MUST have very high
        //! entropy, i.e. 1 bit of entropy per generated bit. If the underlying
        //! source generates <1 bit of entropy per bit, these low-entropy bits
        //! SHOULD be mixed and combined with a cryptographic hash function.
        //! A good, short reference on the difference between entropy and
        //! randomness as well as guidelines for high-entropy sources is
        //! Recommendations for Randomness in the Operating System: How to
        //! Keep Evil Children Out of Your Pool and Other Random Facts,
        //! Corrigan-Gibbs et al., HotOS 2015.
        //!
        //! The interface is designed to work well with entropy generators
        //! that may not have values ready immediately. This is important when
        //! generating numbers from a low-bandwidth hardware entropy source
        //! generator or when virtualized among many consumers.
        //!
        //! Entropy is yielded to a Client as an `Iterator` which only
        //! terminates when no more entropy is currently available. Clients
        //! can request more entropy if needed and will be called again when
        //! more is available.
        //!
        //! # Example
        //!
        //! The following example is a simple capsule that prints out entropy
        //! once a second using the `Alarm` and `Entropy` traits.
        //!
        //! ```
        //! use kernel::hil;
        //! use kernel::hil::entropy::Entropy32;
        //! use kernel::hil::entropy::Client32;
        //! use kernel::hil::time::Alarm;
        //! use kernel::hil::time::Frequency;
        //! use kernel::hil::time::AlarmClient;
        //! use kernel::ReturnCode;
        //!
        //! struct EntropyTest<'a, A: 'a + Alarm<'a>> {
        //!     entropy: &'a Entropy32 <'a>,
        //!     alarm: &'a A
        //! }
        //!
        //! impl<'a, A: Alarm<'a>> EntropyTest<'a, A> {
        //!     pub fn initialize(&self) {
        //!         let interval = 1 * <A::Frequency>::frequency();
        //!         let tics = self.alarm.now().wrapping_add(interval);
        //!         self.alarm.set_alarm(tics);
        //!     }
        //! }
        //!
        //! impl<'a, A: Alarm<'a>> AlarmClient for EntropyTest<'a, A> {
        //!     fn fired(&self) {
        //!         self.entropy.get();
        //!     }
        //! }
        //!
        //! impl<'a, A: Alarm<'a>> Client32 for EntropyTest<'a, A> {
        //!     fn entropy_available(&self,
        //!                          entropy: &mut Iterator<Item = u32>,
        //!                          error: ReturnCode) -> hil::entropy::Continue {
        //!         match entropy.next() {
        //!             Some(val) => {
        //!                 println!("Entropy {}", val);
        //!                 let interval = 1 * <A::Frequency>::frequency();
        //!                 let tics = self.alarm.now().wrapping_add(interval);
        //!                 self.alarm.set_alarm(tics);
        //!                 hil::entropy::Continue::Done
        //!             },
        //!             None => hil::entropy::Continue::More
        //!         }
        //!     }
        //! }
        //! ```
        use crate::returncode::ReturnCode;
        /// Denotes whether the [Client](trait.Client.html) wants to be notified when
        /// `More` randomness is available or if they are `Done`
        pub enum Continue {

            /// More randomness is required.
            More,

            /// No more randomness required.
            Done,
        }
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
        #[automatically_derived]
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
        /// Generic interface for a 32-bit entropy source.
        ///
        /// Implementors should assume the client implements the
        /// [Client](trait.Client32.html) trait.
        pub trait Entropy32<'a> {
            /// Initiate the aquisition of entropy.
            ///
            /// There are three valid return values:
            ///   - SUCCESS: a `entropy_available` callback will be called in
            ///     the future when entropy is available.
            ///   - FAIL: a `entropy_available` callback will not be called in
            ///     the future, because entropy cannot be generated. This
            ///     is a general failure condition.
            ///   - EOFF: a `entropy_available` callback will not be called in
            ///     the future, because the random number generator is off/not
            ///     powered.
            fn get(&self)
            -> ReturnCode;
            /// Cancel acquisition of entropy.
            ///
            /// There are three valid return values:
            ///   - SUCCESS: an outstanding request from `get` has been cancelled,
            ///     or there was no outstanding request. No `entropy_available`
            ///     callback will be issued.
            ///   - FAIL: There will be a `entropy_available` callback, which
            ///     may or may not return an error code.
            fn cancel(&self)
            -> ReturnCode;
            /// Set the client to receive `entropy_available` callbacks.
            fn set_client(&'a self, _: &'a dyn Client32);
        }
        /// An [Entropy32](trait.Entropy32.html) client
        ///
        /// Clients of an [Entropy32](trait.Entropy32.html) must implement this trait.
        pub trait Client32 {
            /// Called by the (Entropy)[trait.Entropy32.html] when there is entropy
            /// available.
            ///
            /// `entropy` in an `Iterator` of available entropy. The amount of
            /// entropy available may increase if `entropy` is not consumed
            /// quickly so clients should not rely on iterator termination to
            /// finish consuming entropy.
            ///
            /// The client returns either `Continue::More` if the iterator did
            /// not have enough entropy (indicating another
            /// `entropy_available` callback is requested) and the client
            /// would like to be called again when more is available, or
            /// `Continue::Done`, which indicates `entropy_available` should
            /// not be called again until `get()` is called.
            ///
            /// If `entropy_available` is triggered after a call to `cancel()`
            /// then error MUST be ECANCEL and `entropy` MAY contain bits of
            /// entropy.
            fn entropy_available(&self,
                                 entropy: &mut dyn Iterator<Item = u32>,
                                 error: ReturnCode)
            -> Continue;
        }
        /// An 8-bit entropy generator.
        ///
        /// Implementors should assume the client implements the
        /// [Client8](trait.Client8.html) trait.
        pub trait Entropy8<'a> {
            /// Initiate the acquisition of new entropy.
            ///
            /// There are three valid return values:
            ///   - SUCCESS: a `entropy_available` callback will be called in
            ///     the future when entropy is available.
            ///   - FAIL: a `entropy_available` callback will not be called in
            ///     the future, because entropy cannot be generated. This
            ///     is a general failure condition.
            ///   - EOFF: a `entropy_available` callback will not be called in
            ///     the future, because the entropy generator is off/not
            ///     powered.
            fn get(&self)
            -> ReturnCode;
            /// Cancel acquisition of entropy.
            ///
            /// There are three valid return values:
            ///   - SUCCESS: an outstanding request from `get` has been cancelled,
            ///     or there was no outstanding request. No `entropy_available`
            ///     callback will be issued.
            ///   - FAIL:: There will be a `entropy_available` callback, which
            ///     may or may not return an error code.
            fn cancel(&self)
            -> ReturnCode;
            /// Set the client to receive `entropy_available` callbacks.
            fn set_client(&'a self, _: &'a dyn Client8);
        }
        /// An [Entropy8](trait.Entropy8.html) client
        ///
        /// Clients of an [Entropy8](trait.Entropy8.html) must implement this trait.
        pub trait Client8 {
            /// Called by the (Entropy)[trait.Entropy8.html] when there are
            /// one or more bytes of entropy available.
            ///
            /// `entropy` in an `Iterator` of available entropy. The amount of
            /// entropy available may increase if `entropy` is not consumed
            /// quickly so clients should not rely on iterator termination to
            /// finish consuming entropy.
            ///
            /// The client returns either `Continue::More` if the iterator did
            /// not have enough entropy (indicating another
            /// `entropy_available` callback is requested) and the client
            /// would like to be called again when more is available, or
            /// `Continue::Done`, which indicates `entropy_available` should
            /// not be called again until `get()` is called.
            ///
            /// If `entropy_available` is triggered after a call to `cancel()`
            /// then error MUST be ECANCEL and `entropy` MAY contain bits of
            /// entropy.
            fn entropy_available(&self, entropy: &mut dyn Iterator<Item = u8>,
                                 error: ReturnCode)
            -> Continue;
        }
    }
    pub mod flash {
        //! Interface for reading, writing, and erasing flash storage pages.
        //!
        //! Operates on single pages. The page size is set by the associated type
        //! `page`. Here is an example of a page type and implementation of this trait:
        //!
        //! ```rust
        //! # #![feature(const_fn)]
        //! use core::ops::{Index, IndexMut};
        //!
        //! use kernel::hil;
        //! use kernel::ReturnCode;
        //!
        //! // Size in bytes
        //! const PAGE_SIZE: u32 = 1024;
        //!
        //! struct NewChipPage(pub [u8; PAGE_SIZE as usize]);
        //!
        //! impl NewChipPage {
        //!     pub const fn new() -> NewChipPage {
        //!         NewChipPage([0; PAGE_SIZE as usize])
        //!     }
        //!
        //!     fn len(&self) -> usize {
        //!         self.0.len()
        //!     }
        //! }
        //!
        //! impl Index<usize> for NewChipPage {
        //!     type Output = u8;
        //!
        //!     fn index(&self, idx: usize) -> &u8 {
        //!         &self.0[idx]
        //!     }
        //! }
        //!
        //! impl IndexMut<usize> for NewChipPage {
        //!     fn index_mut(&mut self, idx: usize) -> &mut u8 {
        //!         &mut self.0[idx]
        //!     }
        //! }
        //!
        //! impl AsMut<[u8]> for NewChipPage {
        //!     fn as_mut(&mut self) -> &mut [u8] {
        //!         &mut self.0
        //!     }
        //! }
        //!
        //! struct NewChipStruct {};
        //!
        //! impl<'a, C> hil::flash::HasClient<'a, C> for NewChipStruct {
        //!     fn set_client(&'a self, client: &'a C) { }
        //! }
        //!
        //! impl hil::flash::Flash for NewChipStruct {
        //!     type Page = NewChipPage;
        //!
        //!     fn read_page(&self, page_number: usize, buf: &'static mut Self::Page) -> ReturnCode { ReturnCode::FAIL }
        //!     fn write_page(&self, page_number: usize, buf: &'static mut Self::Page) -> ReturnCode { ReturnCode::FAIL }
        //!     fn erase_page(&self, page_number: usize) -> ReturnCode { ReturnCode::FAIL }
        //! }
        //! ```
        //!
        //! A user of this flash interface might look like:
        //!
        //! ```rust
        //! use kernel::common::cells::TakeCell;
        //! use kernel::hil;
        //!
        //! pub struct FlashUser<'a, F: hil::flash::Flash + 'static> {
        //!     driver: &'a F,
        //!     buffer: TakeCell<'static, F::Page>,
        //! }
        //!
        //! impl<'a, F: hil::flash::Flash> FlashUser<'a, F> {
        //!     pub fn new(driver: &'a F, buffer: &'static mut F::Page) -> FlashUser<'a, F> {
        //!         FlashUser {
        //!             driver: driver,
        //!             buffer: TakeCell::new(buffer),
        //!         }
        //!     }
        //! }
        //!
        //! impl<'a, F: hil::flash::Flash> hil::flash::Client<F> for FlashUser<'a, F> {
        //!     fn read_complete(&self, buffer: &'static mut F::Page, error: hil::flash::Error) {}
        //!     fn write_complete(&self, buffer: &'static mut F::Page, error: hil::flash::Error) { }
        //!     fn erase_complete(&self, error: hil::flash::Error) {}
        //! }
        //! ```
        use crate::returncode::ReturnCode;
        /// Flash errors returned in the callbacks.
        pub enum Error {

            /// Success.
            CommandComplete,

            /// An error occurred during the flash operation.
            FlashError,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
        #[automatically_derived]
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
            /// Set the client for this flash peripheral. The client will be called
            /// when operations complete.
            fn set_client(&'a self, client: &'a C);
        }
        /// A page of writable persistent flash memory.
        pub trait Flash {
            /// Type of a single flash page for the given implementation.
            type
            Page: AsMut<[u8]>;
            /// Read a page of flash into the buffer.
            fn read_page(&self, page_number: usize,
                         buf: &'static mut Self::Page)
            -> ReturnCode;
            /// Write a page of flash from the buffer.
            fn write_page(&self, page_number: usize,
                          buf: &'static mut Self::Page)
            -> ReturnCode;
            /// Erase a page of flash.
            fn erase_page(&self, page_number: usize)
            -> ReturnCode;
        }
        /// Implement `Client` to receive callbacks from `Flash`.
        pub trait Client<F: Flash> {
            /// Flash read complete.
            fn read_complete(&self, read_buffer: &'static mut F::Page,
                             error: Error);
            /// Flash write complete.
            fn write_complete(&self, write_buffer: &'static mut F::Page,
                              error: Error);
            /// Flash erase complete.
            fn erase_complete(&self, error: Error);
        }
    }
    pub mod gpio {
        use crate::common::cells::OptionalCell;
        use crate::ReturnCode;
        use core::cell::Cell;
        /// Enum for configuring any pull-up or pull-down resistors on the GPIO pin.
        pub enum FloatingState { PullUp, PullDown, PullNone, }
        #[automatically_derived]
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
        /// Enum for selecting which edge to trigger interrupts on.
        pub enum InterruptEdge { RisingEdge, FallingEdge, EitherEdge, }
        #[automatically_derived]
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
        /// Enum for which state the pin is in. Some MCUs can support Input/Output pins,
        /// so this is a valid option. `Function` means the pin has been configured to
        /// a special function. Determining which function it outside the scope of the HIL,
        /// and should instead use a chip-specific API.
        pub enum Configuration {

            /// Cannot be read or written or used; effectively inactive.
            LowPower,

            /// Calls to the `Input` trait are valid.
            Input,

            /// Calls to the `Output` trait are valid.
            Output,

            /// Calls to both the `Input` and `Output` traits are valid.
            InputOutput,

            /// Chip-specific, requires chip-specific API for more detail,
            Function,

            /// In a state not covered by other values.
            Other,
        }
        #[automatically_derived]
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
        /// The Pin trait allows a pin to be used as either input
        /// or output and to be configured.
        pub trait Pin: Input + Output + Configure { }
        /// The InterruptPin trait allows a pin to be used as either
        /// input or output and also to source interrupts.
        pub trait InterruptPin: Pin + Interrupt { }
        /// The InterruptValuePin trait allows a pin to be used as
        /// either input or output and also to source interrupts which
        /// pass a value.
        pub trait InterruptValuePin: Pin + InterruptWithValue { }
        /// Control and configure a GPIO pin.
        pub trait Configure {
            /// Return the current pin configuration.
            fn configuration(&self)
            -> Configuration;
            /// Make the pin an output, returning the current configuration,
            /// which should be either `Configuration::Output` or
            /// `Configuration::InputOutput`.
            fn make_output(&self)
            -> Configuration;
            /// Disable the pin as an output, returning the current configuration.
            fn disable_output(&self)
            -> Configuration;
            /// Make the pin an input, returning the current configuration,
            /// which should be ither `Configuration::Input` or
            /// `Configuration::InputOutput`.
            fn make_input(&self)
            -> Configuration;
            /// Disable the pin as an input, returning the current configuration.
            fn disable_input(&self)
            -> Configuration;
            /// Put a pin into its lowest power state, with no guarantees on
            /// if it is enabled or not. Implementations are free to use any
            /// state (e.g. input, output, disable, etc.) the hardware pin
            /// supports to ensure the pin is as low power as possible.
            /// Re-enabling the pin requires reconfiguring it (i.e. the state
            /// of its enabled configuration is not stored).
            fn deactivate_to_low_power(&self);
            /// Set the floating state of the pin.
            fn set_floating_state(&self, state: FloatingState);
            /// Return the current floating state of the pin.
            fn floating_state(&self)
            -> FloatingState;
            /// Return whether the pin is an input (reading from
            /// the Input trait will return valid results). Returns
            /// true if the pin is in Configuration::Input or
            /// Configuration::InputOutput.
            fn is_input(&self) -> bool {
                match self.configuration() {
                    Configuration::Input | Configuration::InputOutput => true,
                    _ => false,
                }
            }
            /// Return whether the pin is an output (writing to
            /// the Output trait will change the output of the pin).
            /// Returns true if the pin is in Configuration::Output or
            /// Configuration::InputOutput.
            fn is_output(&self) -> bool {
                match self.configuration() {
                    Configuration::Output | Configuration::InputOutput =>
                    true,
                    _ => false,
                }
            }
        }
        /// Configuration trait for pins that can be simultaneously
        /// input and output. Having this trait allows an implementation
        /// to statically verify this is possible.
        pub trait ConfigureInputOutput: Configure {
            /// Make the pin a simultaneously input and output; should always
            /// return `Configuration::InputOutput`.
            fn make_input_output(&self)
            -> Configuration;
            fn is_input_output(&self)
            -> bool;
        }
        pub trait Output {
            /// Set the GPIO pin high. If the pin is not an output or
            /// input/output, this call is ignored.
            fn set(&self);
            /// Set the GPIO pin low. If the pin is not an output or
            /// input/output, this call is ignored.
            fn clear(&self);
            /// Toggle the GPIO pin. If the pin was high, set it low. If
            /// the pin was low, set it high. If the pin is not an output or
            /// input/output, this call is ignored. Return the new value
            /// of the pin.
            fn toggle(&self)
            -> bool;
        }
        pub trait Input {
            /// Get the current state of an input GPIO pin. For an output
            /// pin, return the output; for an input pin, return the input;
            /// for disabled or function pins the value is undefined.
            fn read(&self)
            -> bool;
        }
        pub trait Interrupt: Input {
            /// Set the client for interrupt events.
            fn set_client(&self, client: &'static dyn Client);
            /// Enable an interrupt on the GPIO pin. This does not
            /// configure the pin except to enable an interrupt: it
            /// should be separately configured as an input, etc.
            fn enable_interrupts(&self, mode: InterruptEdge);
            /// Disable interrupts for the GPIO pin.
            fn disable_interrupts(&self);
            /// Return whether this interrupt is pending
            fn is_pending(&self)
            -> bool;
        }
        /// Interface for users of synchronous GPIO interrupts. In order
        /// to receive interrupts, the user must implement
        /// this `Client` interface.
        pub trait Client {
            /// Called when an interrupt occurs. The `identifier` will
            /// be the same value that was passed to `enable_interrupt()`
            /// when the interrupt was configured.
            fn fired(&self);
        }
        /// Interface that wraps an interrupt to pass a value when it
        /// triggers. The standard use case for this trait is when several
        /// interrupts call the same callback function and it needs to
        /// distinguish which one is calling it by giving each one a unique
        /// value.
        pub trait InterruptWithValue: Input {
            /// Set the client for interrupt events.
            fn set_client(&self, client: &'static dyn ClientWithValue);
            /// Enable an interrupt on the GPIO pin. This does not
            /// configure the pin except to enable an interrupt: it
            /// should be separately configured as an input, etc.
            /// Returns:
            ///    SUCCESS - the interrupt was set up properly
            ///    FAIL    - the interrupt was not set up properly; this is due to
            ///              not having an underlying interrupt source yet, i.e.
            ///              the struct is not yet fully initialized.
            fn enable_interrupts(&self, mode: InterruptEdge)
            -> ReturnCode;
            /// Disable interrupts for the GPIO pin.
            fn disable_interrupts(&self);
            /// Return whether this interrupt is pending
            fn is_pending(&self)
            -> bool;
            /// Set the value that will be passed to clients on an
            /// interrupt.
            fn set_value(&self, value: u32);
            /// Return the value that is passed to clients on an
            /// interrupt.
            fn value(&self)
            -> u32;
        }
        /// Interfaces for users of GPIO interrupts who handle many interrupts
        /// with the same function. The value passed in the callback allows the
        /// callback to distinguish which interrupt fired.
        pub trait ClientWithValue {
            fn fired(&self, value: u32);
        }
        /// Standard implementation of InterruptWithValue: handles an
        /// `gpio::Client::fired` and passes it up as a
        /// `gpio::ClientWithValue::fired`.
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
        //! Interface for GPIO pins that require split-phase operation to control.
        use crate::hil;
        use crate::returncode::ReturnCode;
        /// Interface for banks of asynchronous GPIO pins. GPIO pins are asynchronous
        /// when there is an asynchronous interface used to control them. The most
        /// common example is when using a GPIO extender on an I2C or SPI bus. With
        /// asynchronous GPIO functions, every config action results in an eventual
        /// callback function that indicates that the configuration has finished
        /// (unless the initial function call returns an error code, then no callback
        /// will be generated).
        ///
        /// Asynchronous GPIO pins are grouped into ports because it is assumed that
        /// the remote entity that is controlling the pins can control multiple pins.
        /// Typically, a port will be provided by a particular driver.
        ///
        /// The API for the Port mirrors the synchronous GPIO interface.
        pub trait Port {
            /// Try to disable a GPIO pin. This cannot be supported for all devices.
            fn disable(&self, pin: usize)
            -> ReturnCode;
            /// Configure a pin as an ouput GPIO.
            fn make_output(&self, pin: usize)
            -> ReturnCode;
            /// Configure a pin as an input GPIO. Not all FloatingMode settings may
            /// be supported by a given device.
            fn make_input(&self, pin: usize, mode: hil::gpio::FloatingState)
            -> ReturnCode;
            /// Get the state (0 or 1) of an input pin. The value will be returned
            /// via a callback.
            fn read(&self, pin: usize)
            -> ReturnCode;
            /// Toggle an output GPIO pin.
            fn toggle(&self, pin: usize)
            -> ReturnCode;
            /// Assert a GPIO pin high.
            fn set(&self, pin: usize)
            -> ReturnCode;
            /// Clear a GPIO pin low.
            fn clear(&self, pin: usize)
            -> ReturnCode;
            /// Setup an interrupt on a GPIO input pin. The identifier should be
            /// the port number and will be returned when the interrupt callback
            /// fires.
            fn enable_interrupt(&self, pin: usize,
                                mode: hil::gpio::InterruptEdge)
            -> ReturnCode;
            /// Disable an interrupt on a GPIO input pin.
            fn disable_interrupt(&self, pin: usize)
            -> ReturnCode;
            fn is_pending(&self, pin: usize)
            -> bool;
        }
        /// The gpio_async Client interface is used to both receive callbacks
        /// when a configuration command finishes and to handle interrupt events
        /// from pins with interrupts enabled.
        pub trait Client {
            /// Called when an interrupt occurs. The pin that interrupted is included,
            /// and the identifier that was passed with the call to `enable_interrupt`
            /// is also returned.
            fn fired(&self, pin: usize, identifier: usize);
            /// Done is called when a configuration command finishes.
            fn done(&self, value: usize);
        }
    }
    pub mod i2c {
        //! Interface for I2C master and slave peripherals.
        use core::fmt::{Display, Formatter, Result};
        /// The type of error encoutered during I2C communication.
        pub enum Error {

            /// The slave did not acknowledge the chip address. Most likely the address
            /// is incorrect or the slave is not properly connected.
            AddressNak,

            /// The data was not acknowledged by the slave.
            DataNak,

            /// Arbitration lost, meaning the state of the data line does not correspond
            /// to the data driven onto it. This can happen, for example, when a
            /// higher-priority transmission is in progress by a different master.
            ArbitrationLost,

            /// A start condition was received before received data has been read
            /// from the receive register.
            Overrun,

            /// No error occured and the command completed successfully.
            CommandComplete,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
        #[automatically_derived]
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
        /// This specifies what type of transmission just finished from a Master device.
        pub enum SlaveTransmissionType { Write, Read, }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for SlaveTransmissionType { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for SlaveTransmissionType {
            #[inline]
            fn clone(&self) -> SlaveTransmissionType { { *self } }
        }
        #[automatically_derived]
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
        /// Interface for an I2C Master hardware driver.
        pub trait I2CMaster {
            fn enable(&self);
            fn disable(&self);
            fn write_read(&self, addr: u8, data: &'static mut [u8],
                          write_len: u8, read_len: u8);
            fn write(&self, addr: u8, data: &'static mut [u8], len: u8);
            fn read(&self, addr: u8, buffer: &'static mut [u8], len: u8);
        }
        /// Interface for an I2C Slave hardware driver.
        pub trait I2CSlave {
            fn enable(&self);
            fn disable(&self);
            fn set_address(&self, addr: u8);
            fn write_receive(&self, data: &'static mut [u8], max_len: u8);
            fn read_send(&self, data: &'static mut [u8], max_len: u8);
            fn listen(&self);
        }
        /// Convenience type for capsules that need hardware that supports both
        /// Master and Slave modes.
        pub trait I2CMasterSlave: I2CMaster + I2CSlave { }
        /// Client interface for capsules that use I2CMaster devices.
        pub trait I2CHwMasterClient {
            /// Called when an I2C command completed. The `error` denotes whether the command completed
            /// successfully or if an error occured.
            fn command_complete(&self, buffer: &'static mut [u8],
                                error: Error);
        }
        /// Client interface for capsules that use I2CSlave devices.
        pub trait I2CHwSlaveClient {
            /// Called when an I2C command completed.
            fn command_complete(&self, buffer: &'static mut [u8], length: u8,
                                transmission_type: SlaveTransmissionType);
            /// Called from the I2C slave hardware to say that a Master has sent us
            /// a read message, but the driver did not have a buffer containing data
            /// setup, and therefore cannot respond. The I2C slave hardware will stretch
            /// the clock while waiting for the upper layer capsule to provide data
            /// to send to the remote master. Call `I2CSlave::read_send()` to provide
            /// data.
            fn read_expected(&self);
            /// Called from the I2C slave hardware to say that a Master has sent us
            /// a write message, but there was no buffer setup to read the bytes into.
            /// The HW will stretch the clock while waiting for the user to call
            /// `I2CSlave::write_receive()` with a buffer.
            fn write_expected(&self);
        }
        /// Higher-level interface for I2C Master commands that wraps in the I2C
        /// address. It gives an interface for communicating with a specific I2C
        /// device.
        pub trait I2CDevice {
            fn enable(&self);
            fn disable(&self);
            fn write_read(&self, data: &'static mut [u8], write_len: u8,
                          read_len: u8);
            fn write(&self, data: &'static mut [u8], len: u8);
            fn read(&self, buffer: &'static mut [u8], len: u8);
        }
        /// Client interface for I2CDevice implementations.
        pub trait I2CClient {
            /// Called when an I2C command completed. The `error` denotes whether the command completed
            /// successfully or if an error occured.
            fn command_complete(&self, buffer: &'static mut [u8],
                                error: Error);
        }
    }
    pub mod led {
        //! Interface for LEDs that abstract away polarity and pin.
        //!
        //!  Author: Philip Levis <pal@cs.stanford.edu>
        //!  Date: July 31, 2015
        //!
        use crate::hil::gpio;
        pub trait Led {
            fn init(&mut self);
            fn on(&mut self);
            fn off(&mut self);
            fn toggle(&mut self);
            fn read(&self)
            -> bool;
        }
        /// For LEDs in which on is when GPIO is high.
        pub struct LedHigh<'a> {
            pub pin: &'a mut dyn gpio::Pin,
        }
        /// For LEDs in which on is when GPIO is low.
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
        //! Generic interface for nonvolatile memory.
        use crate::returncode::ReturnCode;
        /// Simple interface for reading and writing nonvolatile memory. It is expected
        /// that drivers for nonvolatile memory would implement this trait.
        pub trait NonvolatileStorage<'a> {
            fn set_client(&self,
                          client: &'a dyn NonvolatileStorageClient<'a>);
            /// Read `length` bytes starting at address `address` in to the provided
            /// buffer. The buffer must be at least `length` bytes long. The address
            /// must be in the address space of the physical storage.
            fn read(&self, buffer: &'a mut [u8], address: usize,
                    length: usize)
            -> ReturnCode;
            /// Write `length` bytes starting at address `address` from the provided
            /// buffer. The buffer must be at least `length` bytes long. This address
            /// must be in the address space of the physical storage.
            fn write(&self, buffer: &'a mut [u8], address: usize,
                     length: usize)
            -> ReturnCode;
        }
        /// Client interface for nonvolatile storage.
        pub trait NonvolatileStorageClient<'a> {
            /// `read_done` is called when the implementor is finished reading in to the
            /// buffer. The callback returns the buffer and the number of bytes that
            /// were actually read.
            fn read_done(&self, buffer: &'a mut [u8], length: usize);
            /// `write_done` is called when the implementor is finished writing from the
            /// buffer. The callback returns the buffer and the number of bytes that
            /// were actually written.
            fn write_done(&self, buffer: &'a mut [u8], length: usize);
        }
    }
    pub mod pwm {
        //! Interfaces for Pulse Width Modulation output.
        use crate::returncode::ReturnCode;
        /// PWM control for a single pin.
        pub trait Pwm {
            /// The chip-dependent type of a PWM pin.
            type
            Pin;
            /// Generate a PWM single on the given pin at the given frequency and duty
            /// cycle.
            ///
            /// - `frequency_hz` is specified in Hertz.
            /// - `duty_cycle` is specified as a portion of the max duty cycle supported
            ///   by the chip. Clients should call `get_maximum_duty_cycle()` to get the
            ///   value that corresponds to 100% duty cycle, and divide that
            ///   appropriately to get the desired duty cycle value. For example, a 25%
            ///   duty cycle would be `PWM0.get_maximum_duty_cycle() / 4`.
            fn start(&self, pin: &Self::Pin, frequency_hz: usize,
                     duty_cycle: usize)
            -> ReturnCode;
            /// Stop a PWM pin output.
            fn stop(&self, pin: &Self::Pin)
            -> ReturnCode;
            /// Return the maximum PWM frequency supported by the PWM implementation.
            /// The frequency will be specified in Hertz.
            fn get_maximum_frequency_hz(&self)
            -> usize;
            /// Return an opaque number that represents a 100% duty cycle. This value
            /// will be hardware specific, and essentially represents the precision
            /// of the underlying PWM hardware.
            ///
            /// Users of this HIL should divide this number to calculate a duty cycle
            /// value suitable for calling `start()`. For example, to generate a 50%
            /// duty cycle:
            ///
            /// ```ignore
            /// let max = PWM0.get_maximum_duty_cycle();
            /// let dc  = max / 2;
            /// PWM0.start(pin, freq, dc);
            /// ```
            fn get_maximum_duty_cycle(&self)
            -> usize;
        }
        /// Higher-level PWM interface that restricts the user to a specific PWM pin.
        /// This is particularly useful for passing to capsules that need to control
        /// only a specific pin.
        pub trait PwmPin {
            /// Start a PWM output. Same as the `start` function in the `Pwm` trait.
            fn start(&self, frequency_hz: usize, duty_cycle: usize)
            -> ReturnCode;
            /// Stop a PWM output. Same as the `stop` function in the `Pwm` trait.
            fn stop(&self)
            -> ReturnCode;
            /// Return the maximum PWM frequency supported by the PWM implementation.
            /// Same as the `get_maximum_frequency_hz` function in the `Pwm` trait.
            fn get_maximum_frequency_hz(&self)
            -> usize;
            /// Return an opaque number that represents a 100% duty cycle. This value
            /// Same as the `get_maximum_duty_cycle` function in the `Pwm` trait.
            fn get_maximum_duty_cycle(&self)
            -> usize;
        }
    }
    pub mod radio {
        //! Interface for sending and receiving IEEE 802.15.4 packets.
        //!
        //! Hardware independent interface for an 802.15.4 radio. Note that
        //! configuration commands are asynchronous and must be committed with a call to
        //! config_commit. For example, calling set_address will change the source
        //! address of packets but does not change the address stored in hardware used
        //! for address recognition. This must be committed to hardware with a call to
        //! config_commit. Please see the relevant TRD for more details.
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
        /// These constants are used for interacting with the SPI buffer, which contains
        /// a 1-byte SPI command, a 1-byte PHY header, and then the 802.15.4 frame. In
        /// theory, the number of extra bytes in front of the frame can depend on the
        /// particular method used to communicate with the radio, but we leave this as a
        /// constant in this generic trait for now.
        ///
        /// Furthermore, the minimum MHR size assumes that
        ///
        /// - The source PAN ID is omitted
        /// - There is no auxiliary security header
        /// - There are no IEs
        ///
        /// ```text
        /// +---------+-----+-----+-------------+-----+
        /// | SPI com | PHR | MHR | MAC payload | MFR |
        /// +---------+-----+-----+-------------+-----+
        /// \______ Static buffer rx/txed to SPI _____/
        ///                 \__ PSDU / frame length __/
        /// \___ 2 bytes ___/
        /// ```
        pub const MIN_MHR_SIZE: usize = 9;
        pub const MFR_SIZE: usize = 2;
        pub const MAX_MTU: usize = 127;
        pub const MIN_FRAME_SIZE: usize = MIN_MHR_SIZE + MFR_SIZE;
        pub const MAX_FRAME_SIZE: usize = MAX_MTU;
        pub const PSDU_OFFSET: usize = 2;
        pub const MAX_BUF_SIZE: usize = PSDU_OFFSET + MAX_MTU;
        pub const MIN_PAYLOAD_OFFSET: usize = PSDU_OFFSET + MIN_MHR_SIZE;
        pub trait Radio: RadioConfig + RadioData { }
        /// Configure the 802.15.4 radio.
        pub trait RadioConfig {
            /// buf must be at least MAX_BUF_SIZE in length, and
            /// reg_read and reg_write must be 2 bytes.
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
            /// Commit the config calls to hardware, changing the address,
            /// PAN ID, TX power, and channel to the specified values, issues
            /// a callback to the config client when done.
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
        //! Interfaces for accessing a random number generator.
        //!
        //! A random number generator produces a stream of random numbers,
        //! either from hardware or based on an initial seed. The
        //! [RNG](trait.RNG.html) trait provides a simple, implementation
        //! agnostic interface for getting new random values.
        //!
        //! _Randomness_: Random numbers generated by this trait MUST pass
        //! standard randomness tests, such as A. Rukhin, J. Soto,
        //! J. Nechvatal, M. Smid, E. Barker, S. Leigh, M. Levenson,
        //! M. Vangel, D. Banks, A. Heckert, J. Dray, and S. Vo. A statistical
        //! test suite for random and pseudorandom number generators for
        //! cryptographic applications. Technical report, NIST, 2010. It is
        //! acceptable for implementations to rely on prior verification of
        //! the algorithm being used. For example, if the implementation
        //! chooses to use a Fishman and Moore Linear Congruence Generator
        //! (LCG) with the parameters specified in the NIST report above, it
        //! does not need to re-run the tests.
        //!
        //! Entropy: This trait does not promise high-entropy random numbers,
        //! although it MAY generate them. Implementations of this interface
        //! MAY generate random numbers using techniques other than true
        //! random number generation (through entropy) or cryptographically
        //! secure pseudorandom number generation. Other traits, described
        //! elsewhere, provide random numbers with entropy guarantees. This
        //! trait MUST NOT be used for randomness needed for security or
        //! cryptography. If high-entropy randomness is needed, the `Entropy`
        //! trait should be used instead.
        //!
        //! The Rng trait is designed to work well with random number
        //! generators that may not have values ready immediately. This is
        //! important when generating numbers from a low-bandwidth hardware
        //! random number generator or when the RNG is virtualized among many
        //! consumers.  Random numbers are yielded to the
        //! [Client](trait.Client.html) as an `Iterator` which only terminates
        //! when no more numbers are currently available. Clients can request
        //! more randomness if needed and will be called again when more is
        //! available.
        //!
        //! The Random trait is synchronous, so designed to work
        //! with arithmetically simple random number generators that can
        //! return a result quickly.
        //!
        //!
        //! # Example
        //!
        //! The following example is a simple capsule that prints out a random number
        //! once a second using the `Alarm` and `RNG` traits.
        //!
        //! ```
        //! use kernel::hil;
        //! use kernel::hil::time::Frequency;
        //! use kernel::ReturnCode;
        //!
        //! struct RngTest<'a, A: 'a + hil::time::Alarm<'a>> {
        //!     rng: &'a hil::rng::Rng<'a>,
        //!     alarm: &'a A
        //! }
        //!
        //! impl<'a, A: hil::time::Alarm<'a>> RngTest<'a, A> {
        //!     pub fn initialize(&self) {
        //!         let interval = 1 * <A::Frequency>::frequency();
        //!         let tics = self.alarm.now().wrapping_add(interval);
        //!         self.alarm.set_alarm(tics);
        //!     }
        //! }
        //!
        //! impl<'a, A: hil::time::Alarm<'a>> hil::time::AlarmClient for RngTest<'a, A> {
        //!     fn fired(&self) {
        //!         self.rng.get();
        //!     }
        //! }
        //!
        //! impl<'a, A: hil::time::Alarm<'a>> hil::rng::Client for RngTest<'a, A> {
        //!     fn randomness_available(&self,
        //!                             randomness: &mut Iterator<Item = u32>,
        //!                             error: ReturnCode) -> hil::rng::Continue {
        //!         match randomness.next() {
        //!             Some(random) => {
        //!                 println!("Rand {}", random);
        //!                 let interval = 1 * <A::Frequency>::frequency();
        //!                 let tics = self.alarm.now().wrapping_add(interval);
        //!                 self.alarm.set_alarm(tics);
        //!                 hil::rng::Continue::Done
        //!             },
        //!             None => hil::rng::Continue::More
        //!         }
        //!     }
        //! }
        //! ```
        use crate::returncode::ReturnCode;
        /// Denotes whether the [Client](trait.Client.html) wants to be notified when
        /// `More` randomness is available or if they are `Done`
        pub enum Continue {

            /// More randomness is required.
            More,

            /// No more randomness required.
            Done,
        }
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for Continue {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () { { } }
        }
        #[automatically_derived]
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
        /// Generic interface for a 32-bit random number generator.
        ///
        /// Implementors should assume the client implements the
        /// [Client](trait.Client.html) trait.
        pub trait Rng<'a> {
            /// Initiate the aquisition of new random number generation.
            ///
            /// There are three valid return values:
            ///   - SUCCESS: a `randomness_available` callback will be called in
            ///     the future when randomness is available.
            ///   - FAIL: a `randomness_available` callback will not be called in
            ///     the future, because random numbers cannot be generated. This
            ///     is a general failure condition.
            ///   - EOFF: a `randomness_available` callback will not be called in
            ///     the future, because the random number generator is off/not
            ///     powered.
            fn get(&self)
            -> ReturnCode;
            /// Cancel acquisition of random numbers.
            ///
            /// There are two valid return values:
            ///   - SUCCESS: an outstanding request from `get` has been cancelled,
            ///     or there was no oustanding request. No `randomness_available`
            ///     callback will be issued.
            ///   - FAIL: There will be a randomness_available callback, which
            ///     may or may not return an error code.
            fn cancel(&self)
            -> ReturnCode;
            fn set_client(&'a self, _: &'a dyn Client);
        }
        /// An [Rng](trait.Rng.html) client
        ///
        /// Clients of an [Rng](trait.Rng.html) must implement this trait.
        pub trait Client {
            /// Called by the (RNG)[trait.RNG.html] when there are one or more random
            /// numbers available
            ///
            /// `randomness` in an `Iterator` of available random numbers. The amount of
            /// randomness available may increase if `randomness` is not consumed
            /// quickly so clients should not rely on iterator termination to finish
            /// consuming random numbers.
            ///
            /// The client returns either `Continue::More` if the iterator did not have
            /// enough random values and the client would like to be called again when
            /// more is available, or `Continue::Done`.
            ///
            /// If randoness_available is triggered after a call to cancel()
            /// then error MUST be ECANCEL and randomness MAY contain
            /// random bits.
            fn randomness_available(&self,
                                    randomness: &mut dyn Iterator<Item = u32>,
                                    error: ReturnCode)
            -> Continue;
        }
        /// Generic interface for a synchronous 32-bit random number
        /// generator.
        pub trait Random<'a> {
            /// Initialize/reseed the random number generator from an
            /// internal source. This initialization MAY be deterministic
            /// (e.g., based on an EUI-64) or MAY be random (e.g., based on an
            /// underlying hardware entropy source); an implementation SHOULD
            /// make reseeding random.
            fn initialize(&'a self);
            /// Reseed the random number generator with a specific
            /// seed. Useful for deterministic tests.
            fn reseed(&self, seed: u32);
            /// Generate a 32-bit random number.
            fn random(&self)
            -> u32;
        }
    }
    pub mod sensors {
        //! Interfaces for environment sensors
        use crate::returncode::ReturnCode;
        /// A basic interface for a temperature sensor
        pub trait TemperatureDriver {
            fn set_client(&self, client: &'static dyn TemperatureClient);
            fn read_temperature(&self)
            -> ReturnCode;
        }
        /// Client for receiving temperature readings.
        pub trait TemperatureClient {
            /// Called when a temperature reading has completed.
            ///
            /// - `value`: the most recently read temperature in hundredths of degrees
            /// centigrate.
            fn callback(&self, value: usize);
        }
        /// A basic interface for a humidity sensor
        pub trait HumidityDriver {
            fn set_client(&self, client: &'static dyn HumidityClient);
            fn read_humidity(&self)
            -> ReturnCode;
        }
        /// Client for receiving humidity readings.
        pub trait HumidityClient {
            /// Called when a humidity reading has completed.
            ///
            /// - `value`: the most recently read humidity in hundredths of percent.
            fn callback(&self, value: usize);
        }
        /// A basic interface for an ambient light sensor.
        pub trait AmbientLight {
            /// Set the client to be notified when the capsule has data ready or has
            /// finished some command.  This is likely called in a board's `main.rs`.
            fn set_client(&self, client: &'static dyn AmbientLightClient);
            /// Get a single instantaneous reading of the ambient light intensity.
            fn read_light_intensity(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
        }
        /// Client for receiving light intensity readings.
        pub trait AmbientLightClient {
            /// Called when an ambient light reading has completed.
            ///
            /// - `lux`: the most recently read ambient light reading in lux (lx).
            fn callback(&self, lux: usize);
        }
        /// A basic interface for a 9-DOF compatible chip.
        ///
        /// This trait provides a standard interface for chips that implement
        /// some or all of a nine degrees of freedom (accelerometer, magnetometer,
        /// gyroscope) sensor. Any interface functions that a chip cannot implement
        /// can be ignored by the chip capsule and an error will automatically be
        /// returned.
        pub trait NineDof {
            /// Set the client to be notified when the capsule has data ready or
            /// has finished some command. This is likely called in a board's main.rs
            /// and is set to the virtual_ninedof.rs driver.
            fn set_client(&self, client: &'static dyn NineDofClient);
            /// Get a single instantaneous reading of the acceleration in the
            /// X,Y,Z directions.
            fn read_accelerometer(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
            /// Get a single instantaneous reading from the magnetometer in all
            /// three directions.
            fn read_magnetometer(&self) -> ReturnCode {
                ReturnCode::ENODEVICE
            }
            /// Get a single instantaneous reading from the gyroscope of the rotation
            /// around all three axes.
            fn read_gyroscope(&self) -> ReturnCode { ReturnCode::ENODEVICE }
        }
        /// Client for receiving done events from the chip.
        pub trait NineDofClient {
            /// Signals a command has finished. The arguments will most likely be passed
            /// over the syscall interface to an application.
            fn callback(&self, arg1: usize, arg2: usize, arg3: usize);
        }
    }
    pub mod spi {
        //! Interfaces for SPI master and slave communication.
        use crate::returncode::ReturnCode;
        use core::option::Option;
        /// Values for the ordering of bits
        pub enum DataOrder { MSBFirst, LSBFirst, }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for DataOrder { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for DataOrder {
            #[inline]
            fn clone(&self) -> DataOrder { { *self } }
        }
        #[automatically_derived]
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
        /// Values for the clock polarity (idle state or CPOL)
        pub enum ClockPolarity { IdleLow, IdleHigh, }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPolarity { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPolarity {
            #[inline]
            fn clone(&self) -> ClockPolarity { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        /// Which clock edge values are sampled on
        pub enum ClockPhase { SampleLeading, SampleTrailing, }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for ClockPhase { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for ClockPhase {
            #[inline]
            fn clone(&self) -> ClockPhase { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
            /// Called when a read/write operation finishes
            fn read_write_done(&self, write_buffer: &'static mut [u8],
                               read_buffer: Option<&'static mut [u8]>,
                               len: usize);
        }
        /// The `SpiMaster` trait for interacting with SPI slave
        /// devices at a byte or buffer level.
        ///
        /// Using SpiMaster normally involves three steps:
        ///
        /// 1. Configure the SPI bus for a peripheral
        ///    1a. Call set_chip_select to select which peripheral and
        ///        turn on SPI
        ///    1b. Call set operations as needed to configure bus
        ///    NOTE: You MUST select the chip select BEFORE configuring
        ///           SPI settings.
        /// 2. Invoke read, write, read_write on SpiMaster
        /// 3a. Call clear_chip_select to turn off bus, or
        /// 3b. Call set_chip_select to choose another peripheral,
        ///     go to step 1b or 2.
        ///
        /// This interface assumes that the SPI configuration for
        /// a particular peripheral persists across chip select. For
        /// example, with this set of calls:
        ///
        ///   specify_chip_select(1);
        ///   set_phase(SampleLeading);
        ///   specify_chip_select(2);
        ///   set_phase(SampleTrailing);
        ///   specify_chip_select(1);
        ///   write_byte(0); // Uses SampleLeading
        ///
        /// If additional chip selects are needed, they can be performed
        /// with GPIO and manual re-initialization of settings.
        ///
        ///   specify_chip_select(0);
        ///   set_phase(SampleLeading);
        ///   pin_a.set();
        ///   write_byte(0xaa); // Uses SampleLeading
        ///   pin_a.clear();
        ///   set_phase(SampleTrailing);
        ///   pin_b.set();
        ///   write_byte(0xaa); // Uses SampleTrailing
        ///
        pub trait SpiMaster {
            type
            ChipSelect: Copy;
            fn set_client(&self, client: &'static dyn SpiMasterClient);
            fn init(&self);
            fn is_busy(&self)
            -> bool;
            /// Perform an asynchronous read/write operation, whose
            /// completion is signaled by invoking SpiMasterClient on
            /// the initialized client. write_buffer must be Some,
            /// read_buffer may be None. If read_buffer is Some, the
            /// length of the operation is the minimum of the size of
            /// the two buffers.
            fn read_write_bytes(&self, write_buffer: &'static mut [u8],
                                read_buffer: Option<&'static mut [u8]>,
                                len: usize)
            -> ReturnCode;
            fn write_byte(&self, val: u8);
            fn read_byte(&self)
            -> u8;
            fn read_write_byte(&self, val: u8)
            -> u8;
            /// Tell the SPI peripheral what to use as a chip select pin.
            /// The type of the argument is based on what makes sense for the
            /// peripheral when this trait is implemented.
            fn specify_chip_select(&self, cs: Self::ChipSelect);
            /// Returns the actual rate set
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
        /// SPIMasterDevice provides a chip-specific interface to the SPI Master
        /// hardware. The interface wraps the chip select line so that chip drivers
        /// cannot communicate with different SPI devices.
        pub trait SpiMasterDevice {
            /// Setup the SPI settings and speed of the bus.
            fn configure(&self, cpol: ClockPolarity, cpal: ClockPhase,
                         rate: u32);
            /// Perform an asynchronous read/write operation, whose
            /// completion is signaled by invoking SpiMasterClient.read_write_done on
            /// the provided client. write_buffer must be Some,
            /// read_buffer may be None. If read_buffer is Some, the
            /// length of the operation is the minimum of the size of
            /// the two buffers.
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
            /// This is called whenever the slave is selected by the master
            fn chip_selected(&self);
            /// This is called as a DMA interrupt when a transfer has completed
            fn read_write_done(&self, write_buffer: Option<&'static mut [u8]>,
                               read_buffer: Option<&'static mut [u8]>,
                               len: usize);
        }
        pub trait SpiSlave {
            fn init(&self);
            /// Returns true if there is a client.
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
        /// SPISlaveDevice provides a chip-specific interface to the SPI Slave
        /// hardware. The interface wraps the chip select line so that chip drivers
        /// cannot communicate with different SPI devices.
        pub trait SpiSlaveDevice {
            /// Setup the SPI settings and speed of the bus.
            fn configure(&self, cpol: ClockPolarity, cpal: ClockPhase);
            /// Perform an asynchronous read/write operation, whose
            /// completion is signaled by invoking SpiSlaveClient.read_write_done on
            /// the provided client. Either write_buffer or read_buffer may be
            /// None. If read_buffer is Some, the length of the operation is the
            /// minimum of the size of the two buffers.
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
        //! Interface for symmetric-cipher encryption
        //!
        //! see boards/imix/src/aes_test.rs for example usage
        use crate::returncode::ReturnCode;
        /// Implement this trait and use `set_client()` in order to receive callbacks from an `AES128`
        /// instance.
        pub trait Client<'a> {
            fn crypt_done(&'a self, source: Option<&'a mut [u8]>,
                          dest: &'a mut [u8]);
        }
        /// The number of bytes used for AES block operations.  Keys and IVs must have this length,
        /// and encryption/decryption inputs must be have a multiple of this length.
        pub const AES128_BLOCK_SIZE: usize = 16;
        pub const AES128_KEY_SIZE: usize = 16;
        pub trait AES128<'a> {
            /// Enable the AES hardware.
            /// Must be called before any other methods
            fn enable(&self);
            /// Disable the AES hardware
            fn disable(&self);
            /// Set the client instance which will receive `crypt_done()` callbacks
            fn set_client(&'a self, client: &'a dyn Client<'a>);
            /// Set the encryption key.
            /// Returns `EINVAL` if length is not `AES128_KEY_SIZE`
            fn set_key(&self, key: &[u8])
            -> ReturnCode;
            /// Set the IV (or initial counter).
            /// Returns `EINVAL` if length is not `AES128_BLOCK_SIZE`
            fn set_iv(&self, iv: &[u8])
            -> ReturnCode;
            /// Begin a new message (with the configured IV) when `crypt()` is
            /// next called.  Multiple calls to `crypt()` may be made between
            /// calls to `start_message()`, allowing the encryption context to
            /// extend over non-contiguous extents of data.
            ///
            /// If an encryption operation is in progress, this method instead
            /// has no effect.
            fn start_message(&self);
            /// Request an encryption/decryption
            ///
            /// If the source buffer is not `None`, the encryption input
            /// will be that entire buffer.  Otherwise the destination buffer
            /// at indices between `start_index` and `stop_index` will
            /// provide the input, which will be overwritten.
            ///
            /// If `None` is returned, the client's `crypt_done` method will eventually
            /// be called, and the portion of the data buffer between `start_index`
            /// and `stop_index` will hold the result of the encryption/decryption.
            ///
            /// If `Some(result, source, dest)` is returned, `result` is the
            /// error condition and `source` and `dest` are the buffers that
            /// were passed to `crypt`.
            ///
            /// The indices `start_index` and `stop_index` must be valid
            /// offsets in the destination buffer, and the length
            /// `stop_index - start_index` must be a multiple of
            /// `AES128_BLOCK_SIZE`.  Otherwise, `Some(EINVAL, ...)` will be
            /// returned.
            ///
            /// If the source buffer is not `None`, its length must be
            /// `stop_index - start_index`.  Otherwise, `Some(EINVAL, ...)`
            /// will be returned.
            ///
            /// If an encryption operation is already in progress,
            /// `Some(EBUSY, ...)` will be returned.
            ///
            /// For correct operation, the methods `set_key` and `set_iv` must have
            /// previously been called to set the buffers containing the
            /// key and the IV (or initial counter value), and a method `set_mode_*()`
            /// must have been called to set the desired mode.  These settings persist
            /// across calls to `crypt()`.
            ///
            fn crypt(&'a self, source: Option<&'a mut [u8]>,
                     dest: &'a mut [u8], start_index: usize,
                     stop_index: usize)
            -> Option<(ReturnCode, Option<&'a mut [u8]>, &'a mut [u8])>;
        }
        pub trait AES128Ctr {
            /// Call before `AES128::crypt()` to perform AES128Ctr
            fn set_mode_aes128ctr(&self, encrypting: bool);
        }
        pub trait AES128CBC {
            /// Call before `AES128::crypt()` to perform AES128CBC
            fn set_mode_aes128cbc(&self, encrypting: bool);
        }
        pub trait CCMClient {
            /// `res` is SUCCESS if the encryption/decryption process succeeded. This
            /// does not mean that the message has been verified in the case of
            /// decryption.
            /// If we are encrypting: `tag_is_valid` is `true` iff `res` is SUCCESS.
            /// If we are decrypting: `tag_is_valid` is `true` iff `res` is SUCCESS and the
            /// message authentication tag is valid.
            fn crypt_done(&self, buf: &'static mut [u8], res: ReturnCode,
                          tag_is_valid: bool);
        }
        pub const CCM_NONCE_LENGTH: usize = 13;
        pub trait AES128CCM<'a> {
            /// Set the client instance which will receive `crypt_done()` callbacks
            fn set_client(&'a self, client: &'a dyn CCMClient);
            /// Set the key to be used for CCM encryption
            fn set_key(&self, key: &[u8])
            -> ReturnCode;
            /// Set the nonce (length NONCE_LENGTH) to be used for CCM encryption
            fn set_nonce(&self, nonce: &[u8])
            -> ReturnCode;
            /// Try to begin the encryption/decryption process
            fn crypt(&self, buf: &'static mut [u8], a_off: usize,
                     m_off: usize, m_len: usize, mic_len: usize,
                     confidential: bool, encrypting: bool)
            -> (ReturnCode, Option<&'static mut [u8]>);
        }
    }
    pub mod time {
        //! Hardware agnostic interfaces for counter-like resources.
        use crate::ReturnCode;
        pub trait Time<W = u32> {
            type
            Frequency: Frequency;
            /// Returns the current time in hardware clock units.
            fn now(&self)
            -> W;
            /// Returns the wrap-around value of the clock.
            ///
            /// The maximum value of the clock, at which `now` will wrap around. I.e., this should return
            /// `core::u32::MAX` on a 32-bit-clock, or `(1 << 24) - 1` for a 24-bit clock.
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
        /// Trait to represent clock frequency in Hz
        ///
        /// This trait is used as an associated type for `Alarm` so clients can portably
        /// convert native cycles to real-time values.
        pub trait Frequency {
            /// Returns frequency in Hz.
            fn frequency()
            -> u32;
        }
        /// 16MHz `Frequency`
        pub struct Freq16MHz;
        #[automatically_derived]
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
        /// 32KHz `Frequency`
        pub struct Freq32KHz;
        #[automatically_derived]
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
        /// 16KHz `Frequency`
        pub struct Freq16KHz;
        #[automatically_derived]
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
        /// 1KHz `Frequency`
        pub struct Freq1KHz;
        #[automatically_derived]
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
        /// The `Alarm` trait models a wrapping counter capable of notifying when the
        /// counter reaches a certain value.
        ///
        /// Alarms represent a resource that keeps track of time in some fixed unit
        /// (usually clock tics). Implementers should use the
        /// [`Client`](trait.Client.html) trait to signal when the counter has
        /// reached a pre-specified value set in [`set_alarm`](#tymethod.set_alarm).
        pub trait Alarm<'a, W = u32>: Time<W> {
            /// Sets a one-shot alarm to fire when the clock reaches `tics`.
            ///
            /// [`Client#fired`](trait.Client.html#tymethod.fired) is signaled
            /// when `tics` is reached.
            ///
            /// # Examples
            ///
            /// ```ignore
            /// let delta = 1337;
            /// let tics = alarm.now().wrapping_add(delta);
            /// alarm.set_alarm(tics);
            /// ```
            fn set_alarm(&self, tics: W);
            /// Returns the value set in [`set_alarm`](#tymethod.set_alarm)
            fn get_alarm(&self)
            -> W;
            /// Set the client for interrupt events.
            fn set_client(&'a self, client: &'a dyn AlarmClient);
            /// Returns whether this alarm is currently active (will eventually trigger
            /// a callback if there is a client).
            fn is_enabled(&self)
            -> bool;
            /// Enables the alarm using the previously set `tics` for the alarm.
            ///
            /// Most implementations should use the default implementation which calls `set_alarm` with the
            /// value returned by `get_alarm` unless there is a more efficient way to achieve the same
            /// semantics.
            fn enable(&self) { self.set_alarm(self.get_alarm()) }
            /// Disables the alarm.
            ///
            /// The implementation will _always_ disable the alarm and prevent events related to previously
            /// set alarms from being delivered to the client.
            fn disable(&self);
        }
        /// A client of an implementer of the [`Alarm`](trait.Alarm.html) trait.
        pub trait AlarmClient {
            /// Callback signaled when the alarm's clock reaches the value set in
            /// [`Alarm#set_alarm`](trait.Alarm.html#tymethod.set_alarm).
            fn fired(&self);
        }
        /// The `Timer` trait models a timer that can notify when a particular interval
        /// has elapsed.
        pub trait Timer<'a, W = u32>: Time<W> {
            /// Set the client for interrupt events.
            fn set_client(&'a self, client: &'a dyn TimerClient);
            /// Sets a one-shot timer to fire in `interval` clock-tics.
            ///
            /// Calling this method will override any existing oneshot or repeating timer.
            fn oneshot(&self, interval: W);
            /// Sets repeating timer to fire every `interval` clock-tics.
            ///
            /// Calling this method will override any existing oneshot or repeating timer.
            fn repeat(&self, interval: W);
            /// Returns the interval for a repeating timer.
            ///
            /// Returns `None` if the timer is disabled or in oneshot mode and `Some(interval)` if it is
            /// repeating.
            fn interval(&self)
            -> Option<W>;
            /// Returns whether this is a oneshot (rather than repeating) timer.
            fn is_oneshot(&self) -> bool { self.interval().is_none() }
            /// Returns whether this is a repeating (rather than oneshot) timer.
            fn is_repeating(&self) -> bool { self.interval().is_some() }
            /// Returns the remaining time in clock tics for a oneshot or repeating timer.
            ///
            /// Returns `None` if the timer is disabled.
            fn time_remaining(&self)
            -> Option<W>;
            /// Returns whether this timer is currently active (has time remaining).
            fn is_enabled(&self) -> bool { self.time_remaining().is_some() }
            /// Cancels an outstanding timer.
            ///
            /// The implementation will _always_ cancel the timer.
            /// delivered.
            fn cancel(&self);
        }
        /// A client of an implementer of the [`Timer`](trait.Timer.html) trait.
        pub trait TimerClient {
            /// Callback signaled when the timer's clock reaches the specified interval.
            fn fired(&self);
        }
    }
    pub mod uart {
        //! Hardware interface layer (HIL) traits for UART communication.
        //!
        //!
        use crate::returncode::ReturnCode;
        pub enum StopBits { One = 1, Two = 2, }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for StopBits { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for StopBits {
            #[inline]
            fn clone(&self) -> StopBits { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Parity { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Parity {
            #[inline]
            fn clone(&self) -> Parity { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Width { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Width {
            #[inline]
            fn clone(&self) -> Width { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Parameters { }
        #[automatically_derived]
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
        #[automatically_derived]
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
        /// The type of error encountered during UART transaction.
        pub enum Error {

            /// No error occurred and the command completed successfully
            None,

            /// Parity error during receive
            ParityError,

            /// Framing error during receive
            FramingError,

            /// Overrun error during receive
            OverrunError,

            /// Repeat call of transmit or receive before initial command complete
            RepeatCallError,

            /// UART hardware was reset
            ResetError,

            /// Read or write was aborted early
            Aborted,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Error { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error { { *self } }
        }
        #[automatically_derived]
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
        #[automatically_derived]
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
        /// Trait for configuring a UART.
        pub trait Configure {
            /// Returns SUCCESS, or
            /// - EOFF: The underlying hardware is currently not available, perhaps
            ///         because it has not been initialized or in the case of a shared
            ///         hardware USART controller because it is set up for SPI.
            /// - EINVAL: Impossible parameters (e.g. a `baud_rate` of 0)
            /// - ENOSUPPORT: The underlying UART cannot satisfy this configuration.
            fn configure(&self, params: Parameters)
            -> ReturnCode;
        }
        pub trait Transmit<'a> {
            /// Set the transmit client, which will be called when transmissions
            /// complete.
            fn set_transmit_client(&self, client: &'a dyn TransmitClient);
            /// Transmit a buffer of data. On completion, `transmitted_buffer`
            /// in the `TransmitClient` will be called.  If the `ReturnCode`
            /// of `transmit`'s return tuple is SUCCESS, the `Option` will be
            /// `None` and the struct will issue a `transmitted_buffer`
            /// callback in the future. If the value of the `ReturnCode` is
            /// not SUCCESS, then the `tx_buffer` argument is returned in the
            /// `Option`. Other valid `ReturnCode` values are:
            ///  - EOFF: The underlying hardware is not available, perhaps because
            ///          because it has not been initialized or in the case of a shared
            ///          hardware USART controller because it is set up for SPI.
            ///  - EBUSY: the UART is already transmitting and has not made a
            ///           transmission callback yet.
            ///  - ESIZE : `tx_len` is larger than the passed slice.
            ///  - FAIL: some other error.
            ///
            /// Each byte in `tx_buffer` is a UART transfer word of 8 or fewer
            /// bits.  The word width is determined by the UART configuration,
            /// truncating any more significant bits. E.g., 0x18f transmitted in
            /// 8N1 will be sent as 0x8f and in 7N1 will be sent as 0x0f. Clients
            /// that need to transfer 9-bit words should use `transmit_word`.
            ///
            /// Calling `transmit_buffer` while there is an outstanding
            /// `transmit_buffer` or `transmit_word` operation will return EBUSY.
            fn transmit_buffer(&self, tx_buffer: &'static mut [u8],
                               tx_len: usize)
            -> (ReturnCode, Option<&'static mut [u8]>);
            /// Transmit a single word of data asynchronously. The word length is
            /// determined by the UART configuration: it can be 6, 7, 8, or 9 bits long.
            /// If the `ReturnCode` is SUCCESS, on completion,
            /// `transmitted_word` will be called on the `TransmitClient`.
            /// Other valid `ReturnCode` values are:
            ///  - EOFF: The underlying hardware is not available, perhaps because
            ///          because it has not been initialized or in the case of a shared
            ///          hardware USART controller because it is set up for SPI.
            ///  - EBUSY: the UART is already transmitting and has not made a
            ///           transmission callback yet.
            ///  - FAIL: not supported, or some other error.
            /// If the `ReturnCode` is not SUCCESS, no callback will be made.
            /// Calling `transmit_word` while there is an outstanding
            /// `transmit_buffer` or `transmit_word` operation will return
            /// EBUSY.
            fn transmit_word(&self, word: u32)
            -> ReturnCode;
            /// Abort an outstanding call to `transmit_word` or `transmit_buffer`.
            /// The return code indicates whether the call has fully terminated or
            /// there will be a callback. Cancelled calls to `transmit_buffer` MUST
            /// always make a callback, to return the passed buffer back to the caller.
            ///
            /// If abort_transmit returns SUCCESS, there will be no future
            /// callback and the client may retransmit immediately. If
            /// abort_transmit returns any other `ReturnCode` there will be a
            /// callback. This means that if there is no outstanding call to
            /// `transmit_word` or `transmit_buffer` then a call to
            /// `abort_transmit` returns SUCCESS. If there was a `transmit`
            /// outstanding and is cancelled successfully then `EBUSY` will
            /// be returned and a there will be a callback with a `ReturnCode`
            /// of `ECANCEL`.  If there was a reception outstanding, which is
            /// not cancelled successfully, then `FAIL` will be returned and
            /// there will be a later callback.
            ///
            /// Returns SUCCESS or
            ///  - FAIL if the outstanding call to either transmit operation could
            ///    not be synchronously cancelled. A callback will be made on the
            ///    client indicating whether the call was successfully cancelled.
            fn transmit_abort(&self)
            -> ReturnCode;
        }
        pub trait Receive<'a> {
            /// Set the receive client, which will he called when reads complete.
            fn set_receive_client(&self, client: &'a dyn ReceiveClient);
            /// Receive `rx_len` bytes into `rx_buffer`, making a callback to
            /// the `ReceiveClient` when complete.  If the `ReturnCode` of
            /// `receive_buffer`'s return tuple is SUCCESS, the `Option` will
            /// be `None` and the struct will issue a `received_buffer`
            /// callback in the future. If the value of the `ReturnCode` is
            /// not SUCCESS, then the `rx_buffer` argument is returned in the
            /// `Option`. Other valid return values are:
            ///  - EOFF: The underlying hardware is not available, perhaps because
            ///          because it has not been initialized or in the case of a shared
            ///          hardware USART controller because it is set up for SPI.
            ///  - EBUSY: the UART is already receiving and has not made a
            ///           reception `complete` callback yet.
            ///  - ESIZE : `rx_len` is larger than the passed slice.
            /// Each byte in `rx_buffer` is a UART transfer word of 8 or fewer
            /// bits.  The width is determined by the UART
            /// configuration. Clients that need to transfer 9-bit words
            /// should use `receive_word`.  Calling `receive_buffer` while
            /// there is an outstanding `receive_buffer` or `receive_word`
            /// operation will return EBUSY.
            fn receive_buffer(&self, rx_buffer: &'static mut [u8],
                              rx_len: usize)
            -> (ReturnCode, Option<&'static mut [u8]>);
            /// Receive a single word of data. The word length is determined
            /// by the UART configuration: it can be 6, 7, 8, or 9 bits long.
            /// If the `ReturnCode` is SUCCESS, on completion,
            /// `received_word` will be called on the `ReceiveClient`.
            /// Other valid `ReturnCode` values are:
            ///  - EOFF: The underlying hardware is not available, perhaps because
            ///          because it has not been initialized or in the case of a shared
            ///          hardware USART controller because it is set up for SPI.
            ///  - EBUSY: the UART is already receiving and has not made a
            ///           reception callback yet.
            ///  - FAIL: not supported or some other error.
            /// Calling `receive_word` while there is an outstanding
            /// `receive_buffer` or `receive_word` operation will return
            /// EBUSY.
            fn receive_word(&self)
            -> ReturnCode;
            /// Abort any ongoing receive transfers and return what is in the
            /// receive buffer with the `receive_complete` callback. If
            /// SUCCESS is returned, there will be no callback (no call to
            /// `receive` was outstanding). If there was a `receive`
            /// outstanding, which is cancelled successfully then `EBUSY` will
            /// be returned and a there will be a callback with a `ReturnCode`
            /// of `ECANCEL`.  If there was a reception outstanding, which is
            /// not cancelled successfully, then `FAIL` will be returned and
            /// there will be a later callback.
            fn receive_abort(&self)
            -> ReturnCode;
        }
        /// Trait implemented by a UART transmitter to receive callbacks when
        /// operations complete.
        pub trait TransmitClient {
            /// A call to `Transmit::transmit_word` completed. The `ReturnCode`
            /// indicates whether the word was successfully transmitted. A call
            /// to `transmit_word` or `transmit_buffer` made within this callback
            /// SHOULD NOT return EBUSY: when this callback is made the UART should
            /// be ready to receive another call.
            ///
            /// `rval` is SUCCESS if the word was successfully transmitted, or
            ///   - ECANCEL if the call to `transmit_word` was cancelled and
            ///     the word was not transmitted.
            ///   - FAIL if the transmission failed in some way.
            fn transmitted_word(&self, _rval: ReturnCode) { }
            /// A call to `Transmit::transmit_buffer` completed. The `ReturnCode`
            /// indicates whether the buffer was successfully transmitted. A call
            /// to `transmit_word` or `transmit_buffer` made within this callback
            /// SHOULD NOT return EBUSY: when this callback is made the UART should
            /// be ready to receive another call.
            ///
            /// The `tx_len` argument specifies how many words were transmitted.
            /// An `rval` of SUCCESS indicates that every requested word was
            /// transmitted: `tx_len` in the callback should be the same as
            /// `tx_len` in the initiating call.
            ///
            /// `rval` is SUCCESS if the full buffer was successfully transmitted, or
            ///   - ECANCEL if the call to `transmit_buffer` was cancelled and
            ///     the buffer was not fully transmitted. `tx_len` contains
            ///     how many words were transmitted.
            ///   - ESIZE if the buffer could only be partially transmitted. `tx_len`
            ///     contains how many words were transmitted.
            ///   - FAIL if the transmission failed in some way.
            fn transmitted_buffer(&self, tx_buffer: &'static mut [u8],
                                  tx_len: usize, rval: ReturnCode);
        }
        pub trait ReceiveClient {
            /// A call to `Receive::receive_word` completed. The `ReturnCode`
            /// indicates whether the word was successfully received. A call
            /// to `receive_word` or `receive_buffer` made within this callback
            /// SHOULD NOT return EBUSY: when this callback is made the UART should
            /// be ready to receive another call.
            ///
            /// `rval` SUCCESS if the word was successfully received, or
            ///   - ECANCEL if the call to `receive_word` was cancelled and
            ///     the word was not received: `word` should be ignored.
            ///   - FAIL if the reception failed in some way and `word`
            ///     should be ignored. `error` may contain further information
            ///     on the sort of error.
            fn received_word(&self, _word: u32, _rval: ReturnCode,
                             _error: Error) {
            }
            /// A call to `Receive::receive_buffer` completed. The `ReturnCode`
            /// indicates whether the buffer was successfully received. A call
            /// to `receive_word` or `receive_buffer` made within this callback
            /// SHOULD NOT return EBUSY: when this callback is made the UART should
            /// be ready to receive another call.
            ///
            /// The `rx_len` argument specifies how many words were transmitted.
            /// An `rval` of SUCCESS indicates that every requested word was
            /// received: `rx_len` in the callback should be the same as
            /// `rx_len` in the initiating call.
            ///
            /// `rval` is SUCCESS if the full buffer was successfully received, or
            ///   - ECANCEL if the call to `received_buffer` was cancelled and
            ///     the buffer was not fully received. `rx_len` contains
            ///     how many words were received.
            ///   - ESIZE if the buffer could only be partially received. `rx_len`
            ///     contains how many words were transmitted.
            ///   - FAIL if reception failed in some way: `error` may contain further
            ///     information.
            fn received_buffer(&self, rx_buffer: &'static mut [u8],
                               rx_len: usize, rval: ReturnCode, error: Error);
        }
        /// Trait that isn't required for basic UART operation, but provides useful
        /// abstractions that capsules may want to be able to leverage.
        ///
        /// The interfaces are included here because some hardware platforms may be able
        /// to directly implement them, while others platforms may need to emulate them
        /// in software. The ones that can implement them in hardware should be able to
        /// leverage that efficiency, and by placing the interfaces here in the HIL they
        /// can do that.
        ///
        /// Other interface ideas that have been discussed, but are not included due to
        /// the lack of a clear use case, but are noted here in case they might help
        /// someone in the future:
        /// - `receive_until_terminator`: This would read in bytes until a specified
        ///   byte is received (or the buffer is full) and then return to the client.
        /// - `receive_len_then_message`: This would do a one byte read to get a length
        ///   byte and then read that many more bytes from UART before returning to the
        ///   client.
        pub trait ReceiveAdvanced<'a>: Receive<'a> {
            /// Receive data until `interbyte_timeout` bit periods have passed since the
            /// last byte or buffer is full. Does not timeout until at least one byte
            /// has been received.
            ///
            /// * `interbyte_timeout`: number of bit periods since last data received.
            fn receive_automatic(&self, rx_buffer: &'static mut [u8],
                                 rx_len: usize, interbyte_timeout: u8)
            -> (ReturnCode, Option<&'static mut [u8]>);
        }
    }
    pub mod usb {
        //! Interface to USB controller hardware
        use crate::common::cells::VolatileCell;
        /// USB controller interface
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
        /// USB controller client interface
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

            /// The Setup request was handled successfully
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
        #[automatically_derived]
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

            /// A packet of the given size was written into the endpoint buffer
            Packet(usize, bool),

            /// The client is not yet able to provide data to the host, but may
            /// be able to in the future.  This result causes the controller
            /// to send a NAK token to the host.
            Delay,

            /// The client does not support the request.  This result causes the
            /// controller to send a STALL token to the host.
            Error,
        }
        pub enum CtrlOutResult {

            /// Data received (send ACK)
            Ok,

            /// Not ready yet (send NAK)
            Delay,

            /// In halt state (send STALL)
            Halted,
        }
        pub enum BulkInResult {

            /// A packet of the given size was written into the endpoint buffer
            Packet(usize),

            /// The client is not yet able to provide data to the host, but may
            /// be able to in the future.  This result causes the controller
            /// to send a NAK token to the host.
            Delay,

            /// The client does not support the request.  This result causes the
            /// controller to send a STALL token to the host.
            Error,
        }
        pub enum BulkOutResult {

            /// The OUT packet was consumed
            Ok,

            /// The client is not yet able to consume data from the host, but may
            /// be able to in the future.  This result causes the controller
            /// to send a NAK token to the host.
            Delay,

            /// The client does not support the request.  This result causes the
            /// controller to send a STALL token to the host.
            Error,
        }
    }
    pub mod watchdog {
        //! Interface for a watchdog timer.
        pub trait Watchdog {
            /// Enable the watchdog timer. Period is the time in milliseconds
            /// the watchdog will timeout if not serviced.
            fn start(&self, period: usize);
            /// Disable the watchdog timer.
            fn stop(&self);
            /// Service the watchdog to let the hardware know the application
            /// is still executing.
            fn tickle(&self);
        }
    }
    /// Shared interface for configuring components.
    pub trait Controller {
        type
        Config;
        fn configure(&self, _: Self::Config);
    }
}
pub mod introspection {
    //! Mechanism for inspecting the status of the kernel.
    //!
    //! In particular this provides functions for getting the status of processes
    //! on the board. It potentially could be expanded to other kernel state.
    //!
    //! To restrict access on what can use this module, even though it is public (in
    //! a Rust sense) so it is visible outside of this crate, the introspection
    //! functions require the caller have the correct capability to call the
    //! functions. This prevents arbitrary capsules from being able to use this
    //! module, and only capsules that the board author has explicitly passed the
    //! correct capabilities to can use it.
    use core::cell::Cell;
    use crate::callback::AppId;
    use crate::capabilities::ProcessManagementCapability;
    use crate::common::cells::NumericCellExt;
    use crate::process;
    use crate::sched::Kernel;
    /// This struct provides the inspection functions.
    pub struct KernelInfo {
        kernel: &'static Kernel,
    }
    impl KernelInfo {
        pub fn new(kernel: &'static Kernel) -> KernelInfo {
            KernelInfo{kernel: kernel,}
        }
        /// Returns how many processes have been loaded on this platform. This is
        /// functionally equivalent to how many of the process slots have been used
        /// on the board. This does not consider what state the process is in, as
        /// long as it has been loaded.
        pub fn number_loaded_processes(&self,
                                       _capability:
                                           &dyn ProcessManagementCapability)
         -> usize {
            let count: Cell<usize> = Cell::new(0);
            self.kernel.process_each(|_| count.increment());
            count.get()
        }
        /// Returns how many processes are considered to be active. This includes
        /// processes in the `Running` and `Yield` states. This does not include
        /// processes which have faulted, or processes which the kernel is no longer
        /// scheduling because they have faulted too frequently or for some other
        /// reason.
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
        /// Returns how many processes are considered to be inactive. This includes
        /// processes in the `Fault` state and processes which the kernel is not
        /// scheduling for any reason.
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
        /// Get the name of the process.
        pub fn process_name(&self, app: AppId,
                            _capability: &dyn ProcessManagementCapability)
         -> &'static str {
            self.kernel.process_map_or("unknown", app.idx(),
                                       |process| process.get_process_name())
        }
        /// Returns the number of syscalls the app has called.
        pub fn number_app_syscalls(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           process.debug_syscall_count())
        }
        /// Returns the number of dropped callbacks the app has experience.
        /// Callbacks can be dropped if the queue for the app is full when a capsule
        /// tries to schedule a callback.
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
        /// Returns the number of time this app has been restarted.
        pub fn number_app_restarts(&self, app: AppId,
                                   _capability:
                                       &dyn ProcessManagementCapability)
         -> usize {
            self.kernel.process_map_or(0, app.idx(),
                                       |process|
                                           process.debug_restart_count())
        }
        /// Returns the number of time this app has exceeded its timeslice.
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
        /// Returns the total number of times all processes have exceeded
        /// their timeslices.
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
    //! Inter-process communication mechanism for Tock.
    //!
    //! This is a special syscall driver that allows userspace applications to
    //! share memory.
    use crate::callback::{AppId, Callback};
    use crate::capabilities::MemoryAllocationCapability;
    use crate::driver::Driver;
    use crate::grant::Grant;
    use crate::mem::{AppSlice, Shared};
    use crate::process;
    use crate::returncode::ReturnCode;
    use crate::sched::Kernel;
    /// Syscall number
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
        /// subscribe enables processes using IPC to register callbacks that fire
        /// when notify() is called.
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
        /// command is how notify() is implemented.
        /// Notifying an IPC service is done by setting client_or_svc to 0,
        /// and notifying an IPC client is done by setting client_or_svc to 1.
        /// In either case, the target_id is the same number as provided in a notify
        /// callback or as returned by allow.
        ///
        /// Returns EINVAL if the other process doesn't exist.
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
        /// allow enables processes to discover IPC services on the platform or
        /// share buffers with existing services.
        ///
        /// If allow is called with target_id == 0, it is an IPC service discover
        /// call. The contents of the slice should be the string name of the IPC
        /// service. If this mechanism can find that service, allow will return
        /// an ID that can be used to notify that service. Otherwise an error will
        /// be returned.
        ///
        /// If allow is called with target_id >= 1, it is a share command where the
        /// application is explicitly sharing a slice with an IPC service (as
        /// specified by the target_id). allow() simply allows both processes to
        /// access the buffer, it does not signal the service.
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
    //! Tock syscall number definitions and arch-agnostic interface trait.
    use core::fmt::Write;
    use crate::process;
    /// The syscall number assignments.
    pub enum Syscall {

        /// Return to the kernel to allow other processes to execute or to wait for
        /// interrupts and callbacks.
        ///
        /// SVC_NUM = 0
        YIELD,

        /// Pass a callback function to the kernel.
        ///
        /// SVC_NUM = 1
        SUBSCRIBE {
            driver_number: usize,
            subdriver_number: usize,
            callback_ptr: *mut (),
            appdata: usize,
        },

        /// Instruct the kernel or a capsule to perform an operation.
        ///
        /// SVC_NUM = 2
        COMMAND {
            driver_number: usize,
            subdriver_number: usize,
            arg0: usize,
            arg1: usize,
        },

        /// Share a memory buffer with the kernel.
        ///
        /// SVC_NUM = 3
        ALLOW {
            driver_number: usize,
            subdriver_number: usize,
            allow_address: *mut u8,
            allow_size: usize,
        },

        /// Various memory operations.
        ///
        /// SVC_NUM = 4
        MEMOP {
            operand: usize,
            arg0: usize,
        },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Syscall { }
    #[automatically_derived]
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
    #[automatically_derived]
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
    #[automatically_derived]
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
    /// Why the process stopped executing and execution returned to the kernel.
    pub enum ContextSwitchReason {

        /// Process called a syscall. Also returns the syscall and relevant values.
        SyscallFired {
            syscall: Syscall,
        },

        /// Process triggered the hardfault handler.
        Fault,

        /// Process exceeded its timeslice.
        TimesliceExpired,

        /// Process interrupted (e.g. by a hardware event)
        Interrupted,
    }
    #[automatically_derived]
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
    /// This trait must be implemented by the architecture of the chip Tock is
    /// running on. It allows the kernel to manage switching to and from processes
    /// in an architecture-agnostic manner.
    pub trait UserspaceKernelBoundary {
        /// Some architecture-specific struct containing per-process state that must
        /// be kept while the process is not running. For example, for keeping CPU
        /// registers that aren't stored on the stack.
        type
        StoredState: Default +
        Copy;
        /// Called by the kernel after a new process has been created by before it
        /// is allowed to begin executing. Allows for architecture-specific process
        /// setup, e.g. allocating a syscall stack frame.
        unsafe fn initialize_new_process(&self, stack_pointer: *const usize,
                                         stack_size: usize,
                                         state: &mut Self::StoredState)
        -> Result<*const usize, ()>;
        /// Set the return value the process should see when it begins executing
        /// again after the syscall. This will only be called after a process has
        /// called a syscall.
        ///
        /// To help implementations, both the current stack pointer of the process
        /// and the saved state for the process are provided. The `return_value` is
        /// the value that should be passed to the process so that when it resumes
        /// executing it knows the return value of the syscall it called.
        unsafe fn set_syscall_return_value(&self, stack_pointer: *const usize,
                                           state: &mut Self::StoredState,
                                           return_value: isize);
        /// Set the function that the process should execute when it is resumed.
        /// This has two major uses: 1) sets up the initial function call to
        /// `_start` when the process is started for the very first time; 2) tells
        /// the process to execute a callback function after calling `yield()`.
        ///
        /// **Note:** This method cannot be called in conjunction with
        /// `set_syscall_return_value`, as the injected function will clobber the
        /// return value.
        ///
        /// ### Arguments
        ///
        /// - `stack_pointer` is the address of the stack pointer for the current
        ///   app.
        /// - `remaining_stack_memory` is the number of bytes below the
        ///   `stack_pointer` that is allocated for the process. This value is
        ///   checked by the implementer to ensure that there is room for this stack
        ///   frame without overflowing the stack.
        /// - `state` is the stored state for this process.
        /// - `callback` is the function that should be executed when the process
        ///   resumes.
        ///
        /// ### Return
        ///
        /// Returns `Ok` or `Err` with the current address of the stack pointer for
        /// the process. One reason for returning `Err` is that adding the function
        /// call requires adding to the stack, and there is insufficient room on the
        /// stack to add the function call.
        unsafe fn set_process_function(&self, stack_pointer: *const usize,
                                       remaining_stack_memory: usize,
                                       state: &mut Self::StoredState,
                                       callback: process::FunctionCall)
        -> Result<*mut usize, *mut usize>;
        /// Context switch to a specific process.
        ///
        /// This returns a tuple:
        /// - The new stack pointer address of the process.
        /// - Why the process stopped executing and switched back to the kernel.
        unsafe fn switch_to_process(&self, stack_pointer: *const usize,
                                    state: &mut Self::StoredState)
        -> (*mut usize, ContextSwitchReason);
        /// Display any general information about the fault.
        unsafe fn fault_fmt(&self, writer: &mut dyn Write);
        /// Display architecture specific (e.g. CPU registers or status flags) data
        /// for a process identified by its stack pointer.
        unsafe fn process_detail_fmt(&self, stack_pointer: *const usize,
                                     state: &Self::StoredState,
                                     writer: &mut dyn Write);
    }
    /// Helper function for converting raw values passed back from an application
    /// into a `Syscall` type in Tock.
    ///
    /// Different architectures may have different mechanisms for passing
    /// information about what syscall an app called, but they will have have to
    /// convert the series of raw values into a more useful Rust type. While
    /// implementations are free to do this themselves, this provides a generic
    /// helper function which should help reduce duplicated code.
    ///
    /// The mappings between raw `syscall_number` values and the associated syscall
    /// type are specified and fixed by Tock. After that, this function only
    /// converts raw values to more meaningful types based on the syscall.
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
    //! Data structure for storing a callback to userspace or kernelspace.
    use core::fmt;
    use core::ptr::NonNull;
    use crate::process;
    use crate::sched::Kernel;
    /// Userspace app identifier.
    pub struct AppId {
        crate kernel: &'static Kernel,
        idx: usize,
    }
    #[automatically_derived]
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
    #[automatically_derived]
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
        /// Returns the full address of the start and end of the flash region that
        /// the app owns and can write to. This includes the app's code and data and
        /// any padding at the end of the app. It does not include the TBF header,
        /// or any space that the kernel is using for any potential bookkeeping.
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
    /// Type to uniquely identify a callback subscription across all drivers.
    ///
    /// This contains the driver number and the subscribe number within the driver.
    pub struct CallbackId {
        pub driver_num: usize,
        pub subscribe_num: usize,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for CallbackId { }
    #[automatically_derived]
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
    #[automatically_derived]
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
    #[automatically_derived]
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
    /// Type for calling a callback in a process.
    ///
    /// This is essentially a wrapper around a function pointer.
    pub struct Callback {
        app_id: AppId,
        callback_id: CallbackId,
        appdata: usize,
        fn_ptr: NonNull<*mut ()>,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Callback { }
    impl Callback {
        crate fn new(app_id: AppId, callback_id: CallbackId, appdata: usize,
                     fn_ptr: NonNull<*mut ()>) -> Callback {
            Callback{app_id, callback_id, appdata, fn_ptr,}
        }
        /// Actually trigger the callback.
        ///
        /// This will queue the `Callback` for the associated process. It returns
        /// `false` if the queue for the process is full and the callback could not
        /// be scheduled.
        ///
        /// The arguments (`r0-r2`) are the values passed back to the process and
        /// are specific to the individual `Driver` interfaces.
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
    //! System call interface for userspace applications.
    //!
    //! Drivers implement these interfaces to expose operations to applications.
    //!
    //! # System-call Overview
    //!
    //! Tock supports four system calls. The `yield` system call is handled entirely
    //! by the scheduler, while three others are passed along to drivers:
    //!
    //!   * `subscribe` lets an application pass a callback to the driver to be
    //!   called later, when an event has occurred or data of interest is available.
    //!
    //!   * `command` tells the driver to do something immediately.
    //!
    //!   * `allow` provides the driver access to an application buffer.
    //!
    //! ## Mapping system-calls to drivers
    //!
    //! Each of these three system calls takes at least two parameters. The first is
    //! a _driver major number_ and tells the scheduler which driver to forward the
    //! system call to. The second parameters is a _driver minor number_ and is used
    //! by the driver to differentiate system calls with different driver-specific
    //! meanings (e.g. `subscribe` to "data ready" vs `subscribe` to "send
    //! complete"). The mapping between _driver major numbers_ and drivers is
    //! determined by a particular platform, while the _driver minor number_ is
    //! driver-specific.
    //!
    //! One convention in Tock is that _driver minor number_ 0 for the `command`
    //! syscall can always be used to determine if the driver is supported by
    //! the running kernel by checking the return code. If the return value is
    //! greater than or equal to zero then the driver is present. Typically this is
    //! implemented by a null command that only returns 0, but in some cases the
    //! command can also return more information, like the number of supported
    //! devices (useful for things like the number of LEDs).
    //!
    //! # The `yield` System-call
    //!
    //! While drivers do not handle the `yield` system call, it is important to
    //! understand its function and how it interacts with `subscribe`.
    use crate::callback::{AppId, Callback};
    use crate::mem::{AppSlice, Shared};
    use crate::returncode::ReturnCode;
    /// `Driver`s implement the three driver-specific system calls: `subscribe`,
    /// `command` and `allow`.
    ///
    /// See [the module level documentation](index.html) for an overview of how
    /// system calls are assigned to drivers.
    pub trait Driver {
        /// `subscribe` lets an application pass a callback to the driver to be
        /// called later. This returns `ENOSUPPORT` if not used.
        ///
        /// Calls to subscribe should do minimal synchronous work.  Instead, they
        /// should defer most work and returns results to the application via the
        /// callback. For example, a subscribe call might setup a DMA transfer to
        /// read from a sensor, and asynchronously respond to the application by
        /// passing the result to the application via the callback.
        ///
        /// Drivers should allow each application to register a single callback for
        /// each minor number subscription. Thus, a second call to subscribe from
        /// the same application would replace a previous callback.
        ///
        /// This pushes most per-application virtualization to the application
        /// itself. For example, a timer driver exposes only one timer to each
        /// application, and the application is responsible for virtualizing that
        /// timer if it needs to.
        ///
        /// The driver should signal success or failure through the sign of the
        /// return value from `subscribe`. A negative return value signifies an
        /// error, while positive a return values signifies success. In addition,
        /// the magnitude of the return value of can signify extra information such
        /// as error type.
        #[allow(unused_variables)]
        fn subscribe(&self, minor_num: usize, callback: Option<Callback>,
                     app_id: AppId) -> ReturnCode {
            ReturnCode::ENOSUPPORT
        }
        /// `command` instructs a driver to perform some action synchronously. This
        /// returns `ENOSUPPORT` if not used.
        ///
        /// The return value should reflect the result of an action. For example,
        /// enabling/disabling a peripheral should return a success or error code.
        /// Reading the current system time should return the time as an integer.
        ///
        /// Commands should not execute long running tasks synchronously. However,
        /// commands might "kick-off" asynchronous tasks in coordination with a
        /// `subscribe` call.
        ///
        /// All drivers must support the command with `minor_num` 0, and return 0
        /// or greater if the driver is supported. This command should not have any
        /// side effects. This convention ensures that applications can query the
        /// kernel for supported drivers on a given platform.
        #[allow(unused_variables)]
        fn command(&self, minor_num: usize, r2: usize, r3: usize,
                   caller_id: AppId) -> ReturnCode {
            ReturnCode::ENOSUPPORT
        }
        /// `allow` lets an application give the driver access to a buffer in the
        /// application's memory. This returns `ENOSUPPORT` if not used.
        ///
        /// The buffer is __shared__ between the application and driver, meaning the
        /// driver should not rely on the contents of the buffer to remain
        /// unchanged.
        #[allow(unused_variables)]
        fn allow(&self, app: AppId, minor_num: usize,
                 slice: Option<AppSlice<Shared, u8>>) -> ReturnCode {
            ReturnCode::ENOSUPPORT
        }
    }
}
mod grant {
    //! Data structure to store a list of userspace applications.
    use core::marker::PhantomData;
    use core::mem::{align_of, size_of};
    use core::ops::{Deref, DerefMut};
    use core::ptr::{write, write_volatile, Unique};
    use crate::callback::AppId;
    use crate::process::Error;
    use crate::sched::Kernel;
    /// Region of process memory reserved for the kernel.
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
    //! Data structure for passing application memory to the kernel.
    use core::marker::PhantomData;
    use core::ops::{Deref, DerefMut};
    use core::ptr::Unique;
    use core::slice;
    use crate::callback::AppId;
    /// Type for specifying an AppSlice is hidden from the kernel.
    pub struct Private;
    #[automatically_derived]
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
    /// Type for specifying an AppSlice is shared with the kernel.
    pub struct Shared;
    #[automatically_derived]
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
    /// Base type for an AppSlice that holds the raw pointer to the memory region
    /// the app shared with the kernel.
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
    /// Buffer of memory shared from an app to the kernel.
    ///
    /// This is the type created after an app calls the `allow` syscall.
    pub struct AppSlice<L, T> {
        ptr: AppPtr<L, T>,
        len: usize,
    }
    impl <L, T> AppSlice<L, T> {
        crate fn new(ptr: *mut T, len: usize, appid: AppId)
         -> AppSlice<L, T> {
            unsafe { AppSlice{ptr: AppPtr::new(ptr, appid), len: len,} }
        }
        /// Number of bytes in the `AppSlice`.
        pub fn len(&self) -> usize { self.len }
        /// Get the raw pointer to the buffer. This will be a pointer inside of the
        /// app's memory region.
        pub fn ptr(&self) -> *const T { self.ptr.ptr.as_ptr() }
        /// Provide access to one app's AppSlice to another app. This is used for
        /// IPC.
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
    //! Implementation of the MEMOP family of syscalls.
    use crate::process::ProcessType;
    use crate::returncode::ReturnCode;
    /// Handle the `memop` syscall.
    ///
    /// ### `memop_num`
    ///
    /// - `0`: BRK. Change the location of the program break and return a
    ///   ReturnCode.
    /// - `1`: SBRK. Change the location of the program break and return the
    ///   previous break address.
    /// - `2`: Get the address of the start of the application's RAM allocation.
    /// - `3`: Get the address pointing to the first address after the end of the
    ///   application's RAM allocation.
    /// - `4`: Get the address of the start of the application's flash region. This
    ///   is where the TBF header is located.
    /// - `5`: Get the address pointing to the first address after the end of the
    ///   application's flash region.
    /// - `6`: Get the address of the lowest address of the grant region for the
    ///   app.
    /// - `7`: Get the number of writeable flash regions defined in the header of
    ///   this app.
    /// - `8`: Get the start address of the writeable region indexed from 0 by r1.
    ///   Returns (void*) -1 on failure, meaning the selected writeable region
    ///   does not exist.
    /// - `9`: Get the end address of the writeable region indexed by r1. Returns
    ///   (void*) -1 on failure, meaning the selected writeable region does not
    ///   exist.
    /// - `10`: Specify where the start of the app stack is. This tells the kernel
    ///   where the app has put the start of its stack. This is not strictly
    ///   necessary for correct operation, but allows for better debugging if the
    ///   app crashes.
    /// - `11`: Specify where the start of the app heap is. This tells the kernel
    ///   where the app has put the start of its heap. This is not strictly
    ///   necessary for correct operation, but allows for better debugging if the
    ///   app crashes.
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
        //! Interface for configuring the Memory Protection Unit.
        use core::cmp;
        use core::fmt::Display;
        /// User mode access permissions.
        pub enum Permissions {
            ReadWriteExecute,
            ReadWriteOnly,
            ReadExecuteOnly,
            ReadOnly,
            ExecuteOnly,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Permissions { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for Permissions {
            #[inline]
            fn clone(&self) -> Permissions { { *self } }
        }
        /// MPU region.
        ///
        /// This is one contiguous address space protected by the MPU.
        pub struct Region {
            /// The memory address where the region starts.
            ///
            /// For maximum compatibility, we use a u8 pointer, however, note that many
            /// memory protection units have very strict alignment requirements for the
            /// memory regions protected by the MPU.
            start_address: *const u8,
            /// The number of bytes of memory in the MPU region.
            size: usize,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for Region { }
        #[automatically_derived]
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
            /// Create a new MPU region with a given starting point and length in bytes.
            pub fn new(start_address: *const u8, size: usize) -> Region {
                Region{start_address: start_address, size: size,}
            }
            /// Getter: retrieve the address of the start of the MPU region.
            pub fn start_address(&self) -> *const u8 { self.start_address }
            /// Getter: retrieve the length of the region in bytes.
            pub fn size(&self) -> usize { self.size }
        }
        pub struct MpuConfig;
        #[automatically_derived]
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
        /// The generic trait that particular memory protection unit implementations
        /// need to implement.
        ///
        /// This trait is a blend of relatively generic MPU functionality that should be
        /// common across different MPU implementations, and more specific requirements
        /// that Tock needs to support protecting applications. While a less
        /// Tock-specific interface may be desirable, due to the sometimes complex
        /// alignment rules and other restrictions imposed by MPU hardware, some of the
        /// Tock details have to be passed into this interface. That allows the MPU
        /// implementation to have more flexibility when satisfying the protection
        /// requirements, and also allows the MPU to specify some addresses used by the
        /// kernel when deciding where to place certain application memory regions so
        /// that the MPU can appropriately provide protection for those memory regions.
        pub trait MPU {
            /// MPU-specific state that defines a particular configuration for the MPU.
            /// That is, this should contain all of the required state such that the
            /// implementation can be passed an object of this type and it should be
            /// able to correctly and entirely configure the MPU.
            ///
            /// This state will be held on a per-process basis as a way to cache all of
            /// the process settings. When the kernel switches to a new process it will
            /// use the `MpuConfig` for that process to quickly configure the MPU.
            ///
            /// It is `Default` so we can create empty state when the process is
            /// created, and `Display` so that the `panic!()` output can display the
            /// current state to help with debugging.
            type
            MpuConfig: Default +
            Display
            =
            ();
            /// Enables the MPU.
            ///
            /// This function must enable the permission restrictions on the various
            /// regions protected by the MPU.
            fn enable_mpu(&self) { }
            /// Disables the MPU.
            ///
            /// This function must completely disable any access control enforced by the
            /// MPU. This will be called before the kernel starts to execute as on some
            /// platforms the MPU rules apply to privileged code as well, and therefore
            /// the MPU must be completely disabled for the kernel to effectively manage
            /// processes.
            fn disable_mpu(&self) { }
            /// Returns the maximum number of regions supported by the MPU.
            fn number_total_regions(&self) -> usize { 0 }
            /// Allocates a new MPU region.
            ///
            /// An implementation must allocate an MPU region at least `min_region_size`
            /// bytes in size within the specified stretch of unallocated memory, and
            /// with the specified user mode permissions, and store it in `config`. The
            /// allocated region may not overlap any of the regions already stored in
            /// `config`.
            ///
            /// # Arguments
            ///
            /// - `unallocated_memory_start`: start of unallocated memory
            /// - `unallocated_memory_size`:  size of unallocated memory
            /// - `min_region_size`:          minimum size of the region
            /// - `permissions`:              permissions for the region
            /// - `config`:                   MPU region configuration
            ///
            /// # Return Value
            ///
            /// Returns the start and size of the allocated MPU region. If it is
            /// infeasible to allocate the MPU region, returns None.
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
            /// Chooses the location for a process's memory, and allocates an MPU region
            /// covering the app-owned part.
            ///
            /// An implementation must choose a contiguous block of memory that is at
            /// least `min_memory_size` bytes in size and lies completely within the
            /// specified stretch of unallocated memory.
            ///
            /// It must also allocate an MPU region with the following properties:
            ///
            /// 1. The region covers at least the first `initial_app_memory_size` bytes
            ///    at the beginning of the memory block.
            /// 2. The region does not overlap the last `initial_kernel_memory_size`
            ///    bytes.
            /// 3. The region has the user mode permissions specified by `permissions`.
            ///
            /// The end address of app-owned memory will increase in the future, so the
            /// implementation should choose the location of the process memory block
            /// such that it is possible for the MPU region to grow along with it. The
            /// implementation must store the allocated region in `config`. The
            /// allocated region may not overlap any of the regions already stored in
            /// `config`.
            ///
            /// # Arguments
            ///
            /// - `unallocated_memory_start`:   start of unallocated memory
            /// - `unallocated_memory_size`:    size of unallocated memory
            /// - `min_memory_size`:            minimum total memory to allocate for process
            /// - `initial_app_memory_size`:    initial size of app-owned memory
            /// - `initial_kernel_memory_size`: initial size of kernel-owned memory
            /// - `permissions`:                permissions for the MPU region
            /// - `config`:                     MPU region configuration
            ///
            /// # Return Value
            ///
            /// This function returns the start address and the size of the memory block
            /// chosen for the process. If it is infeasible to find a memory block or
            /// allocate the MPU region, or if the function has already been called,
            /// returns None.
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
            /// Updates the MPU region for app-owned memory.
            ///
            /// An implementation must reallocate the MPU region for app-owned memory
            /// stored in `config` to maintain the 3 conditions described in
            /// `allocate_app_memory_region`.
            ///
            /// # Arguments
            ///
            /// - `app_memory_break`:    new address for the end of app-owned memory
            /// - `kernel_memory_break`: new address for the start of kernel-owned memory
            /// - `permissions`:         permissions for the MPU region
            /// - `config`:              MPU region configuration
            ///
            /// # Return Value
            ///
            /// Returns an error if it is infeasible to update the MPU region, or if it
            /// was never created.
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
            /// Configures the MPU with the provided region configuration.
            ///
            /// An implementation must ensure that all memory locations not covered by
            /// an allocated region are inaccessible in user mode and accessible in
            /// supervisor mode.
            ///
            /// # Arguments
            ///
            /// - `config: MPU region configuration
            #[allow(unused_variables)]
            fn configure_mpu(&self, config: &Self::MpuConfig) { }
        }
        /// Implement default MPU trait for unit.
        impl MPU for () { }
    }
    crate mod systick {
        //! Interface system tick timer.
        /// Interface for the system tick timer.
        ///
        /// A system tick timer provides a countdown timer to enforce process scheduling
        /// quantums.  Implementations should have consistent timing while the CPU is
        /// active, but need not operate during sleep.
        ///
        /// On most chips, this will be implemented by the core (e.g. the ARM core), but
        /// some chips lack this optional peripheral, in which case it might be
        /// implemented by another timer or alarm controller.
        pub trait SysTick {
            /// Sets the timer as close as possible to the given interval in
            /// microseconds.
            ///
            /// Callers can assume at least a 24-bit wide clock. Specific timing is
            /// dependent on the driving clock. In practice, increments of 10ms are most
            /// accurate and values up to 400ms are valid.
            fn set_timer(&self, us: u32);
            /// Returns if there is at least `us` microseconds left
            fn greater_than(&self, us: u32)
            -> bool;
            /// Returns true if the timer has expired
            fn overflowed(&self)
            -> bool;
            /// Resets the timer
            ///
            /// Resets the timer to 0 and disables it
            fn reset(&self);
            /// Enables the timer
            ///
            /// Enabling the timer will begin a count down from the value set with
            /// `set_timer`.
            ///
            ///   * `with_interrupt` - if set, an expiring timer will fire an interrupt.
            fn enable(&self, with_interrupt: bool);
        }
        /// A dummy `SysTick` implementation in which the timer never expires.
        ///
        /// Using this implementation is functional, but will mean the scheduler cannot
        /// interrupt non-yielding processes.
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
    //! Support for creating and running userspace applications.
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
    /// Helper function to load processes from flash into an array of active
    /// processes. This is the default template for loading processes, but a board
    /// is able to create its own `load_processes()` function and use that instead.
    ///
    /// Processes are found in flash starting from the given address and iterating
    /// through Tock Binary Format headers. Processes are given memory out of the
    /// `app_memory` buffer until either the memory is exhausted or the allocated
    /// number of processes are created, with process structures placed in the
    /// provided array. How process faults are handled by the kernel is also
    /// selected.
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
    /// This trait is implemented by process structs.
    pub trait ProcessType {
        /// Returns the process's identifier
        fn appid(&self)
        -> AppId;
        /// Queue a `Task` for the process. This will be added to a per-process
        /// buffer and executed by the scheduler. `Task`s are some function the app
        /// should run, for example a callback or an IPC call.
        ///
        /// This function returns `true` if the `Task` was successfully enqueued,
        /// and `false` otherwise. This is represented as a simple `bool` because
        /// this is passed to the capsule that tried to schedule the `Task`.
        fn enqueue_task(&self, task: Task)
        -> bool;
        /// Remove the scheduled operation from the front of the queue and return it
        /// to be handled by the scheduler.
        ///
        /// If there are no `Task`s in the queue for this process this will return
        /// `None`.
        fn dequeue_task(&self)
        -> Option<Task>;
        /// Remove all scheduled callbacks for a given callback id from the task
        /// queue.
        fn remove_pending_callbacks(&self, callback_id: CallbackId);
        /// Returns the current state the process is in. Common states are "running"
        /// or "yielded".
        fn get_state(&self)
        -> State;
        /// Move this process from the running state to the yielded state.
        fn set_yielded_state(&self);
        /// Move this process from running or yielded state into the stopped state
        fn stop(&self);
        /// Move this stopped process back into its original state
        fn resume(&self);
        /// Put this process in the fault state. This will trigger the
        /// `FaultResponse` for this process to occur.
        fn set_fault_state(&self);
        /// Get the name of the process. Used for IPC.
        fn get_process_name(&self)
        -> &'static str;
        /// Change the location of the program break and reallocate the MPU region
        /// covering program memory.
        fn brk(&self, new_break: *const u8)
        -> Result<*const u8, Error>;
        /// Change the location of the program break, reallocate the MPU region
        /// covering program memory, and return the previous break address.
        fn sbrk(&self, increment: isize)
        -> Result<*const u8, Error>;
        /// The start address of allocated RAM for this process.
        fn mem_start(&self)
        -> *const u8;
        /// The first address after the end of the allocated RAM for this process.
        fn mem_end(&self)
        -> *const u8;
        /// The start address of the flash region allocated for this process.
        fn flash_start(&self)
        -> *const u8;
        /// The first address after the end of the flash region allocated for this
        /// process.
        fn flash_end(&self)
        -> *const u8;
        /// The lowest address of the grant region for the process.
        fn kernel_memory_break(&self)
        -> *const u8;
        /// How many writeable flash regions defined in the TBF header for this
        /// process.
        fn number_writeable_flash_regions(&self)
        -> usize;
        /// Get the offset from the beginning of flash and the size of the defined
        /// writeable flash region.
        fn get_writeable_flash_region(&self, region_index: usize)
        -> (u32, u32);
        /// Debug function to update the kernel on where the stack starts for this
        /// process. Processes are not required to call this through the memop
        /// system call, but it aids in debugging the process.
        fn update_stack_start_pointer(&self, stack_pointer: *const u8);
        /// Debug function to update the kernel on where the process heap starts.
        /// Also optional.
        fn update_heap_start_pointer(&self, heap_pointer: *const u8);
        /// Creates an `AppSlice` from the given offset and size in process memory.
        ///
        /// ## Returns
        ///
        /// If the buffer is null (a zero-valued offset), return None, signaling the capsule to delete
        /// the entry.  If the buffer is within the process's accessible memory, returns an AppSlice
        /// wrapping that buffer. Otherwise, returns an error `ReturnCode`.
        fn allow(&self, buf_start_addr: *const u8, size: usize)
        -> Result<Option<AppSlice<Shared, u8>>, ReturnCode>;
        /// Get the first address of process's flash that isn't protected by the
        /// kernel. The protected range of flash contains the TBF header and
        /// potentially other state the kernel is storing on behalf of the process,
        /// and cannot be edited by the process.
        fn flash_non_protected_start(&self)
        -> *const u8;
        /// Configure the MPU to use the process's allocated regions.
        fn setup_mpu(&self);
        /// Allocate a new MPU region for the process that is at least `min_region_size`
        /// bytes and lies within the specified stretch of unallocated memory.
        fn add_mpu_region(&self, unallocated_memory_start: *const u8,
                          unallocated_memory_size: usize,
                          min_region_size: usize)
        -> Option<mpu::Region>;
        /// Create new memory in the grant region, and check that the MPU region
        /// covering program memory does not extend past the kernel memory break.
        unsafe fn alloc(&self, size: usize, align: usize)
        -> Option<&mut [u8]>;
        unsafe fn free(&self, _: *mut u8);
        /// Get a pointer to the grant pointer for this grant number.
        unsafe fn grant_ptr(&self, grant_num: usize)
        -> *mut *mut u8;
        /// Set the return value the process should see when it begins executing
        /// again after the syscall.
        unsafe fn set_syscall_return_value(&self, return_value: isize);
        /// Set the function that is to be executed when the process is resumed.
        unsafe fn set_process_function(&self, callback: FunctionCall);
        /// Context switch to a specific process.
        unsafe fn switch_to(&self)
        -> Option<syscall::ContextSwitchReason>;
        unsafe fn fault_fmt(&self, writer: &mut dyn Write);
        unsafe fn process_detail_fmt(&self, writer: &mut dyn Write);
        /// Returns how many syscalls this app has called.
        fn debug_syscall_count(&self)
        -> usize;
        /// Returns how many callbacks for this process have been dropped.
        fn debug_dropped_callback_count(&self)
        -> usize;
        /// Returns how many times this process has been restarted.
        fn debug_restart_count(&self)
        -> usize;
        /// Returns how many times this process has exceeded its timeslice.
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Error { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Error {
        #[inline]
        fn clone(&self) -> Error { { *self } }
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for Error {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
    #[automatically_derived]
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
    /// Various states a process can be in.
    pub enum State {

        /// Process expects to be running code. The process may not be currently
        /// scheduled by the scheduler, but the process has work to do if it is
        /// scheduled.
        Running,

        /// Process stopped executing and returned to the kernel because it called
        /// the `yield` syscall. This likely means it is waiting for some event to
        /// occur, but it could also mean it has finished and doesn't need to be
        /// scheduled again.
        Yielded,

        /// The process is stopped, and its previous state was Running. This is used
        /// if the kernel forcibly stops a process when it is in the `Running`
        /// state. This state indicates to the kernel not to schedule the process,
        /// but if the process is to be resumed later it should be put back in the
        /// running state so it will execute correctly.
        StoppedRunning,

        /// The process is stopped, and it was stopped while it was yielded. If this
        /// process needs to be resumed it should be put back in the `Yield` state.
        StoppedYielded,

        /// The process is stopped, and it was stopped after it faulted. This
        /// basically means the app crashed, and the kernel decided to just stop it
        /// and continue executing other things.
        StoppedFaulted,

        /// The process has caused a fault.
        Fault,

        /// The process has never actually been executed. This of course happens
        /// when the board first boots and the kernel has not switched to any
        /// processes yet. It can also happen if an process is terminated and all
        /// of its state is reset as if it has not been executed yet.
        Unstarted,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for State { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for State {
        #[inline]
        fn clone(&self) -> State { { *self } }
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for State {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
    #[automatically_derived]
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
    /// The reaction the kernel should take when an app encounters a fault.
    ///
    /// When an exception occurs during an app's execution (a common example is an
    /// app trying to access memory outside of its allowed regions) the system will
    /// trap back to the kernel, and the kernel has to decide what to do with the
    /// app at that point.
    pub enum FaultResponse {

        /// Generate a `panic!()` call and crash the entire system. This is useful
        /// for debugging applications as the error is displayed immediately after
        /// it occurs.
        Panic,

        /// Attempt to cleanup and restart the app which caused the fault. This
        /// resets the app's memory to how it was when the app was started and
        /// schedules the app to run again from its init function.
        Restart,

        /// Stop the app by no longer scheduling it to run.
        Stop,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FaultResponse { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FaultResponse {
        #[inline]
        fn clone(&self) -> FaultResponse { { *self } }
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for FaultResponse {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for IPCType { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for IPCType {
        #[inline]
        fn clone(&self) -> IPCType { { *self } }
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Task { }
    #[automatically_derived]
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
    /// Enumeration to identify whether a function call comes directly from the
    /// kernel or from a callback subscribed through a driver.
    ///
    /// An example of kernel function is the application entry point.
    pub enum FunctionCallSource { Kernel, Driver(CallbackId), }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FunctionCallSource { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for FunctionCallSource {
        #[inline]
        fn clone(&self) -> FunctionCallSource {
            { let _: ::core::clone::AssertParamIsClone<CallbackId>; *self }
        }
    }
    #[automatically_derived]
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
    /// Struct that defines a callback that can be passed to a process. The callback
    /// takes four arguments that are `Driver` and callback specific, so they are
    /// represented generically here.
    ///
    /// Likely these four arguments will get passed as the first four register
    /// values, but this is architecture-dependent.
    ///
    /// A `FunctionCall` also identifies the callback that scheduled it, if any, so
    /// that it can be unscheduled when the process unsubscribes from this callback.
    pub struct FunctionCall {
        pub source: FunctionCallSource,
        pub argument0: usize,
        pub argument1: usize,
        pub argument2: usize,
        pub argument3: usize,
        pub pc: usize,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for FunctionCall { }
    #[automatically_derived]
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
    #[automatically_derived]
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
    /// State for helping with debugging apps.
    ///
    /// These pointers and counters are not strictly required for kernel operation,
    /// but provide helpful information when an app crashes.
    struct ProcessDebug {
        /// Where the process has started its heap in RAM.
        app_heap_start_pointer: Option<*const u8>,
        /// Where the start of the stack is for the process. If the kernel does the
        /// PIC setup for this app then we know this, otherwise we need the app to
        /// tell us where it put its stack.
        app_stack_start_pointer: Option<*const u8>,
        /// How low have we ever seen the stack pointer.
        min_stack_pointer: *const u8,
        /// How many syscalls have occurred since the process started.
        syscall_count: usize,
        /// What was the most recent syscall.
        last_syscall: Option<Syscall>,
        /// How many callbacks were dropped because the queue was insufficiently
        /// long.
        dropped_callback_count: usize,
        /// How many times this process has entered into a fault condition and the
        /// kernel has restarted it.
        restart_count: usize,
        /// How many times this process has been paused because it exceeded its
        /// timeslice.
        timeslice_expiration_count: usize,
    }
    pub struct Process<'a, C: 'static + Chip> {
        /// Index of the process in the process table.
        ///
        /// Corresponds to AppId
        app_idx: usize,
        /// Pointer to the main Kernel struct.
        kernel: &'static Kernel,
        /// Pointer to the struct that defines the actual chip the kernel is running
        /// on. This is used because processes have subtle hardware-based
        /// differences. Specifically, the actual syscall interface and how
        /// processes are switched to is architecture-specific, and how memory must
        /// be allocated for memory protection units is also hardware-specific.
        chip: &'static C,
        /// Application memory layout:
        ///
        /// ```text
        ///     ╒════════ ← memory[memory.len()]
        ///  ╔═ │ Grant
        ///     │   ↓
        ///  D  │ ──────  ← kernel_memory_break
        ///  Y  │
        ///  N  │ ──────  ← app_break               ═╗
        ///  A  │                                    ║
        ///  M  │   ↑                                  A
        ///     │  Heap                              P C
        ///  ╠═ │ ──────  ← app_heap_start           R C
        ///     │  Data                              O E
        ///  F  │ ──────  ← data_start_pointer       C S
        ///  I  │ Stack                              E S
        ///  X  │   ↓                                S I
        ///  E  │                                    S B
        ///  D  │ ──────  ← current_stack_pointer      L
        ///     │                                    ║ E
        ///  ╚═ ╘════════ ← memory[0]               ═╝
        /// ```
        ///
        /// The process's memory.
        memory: &'static mut [u8],
        /// Pointer to the end of the allocated (and MPU protected) grant region.
        kernel_memory_break: Cell<*const u8>,
        /// Copy of where the kernel memory break is when the app is first started.
        /// This is handy if the app is restarted so we know where to reset
        /// the kernel_memory break to without having to recalculate it.
        original_kernel_memory_break: *const u8,
        /// Pointer to the end of process RAM that has been sbrk'd to the process.
        app_break: Cell<*const u8>,
        original_app_break: *const u8,
        /// Pointer to high water mark for process buffers shared through `allow`
        allow_high_water_mark: Cell<*const u8>,
        /// Saved when the app switches to the kernel.
        current_stack_pointer: Cell<*const u8>,
        original_stack_pointer: *const u8,
        /// Process flash segment. This is the region of nonvolatile flash that
        /// the process occupies.
        flash: &'static [u8],
        /// Collection of pointers to the TBF header in flash.
        header: tbfheader::TbfHeader,
        /// State saved on behalf of the process each time the app switches to the
        /// kernel.
        stored_state: Cell<<<C as Chip>::UserspaceKernelBoundary as
                           UserspaceKernelBoundary>::StoredState>,
        /// Whether the scheduler can schedule this app.
        state: Cell<State>,
        /// How to deal with Faults occurring in the process
        fault_response: FaultResponse,
        /// Configuration data for the MPU
        mpu_config: MapCell<<<C as Chip>::MPU as MPU>::MpuConfig>,
        /// MPU regions are saved as a pointer-size pair.
        mpu_regions: [Cell<Option<mpu::Region>>; 6],
        /// Essentially a list of callbacks that want to call functions in the
        /// process.
        tasks: MapCell<RingBuffer<'a, Task>>,
        /// Name of the app.
        process_name: &'static str,
        /// Values kept so that we can print useful debug messages when apps fault.
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
        /// Checks if the buffer represented by the passed in base pointer and size
        /// are within the memory bounds currently exposed to the processes (i.e.
        /// ending at `app_break`. If this method returns true, the buffer
        /// is guaranteed to be accessible to the process and to not overlap with
        /// the grant region.
        fn in_app_owned_memory(&self, buf_start_addr: *const u8, size: usize)
         -> bool {
            let buf_end_addr = buf_start_addr.wrapping_add(size);
            buf_end_addr >= buf_start_addr &&
                buf_start_addr >= self.mem_start() &&
                buf_end_addr <= self.app_break.get()
        }
        /// Reset all `grant_ptr`s to NULL.
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
    //! Standard return type for invoking operations, returning success or an error
    //! code.
    //!
    //! - Author: Philip Levis <pal@cs.stanford.edu>
    //! - Date: Dec 22, 2016
    /// Standard return errors in Tock.
    pub enum ReturnCode {

        /// Success value must be positive
        SuccessWithValue {
            value: usize,
        },

        /// Operation completed successfully
        SUCCESS,

        /// Generic failure condition
        FAIL,

        /// Underlying system is busy; retry
        EBUSY,

        /// The state requested is already set
        EALREADY,

        /// The component is powered down
        EOFF,

        /// Reservation required before use
        ERESERVE,

        /// An invalid parameter was passed
        EINVAL,

        /// Parameter passed was too large
        ESIZE,

        /// Operation canceled by a call
        ECANCEL,

        /// Memory required not available
        ENOMEM,

        /// Operation or command is unsupported
        ENOSUPPORT,

        /// Device does not exist
        ENODEVICE,

        /// Device is not physically installed
        EUNINSTALLED,

        /// Packet transmission not acknowledged
        ENOACK,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ReturnCode {
        #[inline]
        fn clone(&self) -> ReturnCode {
            { let _: ::core::clone::AssertParamIsClone<usize>; *self }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ReturnCode { }
    #[automatically_derived]
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
    #[automatically_derived]
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
    //! Tock core scheduler.
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
    /// The time a process is permitted to run before being pre-empted
    const KERNEL_TICK_DURATION_US: u32 = 10000;
    /// Skip re-scheduling a process if its quanta is nearly exhausted
    const MIN_QUANTA_THRESHOLD_US: u32 = 500;
    /// Main object for the kernel. Each board will need to create one.
    pub struct Kernel {
        /// How many "to-do" items exist at any given time. These include
        /// outstanding callbacks and processes in the Running state.
        work: Cell<usize>,
        /// This holds a pointer to the static array of Process pointers.
        processes: &'static [Option<&'static dyn process::ProcessType>],
        /// How many grant regions have been setup. This is incremented on every
        /// call to `create_grant()`. We need to explicitly track this so that when
        /// processes are created they can allocated pointers for each grant.
        grant_counter: Cell<usize>,
        /// Flag to mark that grants have been finalized. This means that the kernel
        /// cannot support creating new grants because processes have already been
        /// created and the data structures for grants have already been
        /// established.
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
        /// Something was scheduled for a process, so there is more work to do.
        crate fn increment_work(&self) { self.work.increment(); }
        /// Something finished for a process, so we decrement how much work there is
        /// to do.
        crate fn decrement_work(&self) { self.work.decrement(); }
        /// Helper function for determining if we should service processes or go to
        /// sleep.
        fn processes_blocked(&self) -> bool { self.work.get() == 0 }
        /// Run a closure on a specific process if it exists. If the process does
        /// not exist (i.e. it is `None` in the `processes` array) then `default`
        /// will be returned. Otherwise the closure will executed and passed a
        /// reference to the process.
        crate fn process_map_or<F,
                                R>(&self, default: R, process_index: usize,
                                   closure: F) -> R where
         F: FnOnce(&dyn process::ProcessType) -> R {
            if process_index > self.processes.len() { return default; }
            self.processes[process_index].map_or(default,
                                                 |process| closure(process))
        }
        /// Run a closure on every valid process. This will iterate the array of
        /// processes and call the closure on every process that exists.
        crate fn process_each<F>(&self, closure: F) where
         F: Fn(&dyn process::ProcessType) {
            for process in self.processes.iter() {
                match process { Some(p) => { closure(*p); } None => { } }
            }
        }
        /// Run a closure on every valid process. This will iterate the
        /// array of processes and call the closure on every process that
        /// exists. Ths method is available outside the kernel crate but
        /// requires a `ProcessManagementCapability` to use.
        pub fn process_each_capability<F>(&'static self,
                                          _capability:
                                              &dyn capabilities::ProcessManagementCapability,
                                          closure: F) where
         F: Fn(usize, &dyn process::ProcessType) {
            for (i, process) in self.processes.iter().enumerate() {
                match process { Some(p) => { closure(i, *p); } None => { } }
            }
        }
        /// Run a closure on every process, but only continue if the closure returns
        /// `FAIL`. That is, if the closure returns any other return code than
        /// `FAIL`, that value will be returned from this function and the iteration
        /// of the array of processes will stop.
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
        /// Return how many processes this board supports.
        crate fn number_of_process_slots(&self) -> usize {
            self.processes.len()
        }
        /// Create a new grant. This is used in board initialization to setup grants
        /// that capsules use to interact with processes.
        ///
        /// Grants **must** only be created _before_ processes are initialized.
        /// Processes use the number of grants that have been allocated to correctly
        /// initialize the process's memory with a pointer for each grant. If a
        /// grant is created after processes are initialized this will panic.
        ///
        /// Calling this function is restricted to only certain users, and to
        /// enforce this calling this function requires the
        /// `MemoryAllocationCapability` capability.
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
        /// Returns the number of grants that have been setup in the system and
        /// marks the grants as "finalized". This means that no more grants can
        /// be created because data structures have been setup based on the number
        /// of grants when this function is called.
        ///
        /// In practice, this is called when processes are created, and the process
        /// memory is setup based on the number of current grants.
        crate fn get_grant_count_and_finalize(&self) -> usize {
            self.grants_finalized.set(true);
            self.grant_counter.get()
        }
        /// Cause all apps to fault.
        ///
        /// This will call `set_fault_state()` on each app, causing the app to enter
        /// the state as if it had crashed (for example with an MPU violation). If
        /// the process is configured to be restarted it will be.
        ///
        /// Only callers with the `ProcessManagementCapability` can call this
        /// function. This restricts general capsules from being able to call this
        /// function, since capsules should not be able to arbitrarily restart all
        /// apps.
        pub fn hardfault_all_apps<C: capabilities::ProcessManagementCapability>(&self,
                                                                                _c:
                                                                                    &C) {
            for p in self.processes.iter() {
                p.map(|process| { process.set_fault_state(); });
            }
        }
        /// Main loop.
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
    //! Tock Binary Format Header definitions and parsing code.
    use core::{mem, slice, str};
    /// Takes a value and rounds it up to be aligned % 4
    macro_rules! align4 {
        ($ e : expr) => { ($ e) + ((4 - (($ e) % 4)) % 4) } ;
    }
    /// TBF fields that must be present in all v2 headers.
    #[repr(C)]
    crate struct TbfHeaderV2Base {
        version: u16,
        header_size: u16,
        total_size: u32,
        flags: u32,
        checksum: u32,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Base { }
    #[automatically_derived]
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
    /// Types in TLV structures for each optional block of the header.
    #[repr(u16)]
    #[allow(dead_code)]
    crate enum TbfHeaderTypes {
        TbfHeaderMain = 1,
        TbfHeaderWriteableFlashRegions = 2,
        TbfHeaderPackageName = 3,
        Unused = 5,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::clone::Clone for TbfHeaderTypes {
        #[inline]
        fn clone(&self) -> TbfHeaderTypes { { *self } }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(dead_code)]
    impl ::core::marker::Copy for TbfHeaderTypes { }
    #[automatically_derived]
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
    /// The TLV header (T and L).
    #[repr(C)]
    crate struct TbfHeaderTlv {
        tipe: TbfHeaderTypes,
        length: u16,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderTlv { }
    #[automatically_derived]
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
    /// The v2 main section for apps.
    ///
    /// All apps must have a main section. Without it, the header is considered as
    /// only padding.
    #[repr(C)]
    crate struct TbfHeaderV2Main {
        init_fn_offset: u32,
        protected_size: u32,
        minimum_ram_size: u32,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2Main { }
    #[automatically_derived]
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
    /// Writeable flash regions only need an offset and size.
    ///
    /// There can be multiple (or zero) flash regions defined, so this is its own
    /// struct.
    #[repr(C)]
    crate struct TbfHeaderV2WriteableFlashRegion {
        writeable_flash_region_offset: u32,
        writeable_flash_region_size: u32,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2WriteableFlashRegion { }
    #[automatically_derived]
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
    /// Single header that can contain all parts of a v2 header.
    crate struct TbfHeaderV2 {
        base: &'static TbfHeaderV2Base,
        main: Option<&'static TbfHeaderV2Main>,
        package_name: Option<&'static str>,
        writeable_regions: Option<&'static [TbfHeaderV2WriteableFlashRegion]>,
    }
    #[automatically_derived]
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for TbfHeaderV2 { }
    #[automatically_derived]
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
    /// Type that represents the fields of the Tock Binary Format header.
    ///
    /// This specifies the locations of the different code and memory sections
    /// in the tock binary, as well as other information about the application.
    /// The kernel can also use this header to keep persistent state about
    /// the application.
    crate enum TbfHeader {
        TbfHeaderV2(TbfHeaderV2),
        Padding(&'static TbfHeaderV2Base),
    }
    #[automatically_derived]
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
        /// Return whether this is an app or just padding between apps.
        crate fn is_app(&self) -> bool {
            match *self {
                TbfHeader::TbfHeaderV2(_) => true,
                TbfHeader::Padding(_) => false,
            }
        }
        /// Return whether the application is enabled or not.
        /// Disabled applications are not started by the kernel.
        crate fn enabled(&self) -> bool {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.base.flags & 0x00000001 == 1
                }
                TbfHeader::Padding(_) => false,
            }
        }
        /// Get the total size in flash of this app or padding.
        crate fn get_total_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => hd.base.total_size,
                TbfHeader::Padding(hd) => hd.total_size,
            }
        }
        /// Add up all of the relevant fields in header version 1, or just used the
        /// app provided value in version 2 to get the total amount of RAM that is
        /// needed for this app.
        crate fn get_minimum_app_ram_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) =>
                hd.main.map_or(0, |m| m.minimum_ram_size),
                _ => 0,
            }
        }
        /// Get the number of bytes from the start of the app's region in flash that
        /// is for kernel use only. The app cannot write this region.
        crate fn get_protected_size(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.main.map_or(0, |m| m.protected_size) +
                        (hd.base.header_size as u32)
                }
                _ => 0,
            }
        }
        /// Get the offset from the beginning of the app's flash region where the
        /// app should start executing.
        crate fn get_init_function_offset(&self) -> u32 {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => {
                    hd.main.map_or(0, |m| m.init_fn_offset) +
                        (hd.base.header_size as u32)
                }
                _ => 0,
            }
        }
        /// Get the name of the app.
        crate fn get_package_name(&self) -> &'static str {
            match *self {
                TbfHeader::TbfHeaderV2(hd) => hd.package_name.unwrap_or(""),
                _ => "",
            }
        }
        /// Get the number of flash regions this app has specified in its header.
        crate fn number_writeable_flash_regions(&self) -> usize {
            match *self {
                TbfHeader::TbfHeaderV2(hd) =>
                hd.writeable_regions.map_or(0, |wr| wr.len()),
                _ => 0,
            }
        }
        /// Get the offset and size of a given flash region.
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
    /// Converts a pointer to memory to a TbfHeader struct
    ///
    /// This function takes a pointer to arbitrary memory and optionally returns a
    /// TBF header struct. This function will validate the header checksum, but does
    /// not perform sanity or security checking on the structure.
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
/// Publicly available process-related objects.
pub mod procs {
    pub use crate::process::{load_processes, FaultResponse, FunctionCall,
                             Process, ProcessType};
}
