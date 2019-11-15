//! Tock specific `MapCell` type for sharing references.

use core::cell::{Cell, UnsafeCell};
use core::mem::MaybeUninit;
use core::ptr;

/// A mutable memory location that enforces borrow rules at runtime without
/// possible panics.
///
/// A `MapCell` is a potential reference to mutable memory. Borrow rules are
/// enforced by forcing clients to either move the memory out of the cell or
/// operate on a borrow within a closure. You can think of a `MapCell` as an
/// `Option` wrapped in a `RefCell` --- attempts to take the value from inside a
/// `MapCell` may fail by returning `None`.
pub struct MapCell<T> {
    // Since val is potentially uninitialized memory, we must be sure to check
    // `.occupied` before calling `.val.get()` or `.val.assume_init()`. See
    // [mem::MaybeUninit](https://doc.rust-lang.org/core/mem/union.MaybeUninit.html).
    val: UnsafeCell<MaybeUninit<T>>,
    occupied: Cell<bool>,
}

impl<T> MapCell<T> {
    /// Creates an empty `MapCell`
    pub const fn empty() -> MapCell<T> {
        MapCell {
            val: UnsafeCell::new(MaybeUninit::uninit()),
            occupied: Cell::new(false),
        }
    }

    /// Creates a new `MapCell` containing `value`
    pub const fn new(value: T) -> MapCell<T> {
        MapCell {
            val: UnsafeCell::new(MaybeUninit::<T>::new(value)),
            occupied: Cell::new(true),
        }
    }

    /// Returns a boolean which indicates if the MapCell is unoccupied.
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Returns a boolean which indicates if the MapCell is occupied.
    pub fn is_some(&self) -> bool {
        self.occupied.get()
    }

    /// Takes the value out of the `MapCell` leaving it empty. If
    /// the value has already been taken elsewhere (and not `replace`ed), the
    /// returned `Option` will be `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate tock_cells;
    /// use tock_cells::map_cell::MapCell;
    ///
    /// let cell = MapCell::new(1234);
    /// let x = &cell;
    /// let y = &cell;
    ///
    /// assert_eq!(x.take(), Some(1234));
    /// assert_eq!(y.take(), None);
    /// ```
    pub fn take(&self) -> Option<T> {
        if self.is_none() {
            None
        } else {
            self.occupied.set(false);
            unsafe {
                let result: MaybeUninit<T> =
                    ptr::replace(self.val.get(), MaybeUninit::<T>::uninit());
                // `result` is _initialized_ and now `self.val` is now a new uninitialized value
                Some(result.assume_init())
            }
        }
    }

    /// Puts a value into the `MapCell`.
    pub fn put(&self, val: T) {
        self.occupied.set(true);
        unsafe {
            ptr::write(self.val.get(), MaybeUninit::<T>::new(val));
        }
    }

    /// Replaces the contents of the `MapCell` with `val`. If the cell was not
    /// empty, the previous value is returned, otherwise `None` is returned.
    pub fn replace(&self, val: T) -> Option<T> {
        if self.is_none() {
            self.put(val);
            None
        } else {
            unsafe {
                let result: MaybeUninit<T> = ptr::replace(self.val.get(), MaybeUninit::new(val));
                // `result` is _initialized_ and now `self.val` is now a new uninitialized value
                Some(result.assume_init())
            }
        }
    }

    /// Allows `closure` to borrow the contents of the `MapCell` if-and-only-if
    /// it is not `take`n already. The state of the `MapCell` is unchanged
    /// after the closure completes.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate tock_cells;
    /// use tock_cells::map_cell::MapCell;
    ///
    /// let cell = MapCell::new(1234);
    /// let x = &cell;
    /// let y = &cell;
    ///
    /// x.map(|value| {
    ///     // We have mutable access to the value while in the closure
    ///     *value += 1;
    /// });
    ///
    /// // After the closure completes, the mutable memory is still in the cell,
    /// // but potentially changed.
    /// assert_eq!(y.take(), Some(1235));
    /// ```
    pub fn map<F, R>(&self, closure: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if self.is_some() {
            self.occupied.set(false);
            let valref = unsafe { &mut *self.val.get() };
            // TODO: change to valref.get_mut() once stabilized [#53491](https://github.com/rust-lang/rust/issues/53491)
            let res = closure(unsafe { &mut *valref.as_mut_ptr() });
            self.occupied.set(true);
            Some(res)
        } else {
            None
        }
    }

    pub fn map_or<F, R>(&self, default: R, closure: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.map(closure).unwrap_or(default)
    }

    /// Behaves the same as `map`, except the closure is allowed to return
    /// an `Option`.
    pub fn and_then<F, R>(&self, closure: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> Option<R>,
    {
        if self.is_some() {
            self.occupied.set(false);
            let valref = unsafe { &mut *self.val.get() };
            // TODO: change to valref.get_mut() once stabilized [#53491](https://github.com/rust-lang/rust/issues/53491)
            let res = closure(unsafe { &mut *valref.as_mut_ptr() });
            self.occupied.set(true);
            res
        } else {
            None
        }
    }

    pub fn modify_or_replace<F, G>(&self, modify: F, mkval: G)
    where
        F: FnOnce(&mut T),
        G: FnOnce() -> T,
    {
        if self.map(modify).is_none() {
            self.put(mkval());
        }
    }
}
