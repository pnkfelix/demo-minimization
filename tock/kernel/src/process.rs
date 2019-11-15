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
    fn appid(&self) -> AppId;

    fn enqueue_task(&self, task: Task) -> bool;

    fn dequeue_task(&self) -> Option<Task>;

    fn remove_pending_callbacks(&self, callback_id: CallbackId);

    fn get_state(&self) -> State;

    fn set_yielded_state(&self);

    fn stop(&self);

    fn resume(&self);

    fn set_fault_state(&self);

    fn get_process_name(&self) -> &'static str;

    fn brk(&self, new_break: *const u8) -> Result<*const u8, Error>;

    fn sbrk(&self, increment: isize) -> Result<*const u8, Error>;

    fn mem_start(&self) -> *const u8;

    fn mem_end(&self) -> *const u8;

    fn flash_start(&self) -> *const u8;

    fn flash_end(&self) -> *const u8;

    fn kernel_memory_break(&self) -> *const u8;

    fn number_writeable_flash_regions(&self) -> usize;

    fn get_writeable_flash_region(&self, region_index: usize) -> (u32, u32);

    fn update_stack_start_pointer(&self, stack_pointer: *const u8);

    fn update_heap_start_pointer(&self, heap_pointer: *const u8);

    fn allow(
        &self,
        buf_start_addr: *const u8,
        size: usize,
    ) -> Result<Option<AppSlice<Shared, u8>>, ReturnCode>;

    fn flash_non_protected_start(&self) -> *const u8;

    fn setup_mpu(&self);

    fn add_mpu_region(
        &self,
        unallocated_memory_start: *const u8,
        unallocated_memory_size: usize,
        min_region_size: usize,
    ) -> Option<mpu::Region>;

    unsafe fn alloc(&self, size: usize, align: usize) -> Option<&mut [u8]>;

    unsafe fn free(&self, _: *mut u8);

    unsafe fn grant_ptr(&self, grant_num: usize) -> *mut *mut u8;

    unsafe fn set_syscall_return_value(&self, return_value: isize);

    unsafe fn set_process_function(&self, callback: FunctionCall);

    unsafe fn switch_to(&self) -> Option<syscall::ContextSwitchReason>;

    unsafe fn fault_fmt(&self, writer: &mut dyn Write);
    unsafe fn process_detail_fmt(&self, writer: &mut dyn Write);

    fn debug_syscall_count(&self) -> usize;

    fn debug_dropped_callback_count(&self) -> usize;

    fn debug_restart_count(&self) -> usize;

    fn debug_timeslice_expiration_count(&self) -> usize;

    fn debug_timeslice_expired(&self);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NoSuchApp,
    OutOfMemory,
    AddressOutOfBounds,
    KernelError, // This likely indicates a bug in the kernel and that some
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

    stored_state:
        Cell<<<C as Chip>::UserspaceKernelBoundary as UserspaceKernelBoundary>::StoredState>,

    state: Cell<State>,

    fault_response: FaultResponse,

    mpu_config: MapCell<<<C as Chip>::MPU as MPU>::MpuConfig>,

    mpu_regions: [Cell<Option<mpu::Region>>; 6],

    tasks: MapCell<RingBuffer<'a, Task>>,

    process_name: &'static str,

    debug: MapCell<ProcessDebug>,
}

impl<C: Chip> ProcessType for Process<'a, C> {
    fn appid(&self) -> AppId {
        AppId::new(self.kernel, self.app_idx)
    }

    fn enqueue_task(&self, task: Task) -> bool {
        if self.state.get() == State::Fault {
            return false;
        }

        self.kernel.increment_work();

        let ret = self.tasks.map_or(false, |tasks| tasks.enqueue(task));

        if ret == false {
            self.debug.map(|debug| {
                debug.dropped_callback_count += 1;
            });
        }

        ret
    }

    fn remove_pending_callbacks(&self, callback_id: CallbackId) {
        self.tasks.map(|tasks| {
            tasks.retain(|task| match task {
                Task::FunctionCall(function_call) => match function_call.source {
                    FunctionCallSource::Kernel => true,
                    FunctionCallSource::Driver(id) => id != callback_id,
                },
                _ => true,
            });
        });
    }

    fn get_state(&self) -> State {
        self.state.get()
    }

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
            _ => {} // Do nothing
        }
    }

    fn resume(&self) {
        match self.state.get() {
            State::StoppedRunning => self.state.set(State::Running),
            State::StoppedYielded => self.state.set(State::Yielded),
            _ => {} // Do nothing
        }
    }

    fn set_fault_state(&self) {
        self.state.set(State::Fault);

        match self.fault_response {
            FaultResponse::Panic => {
                panic!("Process {} had a fault", self.process_name);
            }
            FaultResponse::Restart => {
                let tasks_len = self.tasks.map_or(0, |tasks| tasks.len());
                for _ in 0..tasks_len {
                    self.kernel.decrement_work();
                }

                self.tasks.map(|tasks| {
                    tasks.empty();
                });

                self.debug.map(|debug| {
                    debug.restart_count += 1;

                    debug.syscall_count = 0;
                    debug.last_syscall = None;
                    debug.dropped_callback_count = 0;
                });

                let app_flash_address = self.flash_start();
                let init_fn = unsafe {
                    app_flash_address.offset(self.header.get_init_function_offset() as isize)
                        as usize
                };
                self.state.set(State::Unstarted);

                unsafe {
                    self.grant_ptrs_reset();
                }
                self.kernel_memory_break
                    .set(self.original_kernel_memory_break);

                self.app_break.set(self.original_app_break);
                self.current_stack_pointer.set(self.original_stack_pointer);

                let flash_protected_size = self.header.get_protected_size() as usize;
                let flash_app_start = app_flash_address as usize + flash_protected_size;

                self.tasks.map(|tasks| {
                    tasks.empty();
                    tasks.enqueue(Task::FunctionCall(FunctionCall {
                        source: FunctionCallSource::Kernel,
                        pc: init_fn,
                        argument0: flash_app_start,
                        argument1: self.memory.as_ptr() as usize,
                        argument2: self.memory.len() as usize,
                        argument3: self.app_break.get() as usize,
                    }));
                });

                self.kernel.increment_work();
            }
            FaultResponse::Stop => {
                let tasks_len = self.tasks.map_or(0, |tasks| tasks.len());
                for _ in 0..tasks_len {
                    self.kernel.decrement_work();
                }

                self.tasks.map(|tasks| {
                    tasks.empty();
                });

                unsafe {
                    self.grant_ptrs_reset();
                }

                self.state.set(State::StoppedFaulted);
            }
        }
    }

    fn dequeue_task(&self) -> Option<Task> {
        self.tasks.map_or(None, |tasks| {
            tasks.dequeue().map(|cb| {
                self.kernel.decrement_work();
                cb
            })
        })
    }

    fn mem_start(&self) -> *const u8 {
        self.memory.as_ptr()
    }

    fn mem_end(&self) -> *const u8 {
        unsafe { self.memory.as_ptr().add(self.memory.len()) }
    }

    fn flash_start(&self) -> *const u8 {
        self.flash.as_ptr()
    }

    fn flash_non_protected_start(&self) -> *const u8 {
        ((self.flash.as_ptr() as usize) + self.header.get_protected_size() as usize) as *const u8
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

    fn get_writeable_flash_region(&self, region_index: usize) -> (u32, u32) {
        self.header.get_writeable_flash_region(region_index)
    }

    fn update_stack_start_pointer(&self, stack_pointer: *const u8) {
        if stack_pointer >= self.mem_start() && stack_pointer < self.mem_end() {
            self.debug.map(|debug| {
                debug.app_stack_start_pointer = Some(stack_pointer);

                debug.min_stack_pointer = stack_pointer;
            });
        }
    }

    fn update_heap_start_pointer(&self, heap_pointer: *const u8) {
        if heap_pointer >= self.mem_start() && heap_pointer < self.mem_end() {
            self.debug.map(|debug| {
                debug.app_heap_start_pointer = Some(heap_pointer);
            });
        }
    }

    fn setup_mpu(&self) {
        self.mpu_config.map(|config| {
            self.chip.mpu().configure_mpu(&config);
        });
    }

    fn add_mpu_region(
        &self,
        unallocated_memory_start: *const u8,
        unallocated_memory_size: usize,
        min_region_size: usize,
    ) -> Option<mpu::Region> {
        self.mpu_config.and_then(|mut config| {
            let new_region = self.chip.mpu().allocate_region(
                unallocated_memory_start,
                unallocated_memory_size,
                min_region_size,
                mpu::Permissions::ReadWriteOnly,
                &mut config,
            );

            if new_region.is_none() {
                return None;
            }

            for region in self.mpu_regions.iter() {
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
        self.mpu_config
            .map_or(Err(Error::KernelError), |mut config| {
                if new_break < self.allow_high_water_mark.get() || new_break >= self.mem_end() {
                    Err(Error::AddressOutOfBounds)
                } else if new_break > self.kernel_memory_break.get() {
                    Err(Error::OutOfMemory)
                } else if let Err(_) = self.chip.mpu().update_app_memory_region(
                    new_break,
                    self.kernel_memory_break.get(),
                    mpu::Permissions::ReadWriteOnly,
                    &mut config,
                ) {
                    Err(Error::OutOfMemory)
                } else {
                    let old_break = self.app_break.get();
                    self.app_break.set(new_break);
                    self.chip.mpu().configure_mpu(&config);
                    Ok(old_break)
                }
            })
    }

    fn allow(
        &self,
        buf_start_addr: *const u8,
        size: usize,
    ) -> Result<Option<AppSlice<Shared, u8>>, ReturnCode> {
        if buf_start_addr == ptr::null_mut() {
            Ok(None)
        } else if self.in_app_owned_memory(buf_start_addr, size) {
            let buf_end_addr = buf_start_addr.wrapping_add(size);
            let new_water_mark = max(self.allow_high_water_mark.get(), buf_end_addr);
            self.allow_high_water_mark.set(new_water_mark);
            Ok(Some(AppSlice::new(
                buf_start_addr as *mut u8,
                size,
                self.appid(),
            )))
        } else {
            Err(ReturnCode::EINVAL)
        }
    }

    unsafe fn alloc(&self, size: usize, align: usize) -> Option<&mut [u8]> {
        self.mpu_config.and_then(|mut config| {
            let new_break_unaligned = self.kernel_memory_break.get().offset(-(size as isize));
            let alignment_mask = !(align - 1);
            let new_break = (new_break_unaligned as usize & alignment_mask) as *const u8;
            if new_break < self.app_break.get() {
                None
            } else if let Err(_) = self.chip.mpu().update_app_memory_region(
                self.app_break.get(),
                new_break,
                mpu::Permissions::ReadWriteOnly,
                &mut config,
            ) {
                None
            } else {
                self.kernel_memory_break.set(new_break);
                Some(slice::from_raw_parts_mut(new_break as *mut u8, size))
            }
        })
    }

    unsafe fn free(&self, _: *mut u8) {}

    #[allow(clippy::cast_ptr_alignment)]
    unsafe fn grant_ptr(&self, grant_num: usize) -> *mut *mut u8 {
        let grant_num = grant_num as isize;
        (self.mem_end() as *mut *mut u8).offset(-(grant_num + 1))
    }

    fn get_process_name(&self) -> &'static str {
        self.process_name
    }

    unsafe fn set_syscall_return_value(&self, return_value: isize) {
        let mut stored_state = self.stored_state.get();
        self.chip
            .userspace_kernel_boundary()
            .set_syscall_return_value(self.sp(), &mut stored_state, return_value);
        self.stored_state.set(stored_state);
    }

    unsafe fn set_process_function(&self, callback: FunctionCall) {
        let remaining_stack_bytes = self.sp() as usize - self.memory.as_ptr() as usize;

        let mut stored_state = self.stored_state.get();

        match self.chip.userspace_kernel_boundary().set_process_function(
            self.sp(),
            remaining_stack_bytes,
            &mut stored_state,
            callback,
        ) {
            Ok(stack_bottom) => {
                self.kernel.increment_work();

                self.state.set(State::Running);

                self.current_stack_pointer.set(stack_bottom as *mut u8);
                self.debug_set_max_stack_depth();
            }

            Err(bad_stack_bottom) => {
                self.debug.map(|debug| {
                    let bad_stack_bottom = bad_stack_bottom as *const u8;
                    if bad_stack_bottom < debug.min_stack_pointer {
                        debug.min_stack_pointer = bad_stack_bottom;
                    }
                });
                self.set_fault_state();
            }
        }
        self.stored_state.set(stored_state);
    }

    unsafe fn switch_to(&self) -> Option<syscall::ContextSwitchReason> {
        let mut stored_state = self.stored_state.get();
        let (stack_pointer, switch_reason) = self
            .chip
            .userspace_kernel_boundary()
            .switch_to_process(self.sp(), &mut stored_state);
        self.current_stack_pointer.set(stack_pointer as *const u8);
        self.stored_state.set(stored_state);

        self.debug.map(|debug| {
            if self.current_stack_pointer.get() < debug.min_stack_pointer {
                debug.min_stack_pointer = self.current_stack_pointer.get();
            }

            if switch_reason == syscall::ContextSwitchReason::TimesliceExpired {
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
        self.debug
            .map_or(0, |debug| debug.timeslice_expiration_count)
    }

    fn debug_timeslice_expired(&self) {
        self.debug
            .map(|debug| debug.timeslice_expiration_count += 1);
    }

    unsafe fn fault_fmt(&self, writer: &mut dyn Write) {
        self.chip.userspace_kernel_boundary().fault_fmt(writer);
    }

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

        self.chip.userspace_kernel_boundary().process_detail_fmt(
            self.sp(),
            &self.stored_state.get(),
            writer,
        );

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

fn exceeded_check(size: usize, allocated: usize) -> &'static str {
    if size > allocated {
        " EXCEEDED!"
    } else {
        "          "
    }
}

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

            process.stored_state = Cell::new(Default::default());
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
    fn sp(&self) -> *const usize {
        self.current_stack_pointer.get() as *const usize
    }

    fn in_app_owned_memory(&self, buf_start_addr: *const u8, size: usize) -> bool {
        let buf_end_addr = buf_start_addr.wrapping_add(size);

        buf_end_addr >= buf_start_addr
            && buf_start_addr >= self.mem_start()
            && buf_end_addr <= self.app_break.get()
    }

    #[allow(clippy::cast_ptr_alignment)]
    unsafe fn grant_ptrs_reset(&self) {
        let grant_ptrs_num = self.kernel.get_grant_count_and_finalize();
        for grant_num in 0..grant_ptrs_num {
            let grant_num = grant_num as isize;
            let ctr_ptr = (self.mem_end() as *mut *mut usize).offset(-(grant_num + 1));
            write_volatile(ctr_ptr, ptr::null_mut());
        }
    }

    fn debug_set_max_stack_depth(&self) {
        self.debug.map(|debug| {
            if self.current_stack_pointer.get() < debug.min_stack_pointer {
                debug.min_stack_pointer = self.current_stack_pointer.get();
            }
        });
    }
}
