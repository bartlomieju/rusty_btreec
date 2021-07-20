#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

use std::marker::PhantomData;
use std::mem::size_of;

struct BTreeC<T> {
    btree: *mut btree,
    _marker: PhantomData<T>,
}

impl<T> BTreeC<T> {
    pub fn new<F>(mut compare: F) -> Self 
    where
        F: FnMut(&T, &T) -> bool
    {
        unsafe extern "C" fn trampoline<T, F>(
            a: *const c_void,
            b: *const c_void,
            _user_data: *mut c_void
        ) -> i32
        where
            F: FnMut(&T, &T) -> bool
        {
            compare(a, b) as i32
        }

        let p = unsafe {
            btree_new(
                size_of::<T>() as u64,
                0,
                trampoline::<T, F>,
                std::ptr::null_mut(),
            )
        };
        Self {
            btree: p,
            _marker: PhantomData
        }
    }
}

impl<T> Drop for BTreeC<T> {
    fn drop(&mut self) {
        unsafe {
            btree_free(self.btree)
        }
    }
}
