#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;

struct BTreeC<T, F> {
    btree: *mut btree,
    _compare_fn: Box<F>,
    _phantom: PhantomData<T>,
}

impl<T, F> BTreeC<T, F>
where
    F: Fn(&T, &T) -> Ordering,
{
    #[allow(dead_code)]
    pub fn new(compare_fn: F) -> Self {
        unsafe extern "C" fn trampoline<T, F>(
            a: *const c_void,
            b: *const c_void,
            user_data: *mut c_void,
        ) -> i32
        where
            F: Fn(&T, &T) -> Ordering,
        {
            let a = &*(a as *const T);
            let b = &*(b as *const T);
            let compare_fn = transmute::<*mut c_void, *const F>(user_data);
            let r = (*compare_fn)(a, b);
            match r {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        }

        let compare_fn = Box::new(compare_fn);
        let user_data = unsafe { transmute::<*const F, *mut c_void>(&*compare_fn) };

        let p = unsafe { btree_new(size_of::<T>() as u64, 0, trampoline::<T, F>, user_data) };
        Self {
            btree: p,
            _compare_fn: compare_fn,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> Drop for BTreeC<T, F> {
    fn drop(&mut self) {
        unsafe { btree_free(self.btree) }
    }
}

#[test]
fn not_really_a_test() {
    let mask = 0x12345678u64;
    let _btree = BTreeC::new(|a: &u64, b: &u64| {
        let a = *a ^ mask;
        let b = *b ^ mask;
        a.cmp(&b)
    });
}
