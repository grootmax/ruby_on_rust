//! Ports of st.c (port unit st-A-01).

use core::ffi::{c_int, c_uchar, c_uint, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct st_hash_type {
    pub compare: Option<unsafe extern "C" fn(usize, usize) -> c_int>,
    pub hash: Option<unsafe extern "C" fn(usize) -> usize>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct st_table {
    pub entry_power: c_uchar,
    pub bin_power: c_uchar,
    pub size_ind: c_uchar,
    pub rebuilds_num: c_uchar,
    pub entries_start: c_uint,
    pub type_: *const st_hash_type,
    pub num_entries: usize,
    pub entries_bound: usize,
    pub entries: *mut st_table_entry,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct st_table_entry {
    pub hash: usize,
    pub key: usize,
    pub record: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct set_table {
    pub entry_power: c_uchar,
    pub bin_power: c_uchar,
    pub size_ind: c_uchar,
    pub rebuilds_num: c_uchar,
    pub entries_start: c_uint,
    pub type_: *const st_hash_type,
    pub num_entries: usize,
    pub entries_bound: usize,
    pub entries: *mut set_table_entry,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct set_table_entry {
    pub hash: usize,
    pub key: usize,
}

pub(crate) unsafe fn entry_equal(
    type_: *const c_void,
    entry_hash: usize,
    entry_key: usize,
    hash_val: usize,
    key: usize,
) -> c_int {
    if entry_hash != hash_val {
        return 0;
    }
    if entry_key == key {
        return 1;
    }
    if type_.is_null() {
        return 0;
    }
    let type_struct = type_ as *const st_hash_type;
    let compare = unsafe { (*type_struct).compare };
    if let Some(f) = compare {
        if unsafe { f(key, entry_key) } == 0 { 1 } else { 0 }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_st_ptr_equal_check(
    tab: *const c_void,
    entry: *const c_void,
    hash_val: usize,
    key: usize,
    res: *mut c_int,
    rebuilt_p: *mut c_int,
) {
    if tab.is_null() || entry.is_null() {
        return;
    }
    let tab_ptr = tab as *const st_table;
    let entry_ptr = entry as *const st_table_entry;
    let old_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
    let type_ptr = unsafe { (*tab_ptr).type_ } as *const c_void;
    let e_hash = unsafe { (*entry_ptr).hash };
    let e_key = unsafe { (*entry_ptr).key };
    let eq = unsafe { entry_equal(type_ptr, e_hash, e_key, hash_val, key) };
    if !res.is_null() {
        unsafe { *res = eq };
    }
    if !rebuilt_p.is_null() {
        let new_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
        unsafe { *rebuilt_p = if old_rebuilds_num != new_rebuilds_num { 1 } else { 0 } };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_st_set_ptr_equal_check(
    tab: *const c_void,
    entry: *const c_void,
    hash_val: usize,
    key: usize,
    res: *mut c_int,
    rebuilt_p: *mut c_int,
) {
    if tab.is_null() || entry.is_null() {
        return;
    }
    let tab_ptr = tab as *const set_table;
    let entry_ptr = entry as *const set_table_entry;
    let old_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
    let type_ptr = unsafe { (*tab_ptr).type_ } as *const c_void;
    let e_hash = unsafe { (*entry_ptr).hash };
    let e_key = unsafe { (*entry_ptr).key };
    let eq = unsafe { entry_equal(type_ptr, e_hash, e_key, hash_val, key) };
    if !res.is_null() {
        unsafe { *res = eq };
    }
    if !rebuilt_p.is_null() {
        let new_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
        unsafe { *rebuilt_p = if old_rebuilds_num != new_rebuilds_num { 1 } else { 0 } };
    }
}
