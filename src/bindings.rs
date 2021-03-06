use std::os::raw::c_int;
use std::os::raw::c_uint;
use std::os::raw::c_ulong;
use std::os::raw::c_void;

pub type size_t = c_ulong;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct btree {
    _unused: [u8; 0],
}

pub type IterCallback = unsafe extern "C" fn(item: *const c_void, udata: *mut c_void) -> bool;
pub type CompareCallback =
    unsafe extern "C" fn(a: *const c_void, b: *const c_void, udata: *mut c_void) -> c_int;

extern "C" {
    pub fn btree_new(
        elsize: size_t,
        max_items: size_t,
        compare: CompareCallback,
        udata: *mut c_void,
    ) -> *mut btree;
    pub fn btree_free(btree: *mut btree);
    pub fn btree_oom(btree: *mut btree) -> bool;
    pub fn btree_height(btree: *mut btree) -> size_t;
    pub fn btree_count(btree: *mut btree) -> size_t;
    pub fn btree_set(btree: *mut btree, item: *mut c_void) -> *mut c_void;
    pub fn btree_get(btree: *mut btree, key: *mut c_void) -> *mut c_void;
    pub fn btree_delete(btree: *mut btree, key: *mut c_void) -> *mut c_void;
    pub fn btree_pop_min(btree: *mut btree) -> *mut c_void;
    pub fn btree_pop_max(btree: *mut btree) -> *mut c_void;
    pub fn btree_min(btree: *mut btree) -> *mut c_void;
    pub fn btree_max(btree: *mut btree) -> *mut c_void;
    pub fn btree_load(btree: *mut btree, item: *mut c_void) -> *mut c_void;
    pub fn btree_ascend(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: IterCallback,
        udata: *mut c_void,
    ) -> bool;
    pub fn btree_descend(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: IterCallback,
        udata: *mut c_void,
    ) -> bool;
}
pub const btree_action_BTREE_STOP: btree_action = 0;
pub const btree_action_BTREE_NONE: btree_action = 1;
pub const btree_action_BTREE_DELETE: btree_action = 2;
pub const btree_action_BTREE_UPDATE: btree_action = 3;
pub type btree_action = c_uint;
extern "C" {
    pub fn btree_action_ascend(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: unsafe extern "C" fn(item: *mut c_void, udata: *mut c_void) -> btree_action,
        udata: *mut c_void,
    );
    pub fn btree_action_descend(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: unsafe extern "C" fn(item: *mut c_void, udata: *mut c_void) -> btree_action,
        udata: *mut c_void,
    );
    pub fn btree_set_hint(btree: *mut btree, item: *mut c_void, hint: *mut u64) -> *mut c_void;
    pub fn btree_get_hint(btree: *mut btree, key: *mut c_void, hint: *mut u64) -> *mut c_void;
    pub fn btree_delete_hint(btree: *mut btree, key: *mut c_void, hint: *mut u64) -> *mut c_void;
    pub fn btree_ascend_hint(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: IterCallback,
        udata: *mut c_void,
        hint: *mut u64,
    ) -> bool;
    pub fn btree_descend_hint(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: IterCallback,
        udata: *mut c_void,
        hint: *mut u64,
    ) -> bool;
    pub fn btree_action_ascend_hint(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: unsafe extern "C" fn(item: *mut c_void, udata: *mut c_void) -> btree_action,
        udata: *mut c_void,
        hint: *mut u64,
    );
    pub fn btree_action_descend_hint(
        btree: *mut btree,
        pivot: *mut c_void,
        iter: unsafe extern "C" fn(item: *mut c_void, udata: *mut c_void) -> btree_action,
        udata: *mut c_void,
        hint: *mut u64,
    );
    pub fn btree_set_allocator(
        malloc: unsafe extern "C" fn(arg1: size_t) -> *mut c_void,
        free: unsafe extern "C" fn(arg1: *mut c_void),
    );
}
