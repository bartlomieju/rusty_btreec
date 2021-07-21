#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;

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
        let prev_ptr = unsafe { btree_set(self.btree, &item as *const T as *mut T as *mut c_void) };

        if prev_ptr.is_null() {
            return None;
        }

        let mut prev = std::ptr::NonNull::new(prev_ptr as *mut T).unwrap();
        Some(unsafe { prev.as_mut() })
    }

    pub fn get(&self, key: T) -> Option<&T> {
        let item_ptr = unsafe { btree_get(self.btree, &key as *const T as *mut T as *mut c_void) };

        if item_ptr.is_null() {
            return None;
        }

        let item = std::ptr::NonNull::new(item_ptr as *mut T).unwrap();
        Some(unsafe { item.as_ref() })
    }

    pub fn delete(&self, key: T) -> Option<&mut T> {
        let item_ptr =
            unsafe { btree_delete(self.btree, &key as *const T as *mut T as *mut c_void) };

        if item_ptr.is_null() {
            return None;
        }

        let mut item = std::ptr::NonNull::new(item_ptr as *mut T).unwrap();
        Some(unsafe { item.as_mut() })
    }

    pub fn ascend<I>(&self, maybe_pivot: Option<T>, iter_fn: I) -> bool
    where
        I: FnMut(&T) -> bool,
    {
        unsafe extern "C" fn iter_trampoline<T, I>(
            item: *const c_void,
            user_data: *mut c_void,
        ) -> bool
        where
            I: FnMut(&T) -> bool,
        {
            let item = &*(item as *const T);
            let iter_fn = transmute::<*mut c_void, *mut I>(user_data);
            (*iter_fn)(item)
        }

        let mut iter_fn = Box::new(iter_fn);
        let user_data = unsafe { transmute::<*mut I, *mut c_void>(&mut *iter_fn) };

        let pivot_ptr = if let Some(pivot) = maybe_pivot {
            &pivot as *const T as *mut T as *mut c_void
        } else {
            std::ptr::null::<T>() as *mut c_void
        };

        unsafe { btree_ascend(self.btree, pivot_ptr, iter_trampoline::<T, I>, user_data) }
    }
}

impl<T: Debug, F> Drop for BTreeC<T, F> {
    fn drop(&mut self) {
        unsafe { btree_free(self.btree) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[derive(Debug, Default, Clone)]
    struct User {
        first: CString,
        last: CString,
        age: i64,
    }

    impl User {
        fn new(first: &str, last: &str, age: i64) -> Self {
            Self {
                first: CString::new(first).unwrap(),
                last: CString::new(last).unwrap(),
                age,
            }
        }
    }

    #[test]
    fn btreec_new() {
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
    fn get_set() {
        #[derive(Debug, Default, Clone)]
        struct TestItem {
            key: String,
            value: i64,
            text: String,
        }

        let mut btree = BTreeC::new(|a: &TestItem, b: &TestItem| {
            eprintln!("compare {:#?} {:#?}", a, b);
            a.key.cmp(&b.key)
        });

        // we'll use this struct for querying the tree
        // value is ignored in this case, because we're only comparing keys in the `compare` function
        let key = TestItem {
            key: "foo".to_string(),
            ..Default::default()
        };

        let maybe_prev = btree.set(TestItem {
            key: "foo".to_string(),
            value: 1,
            text: "hello world".to_string()
        });
        assert!(maybe_prev.is_none());
        assert_eq!(btree.count(), 1);

        let prev = btree
            .set(TestItem {
                key: "foo".to_string(),
                value: 2,
                text: "hello world2".to_string(),
            })
            .unwrap();
        eprintln!("prev {:#?}", prev);
        assert_eq!(prev.key, "foo");
        assert_eq!(prev.value, 1);
        assert_eq!(btree.count(), 1);

        let item = btree.get(key.clone()).unwrap();
        eprintln!("item {:#?}", item);
        assert_eq!(item.value, 2);

        assert!(btree.delete(key.clone()).is_some());
        assert!(btree.get(key.clone()).is_none());
    }

    #[test]
    fn ascend_descend() {
        let mut ascending = vec![];
        // let mut ascending_with_pivot = vec![];

        let mut btree = BTreeC::new(|a: &User, b: &User| {
            let mut result = a.last.cmp(&b.last);

            if result == Ordering::Equal {
                result = a.first.cmp(&b.first);
            }

            // if result == Ordering::Equal
            eprintln!("a {:#?} b {:#?} cmp {:#?}", a, b, result);
            result
        });

        btree.set(User::new("Dale", "Murphy", 44));
        btree.set(User::new("Roger", "Craig", 68));
        btree.set(User::new("Jane", "Murphy", 47));
        assert_eq!(btree.count(), 3);

        btree.ascend(None, |item| {
            eprintln!("item {:#?}", item);
            ascending.push(format!("{:?} {:?} {}", item.first, item.last, item.age));
            true
        });

        assert_eq!(
            ascending,
            vec!["Roger Craig 68", "Dale Murphy 44", "Jane Murphy 47"]
        );

        // let pivot = User::new("", "Murphy", 0);
        // btree.ascend(Some(pivot), |item| {
        //     eprintln!("item {:#?}", item);
        //     ascending_with_pivot.push(format!("{} {} {}", item.first, item.last, item.age));
        //     true
        // });

        // assert_eq!(ascending_with_pivot, vec![
        //     "Dale Murphy 44",
        //     "Jane Murphy 47"
        // ]);
    }
}
