#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BTreeC<T: Debug, F> {
    btree: *mut btree,
    _compare_fn: Box<F>,
    _phantom: PhantomData<T>,
}

impl<T: Debug, F> BTreeC<T, F>
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

    pub fn oom(&self) -> bool {
        unsafe { btree_oom(self.btree) }
    }

    pub fn height(&self) -> u64 {
        unsafe { btree_height(self.btree) }
    }

    pub fn count(&self) -> u64 {
        unsafe { btree_count(self.btree) }
    }

    // The item is copied over
    pub fn set(&mut self, item: T) -> Option<&mut T> {
        let prev_pointer = unsafe { 
            btree_set(self.btree, &item as *const T as *mut T as *mut c_void)
        };

        if prev_pointer.is_null() {
            return None;
        }

        let mut prev = std::ptr::NonNull::new(prev_pointer as *mut T).unwrap();
        Some(unsafe { prev.as_mut() })
    }

    pub fn get(&self, item: T) -> Option<&T> {
        let item_pointer = unsafe { 
            btree_get(self.btree, &item as *const T as *mut T as *mut c_void)
        };

        if item_pointer.is_null() {
            return None;
        }

        let prev = std::ptr::NonNull::new(item_pointer as *mut T).unwrap();
        Some(unsafe { prev.as_ref() })
    }
}

impl<T: Debug, F> Drop for BTreeC<T, F> {
    fn drop(&mut self) {
        unsafe { btree_free(self.btree) }
    }
}

#[test]
fn not_really_a_test() {
    let mask = 0x12345678u64;
    let btree = BTreeC::new(|a: &u64, b: &u64| {
        let a = *a ^ mask;
        let b = *b ^ mask;
        a.cmp(&b)
    });
    assert_eq!(btree.count(), 0);
    assert_eq!(btree.height(), 0);
    assert!(!btree.oom());
}

#[test]
fn set() {
    #[repr(C)]
    #[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
    struct TestItem {
        key: String,
        value: i64
    }

    let mut btree = BTreeC::new(|a: &TestItem, b: &TestItem| {
        a.key.cmp(&b.key)
    });

    let maybe_prev = btree.set(TestItem { key: "foo".to_string(), value: 1 });
    assert!(maybe_prev.is_none());
    assert_eq!(btree.count(), 1);
    let prev = btree.set(TestItem { key: "foo".to_string(), value: 2 }).unwrap();
    assert_eq!(prev.key, "foo");
    assert_eq!(prev.value, 1);
    assert_eq!(btree.count(), 1);
    // value is ignored in this case, because we're only comparing keys in the `compare` function
    let item = btree.get(TestItem { key: "foo".to_string(), value: 0 }).unwrap();
    assert_eq!(item.value, 2);
}
