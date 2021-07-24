#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!("./bindings.rs");

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::transmute;
use std::ptr;

unsafe extern "C" fn compare_trampoline<T, F>(
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

unsafe extern "C" fn iter_trampoline<T, I>(item: *const c_void, user_data: *mut c_void) -> bool
where
    I: FnMut(&T) -> bool,
{
    let item = &*(item as *const T);
    let iter_fn = transmute::<*mut c_void, *mut I>(user_data);
    (*iter_fn)(item)
}

#[repr(C)]
pub struct BTreeC<T> {
    btree: *mut btree,
    compare_fn: Box<dyn Fn(&T, &T) -> Ordering>,
    _phantom: PhantomData<T>,
}

impl<T> BTreeC<T> {
    pub fn new<F: 'static>(compare_fn: Box<F>) -> Self
    where
        F: Fn(&T, &T) -> Ordering,
    {
        let user_data = unsafe { transmute::<*const F, *mut c_void>(&*compare_fn) };

        let p = unsafe {
            btree_new(
                size_of::<T>() as u64,
                0,
                compare_trampoline::<T, F>,
                user_data,
            )
        };
        Self {
            btree: p,
            compare_fn,
            _phantom: PhantomData,
        }
    }

    pub fn less(&self, a: &T, b: &T) -> bool {
        (self.compare_fn)(a, b) == Ordering::Less
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

    // TODO: should return Option<T>
    pub fn set(&mut self, mut item: T) -> Option<&mut T> {
        let item_ptr: *mut c_void = &mut item as *mut _ as *mut c_void;
        let prev_ptr = unsafe { btree_set(self.btree, item_ptr) };
        // C code has copied over the data, forget it so Rust doesn't run
        // Drop handler
        std::mem::forget(item);

        if prev_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(prev_ptr as *mut T).unwrap().as_mut() })
        }
    }

    pub fn get(&self, key: T) -> Option<&T> {
        let item_ptr = unsafe { btree_get(self.btree, &key as *const T as *mut T as *mut c_void) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_ref() })
        }
    }

    // TODO: should return Option<T>
    pub fn delete(&mut self, key: T) -> Option<&mut T> {
        let item_ptr =
            unsafe { btree_delete(self.btree, &key as *const T as *mut T as *mut c_void) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_mut() })
        }
    }

    // TODO: should return Option<T>
    pub fn pop_min(&mut self) -> Option<&mut T> {
        let item_ptr = unsafe { btree_pop_min(self.btree) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_mut() })
        }
    }

    // TODO: should return Option<T>
    pub fn pop_max(&mut self) -> Option<&mut T> {
        let item_ptr = unsafe { btree_pop_max(self.btree) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_mut() })
        }
    }

    pub fn min(&self) -> Option<&T> {
        let item_ptr = unsafe { btree_min(self.btree) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_ref() })
        }
    }

    pub fn max(&self) -> Option<&T> {
        let item_ptr = unsafe { btree_max(self.btree) };

        if item_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(item_ptr as *mut T).unwrap().as_ref() })
        }
    }

    // TODO: should return Option<T>
    pub fn load(&mut self, mut item: T) -> Option<&mut T> {
        let item_ptr: *mut c_void = &mut item as *mut _ as *mut c_void;
        let prev_ptr = unsafe { btree_load(self.btree, item_ptr) };
        // C code has copied over the data, forget it so Rust doesn't run
        // Drop handler
        std::mem::forget(item);

        if prev_ptr.is_null() {
            None
        } else {
            Some(unsafe { ptr::NonNull::new(prev_ptr as *mut T).unwrap().as_mut() })
        }
    }

    pub fn ascend<I>(&self, maybe_pivot: Option<T>, iter_fn: I) -> bool
    where
        I: FnMut(&T) -> bool,
    {
        let mut iter_fn = Box::new(iter_fn);
        let user_data = unsafe { transmute::<*mut I, *mut c_void>(&mut *iter_fn) };

        let pivot_ptr = if let Some(pivot) = &maybe_pivot {
            pivot as *const T as *mut T as *mut c_void
        } else {
            std::ptr::null::<T>() as *mut c_void
        };

        let r = unsafe { btree_ascend(self.btree, pivot_ptr, iter_trampoline::<T, I>, user_data) };
        drop(maybe_pivot);
        r
    }

    pub fn descend<I>(&self, maybe_pivot: Option<T>, iter_fn: I) -> bool
    where
        I: FnMut(&T) -> bool,
    {
        let mut iter_fn = Box::new(iter_fn);
        let user_data = unsafe { transmute::<*mut I, *mut c_void>(&mut *iter_fn) };

        let pivot_ptr = if let Some(pivot) = maybe_pivot {
            &pivot as *const T as *mut T as *mut c_void
        } else {
            std::ptr::null::<T>() as *mut c_void
        };

        unsafe { btree_descend(self.btree, pivot_ptr, iter_trampoline::<T, I>, user_data) }
    }
}

impl<T> Drop for BTreeC<T> {
    fn drop(&mut self) {
        unsafe { btree_free(self.btree) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Clone)]
    struct User {
        first: &'static str,
        last: &'static str,
        age: i64,
    }

    impl User {
        fn new(first: &'static str, last: &'static str, age: i64) -> Self {
            Self { first, last, age }
        }
    }

    #[test]
    fn btreec_new() {
        let compare_fn = Box::new(|a: &u64, b: &u64| {
            let mask = 0x12345678u64;
            let a = *a ^ mask;
            let b = *b ^ mask;
            a.cmp(&b)
        });
        let btree = BTreeC::new(compare_fn);
        assert_eq!(btree.count(), 0);
        assert_eq!(btree.height(), 0);
        assert!(!btree.oom());
    }

    #[test]
    fn basic() {
        #[derive(Debug, Default, Clone)]
        struct TestItem {
            key: String,
            text: String,
            value: i64,
        }

        let compare_fn = Box::new(|a: &TestItem, b: &TestItem| a.key.cmp(&b.key));
        let mut btree = BTreeC::new(compare_fn);

        // we'll use this struct for querying the tree
        // value is ignored in this case, because we're only comparing keys in the `compare` function
        let key = TestItem {
            key: "foo".to_string(),
            ..Default::default()
        };

        let item1 = TestItem {
            key: "foo".to_string(),
            text: "hello world".to_string(),
            value: 1,
        };
        let maybe_prev = btree.set(item1);
        assert!(maybe_prev.is_none());
        assert_eq!(btree.count(), 1);

        let item2 = TestItem {
            key: "foo".to_string(),
            text: "hello world2".to_string(),
            value: 2,
        };
        let prev = btree.set(item2).unwrap();
        // std::mem::forget(item2);
        assert_eq!(prev.key, "foo");
        assert_eq!(prev.value, 1);
        assert_eq!(btree.count(), 1);

        let item = btree.get(key.clone()).unwrap();
        assert_eq!(item.value, 2);

        assert!(btree.delete(key.clone()).is_some());
        assert!(btree.get(key).is_none());
    }

    #[test]
    fn min_max_pop() {
        let compare_fn = Box::new(|a: &i64, b: &i64| a.cmp(b));
        let mut btree = BTreeC::new(compare_fn);
        btree.set(3);
        btree.set(4);
        btree.set(5);

        let min_ = btree.min().unwrap();
        assert_eq!(min_, &3);
        let min_ = btree.pop_min().unwrap();
        assert_eq!(min_, &3);
        let max_ = btree.max().unwrap();
        assert_eq!(max_, &5);
        let max_ = btree.pop_max().unwrap();
        assert_eq!(max_, &5);
        assert_eq!(btree.count(), 1);
    }

    #[test]
    fn ascend_descend() {
        let mut ascending = vec![];
        let mut ascending_with_pivot = vec![];
        let mut descending = vec![];
        let mut descending_with_pivot = vec![];

        let compare_fn = Box::new(|a: &User, b: &User| {
            let mut result = a.last.cmp(&b.last);

            if result == Ordering::Equal {
                result = a.first.cmp(&b.first);
            }

            result
        });
        let mut btree = BTreeC::new(compare_fn);

        btree.set(User::new("Dale", "Murphy", 44));
        btree.set(User::new("Roger", "Craig", 68));
        btree.set(User::new("Jane", "Murphy", 47));
        assert_eq!(btree.count(), 3);

        btree.ascend(None, |item| {
            ascending.push(format!("{} {} {}", item.first, item.last, item.age));
            true
        });
        assert_eq!(
            ascending,
            vec!["Roger Craig 68", "Dale Murphy 44", "Jane Murphy 47"]
        );

        let pivot = User::new("", "Murphy", 0);
        btree.ascend(Some(pivot), |item| {
            ascending_with_pivot.push(format!("{} {} {}", item.first, item.last, item.age));
            true
        });
        assert_eq!(
            ascending_with_pivot,
            vec!["Dale Murphy 44", "Jane Murphy 47"]
        );

        btree.descend(None, |item| {
            descending.push(format!("{} {} {}", item.first, item.last, item.age));
            true
        });
        assert_eq!(
            descending,
            vec!["Jane Murphy 47", "Dale Murphy 44", "Roger Craig 68"]
        );

        let pivot = User::new("", "Murphy", 0);
        btree.descend(Some(pivot), |item| {
            descending_with_pivot.push(format!("{} {} {}", item.first, item.last, item.age));
            true
        });
        assert_eq!(descending_with_pivot, vec!["Roger Craig 68"]);
    }

    #[test]
    fn db_item() {
        /// DbItemOpts holds various meta information about an item.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct DbItemOpts {
            /// does this item expire?
            ex: bool,
            /// when does this item expire?
            exat: std::time::Instant,
        }

        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct DbItem {
            // the binary key
            key: &'static str,
            // the binary value
            val: &'static str,
            // optional meta information
            opts: Option<DbItemOpts>,
            // keyless item for scanning
            keyless: bool,
        }

        let mut ascending = vec![];
        let compare_fn = Box::new(|a: &DbItem, b: &DbItem| a.key.cmp(&b.key));
        let mut btree = BTreeC::new(compare_fn);

        btree.set(DbItem {
            key: "foo",
            val: "bar",
            opts: None,
            keyless: false,
        });
        btree.set(DbItem {
            key: "fizz",
            val: "buzz",
            opts: Some(DbItemOpts {
                ex: true,
                exat: std::time::Instant::now(),
            }),
            keyless: false,
        });
        assert_eq!(btree.count(), 2);

        btree.ascend(None, |item| {
            eprintln!("item {:#?}", item);
            ascending.push(format!("{} {} {:#?}", item.key, item.val, item.opts));
            true
        });
    }
}
