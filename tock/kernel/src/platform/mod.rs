use crate::driver::Driver;
use crate::syscall;

pub mod mpu;
crate mod systick;

pub trait Platform {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn Driver>) -> R;
}

pub trait Chip {
    type MPU: mpu::MPU;

    type UserspaceKernelBoundary: syscall::UserspaceKernelBoundary;

    type SysTick: systick::SysTick;

    fn service_pending_interrupts(&self);

    fn has_pending_interrupts(&self) -> bool;

    fn mpu(&self) -> &Self::MPU;

    fn systick(&self) -> &Self::SysTick;

    fn userspace_kernel_boundary(&self) -> &Self::UserspaceKernelBoundary;

    fn sleep(&self);

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;
}

pub trait ClockInterface {
    fn is_enabled(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
}

pub struct NoClockControl {}
impl ClockInterface for NoClockControl {
    fn is_enabled(&self) -> bool {
        true
    }
    fn enable(&self) {}
    fn disable(&self) {}
}

pub static mut NO_CLOCK_CONTROL: NoClockControl = NoClockControl {};
