use crate::Ruby;
use crate::api::RubyMutator;
use crate::extra_assert;
use libc::c_int;
use mmtk::scheduler::GCWorker;
use mmtk::util::Address;
use mmtk::util::ObjectReference;
use mmtk::util::VMMutatorThread;
use mmtk::util::VMWorkerThread;

// For the C binding
pub const OBJREF_OFFSET: usize = 8;
pub const MIN_OBJ_ALIGN: usize = 8; // Even on 32-bit machine.  A Ruby object is at least 40 bytes large.

pub const GC_THREAD_KIND_WORKER: libc::c_int = 1;

const HIDDEN_SIZE_MASK: usize = 0x0000FFFFFFFFFFFF;

// An opaque type for the C counterpart.
#[allow(non_camel_case_types)]
pub struct st_table;

#[repr(C)]
pub struct HiddenHeader {
    pub prefix: usize,
}

impl HiddenHeader {
    #[inline(always)]
    pub fn is_sane(&self) -> bool {
        self.prefix & !HIDDEN_SIZE_MASK == 0
    }

    #[inline(always)]
    fn assert_sane(&self) {
        extra_assert!(
            self.is_sane(),
            "Hidden header is corrupted: {:x}",
            self.prefix
        );
    }

    pub fn payload_size(&self) -> usize {
        self.assert_sane();
        self.prefix & HIDDEN_SIZE_MASK
    }
}

/// Provide convenient methods for accessing Ruby objects.
/// TODO: Wrap C functions in `RubyUpcalls` as Rust-friendly methods.
pub struct RubyObjectAccess {
    objref: ObjectReference,
}

impl RubyObjectAccess {
    pub fn from_objref(objref: ObjectReference) -> Self {
        Self { objref }
    }

    pub fn obj_start(&self) -> Address {
        self.objref.to_raw_address().sub(Self::prefix_size())
    }

    pub fn payload_addr(&self) -> Address {
        self.objref.to_raw_address()
    }

    pub fn suffix_addr(&self) -> Address {
        self.objref.to_raw_address().add(self.payload_size())
    }

    pub fn obj_end(&self) -> Address {
        self.suffix_addr() + Self::suffix_size()
    }

    fn hidden_header(&self) -> &'static HiddenHeader {
        unsafe { self.obj_start().as_ref() }
    }

    #[allow(unused)] // Maybe we need to mutate the hidden header in the future.
    fn hidden_header_mut(&self) -> &'static mut HiddenHeader {
        unsafe { self.obj_start().as_mut_ref() }
    }

    pub fn payload_size(&self) -> usize {
        self.hidden_header().payload_size()
    }

    fn flags_field(&self) -> Address {
        self.objref.to_raw_address()
    }

    pub fn load_flags(&self) -> usize {
        unsafe { self.flags_field().load::<usize>() }
    }

    pub fn prefix_size() -> usize {
        // Currently, a hidden size field of word size is placed before each object.
        OBJREF_OFFSET
    }

    pub fn suffix_size() -> usize {
        // In RACTOR_CHECK_MODE, Ruby hides a field after each object to hold the Ractor ID.
        unsafe { crate::BINDING_FAST.suffix_size }
    }

    pub fn object_size(&self) -> usize {
        Self::prefix_size() + self.payload_size() + Self::suffix_size()
    }
}

type ObjectClosureFunction =
    extern "C" fn(*mut libc::c_void, *mut libc::c_void, ObjectReference, bool) -> ObjectReference;

#[repr(C)]
pub struct ObjectClosure {
    /// The function to be called from C.
    pub c_function: ObjectClosureFunction,
    /// The pointer to the Rust-level closure object.
    pub rust_closure: *mut libc::c_void,
}

impl Default for ObjectClosure {
    fn default() -> Self {
        Self {
            c_function: THE_UNREGISTERED_CLOSURE_FUNC,
            rust_closure: std::ptr::null_mut(),
        }
    }
}

/// Rust doesn't require function items to have a unique address.
/// We therefore force using this particular constant.
///
/// See: https://rust-lang.github.io/rust-clippy/master/index.html#fn_address_comparisons
const THE_UNREGISTERED_CLOSURE_FUNC: ObjectClosureFunction = ObjectClosure::c_function_unregistered;

impl ObjectClosure {
    /// Set this ObjectClosure temporarily to `visit_object`, and execute `f`.  During the execution of
    /// `f`, the Ruby VM may call this ObjectClosure.  When the Ruby VM calls this ObjectClosure,
    /// it effectively calls `visit_object`.
    ///
    /// This method is intended to run Ruby VM code in `f` with temporarily modified behavior of
    /// `rb_gc_mark`, `rb_gc_mark_movable` and `rb_gc_location`
    ///
    /// Both `f` and `visit_object` may access and modify local variables in the environment where
    /// `set_temporarily_and_run_code` called.
    ///
    /// Note that this function is not reentrant.  Don't call this function in either `callback` or
    /// `f`.
    pub fn set_temporarily_and_run_code<'env, T, F1, F2>(
        &mut self,
        mut visit_object: F1,
        f: F2,
    ) -> T
    where
        F1: 'env + FnMut(&'static mut GCWorker<Ruby>, ObjectReference, bool) -> ObjectReference,
        F2: 'env + FnOnce() -> T,
    {
        debug_assert!(
            std::ptr::fn_addr_eq(self.c_function, THE_UNREGISTERED_CLOSURE_FUNC),
            "set_temporarily_and_run_code is recursively called."
        );
        self.c_function = Self::c_function_registered::<F1>;
        self.rust_closure = &mut visit_object as *mut F1 as *mut libc::c_void;
        let result = f();
        *self = Default::default();
        result
    }

    extern "C" fn c_function_registered<F>(
        rust_closure: *mut libc::c_void,
        worker: *mut libc::c_void,
        object: ObjectReference,
        pin: bool,
    ) -> ObjectReference
    where
        F: FnMut(&'static mut GCWorker<Ruby>, ObjectReference, bool) -> ObjectReference,
    {
        let rust_closure = unsafe { &mut *(rust_closure as *mut F) };
        let worker = unsafe { &mut *(worker as *mut GCWorker<Ruby>) };
        rust_closure(worker, object, pin)
    }

    extern "C" fn c_function_unregistered(
        _rust_closure: *mut libc::c_void,
        worker: *mut libc::c_void,
        object: ObjectReference,
        pin: bool,
    ) -> ObjectReference {
        let worker = unsafe { &mut *(worker as *mut GCWorker<Ruby>) };
        panic!(
            "object_closure is not set.  worker ordinal: {}, object: {}, pin: {}",
            worker.ordinal, object, pin
        );
    }
}

#[repr(C)]
pub struct GCThreadTLS {
    pub kind: libc::c_int,
    pub gc_context: *mut libc::c_void,
    pub object_closure: ObjectClosure,
}

impl GCThreadTLS {
    fn new(kind: libc::c_int, gc_context: *mut libc::c_void) -> Self {
        Self {
            kind,
            gc_context,
            object_closure: Default::default(),
        }
    }

    pub fn for_worker(gc_context: *mut GCWorker<Ruby>) -> Self {
        Self::new(GC_THREAD_KIND_WORKER, gc_context as *mut libc::c_void)
    }

    pub fn from_vwt(vwt: VMWorkerThread) -> *mut GCThreadTLS {
        unsafe { std::mem::transmute(vwt) }
    }

    /// Cast a pointer to `GCThreadTLS` to a ref, with assertion for null pointer.
    ///
    /// # Safety
    ///
    /// Has undefined behavior if `ptr` is invalid.
    pub unsafe fn check_cast(ptr: *mut GCThreadTLS) -> &'static mut GCThreadTLS {
        assert!(!ptr.is_null());
        let result = unsafe { &mut *ptr };
        debug_assert!({
            let kind = result.kind;
            kind == GC_THREAD_KIND_WORKER
        });
        result
    }

    /// Cast a pointer to `VMWorkerThread` to a ref, with assertion for null pointer.
    ///
    /// # Safety
    ///
    /// Has undefined behavior if `ptr` is invalid.
    pub unsafe fn from_vwt_check(vwt: VMWorkerThread) -> &'static mut GCThreadTLS {
        let ptr = Self::from_vwt(vwt);
        unsafe { Self::check_cast(ptr) }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)] // `transmute` does not dereference pointer
    pub fn to_vwt(ptr: *mut Self) -> VMWorkerThread {
        unsafe { std::mem::transmute(ptr) }
    }

    pub fn worker<'w>(&mut self) -> &'w mut GCWorker<Ruby> {
        // NOTE: The returned ref points to the worker which does not have the same lifetime as self.
        assert!(self.kind == GC_THREAD_KIND_WORKER);
        unsafe { &mut *(self.gc_context as *mut GCWorker<Ruby>) }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct RawVecOfObjRef {
    pub ptr: *mut ObjectReference,
    pub len: usize,
    pub capa: usize,
}

impl RawVecOfObjRef {
    pub fn from_vec(vec: Vec<ObjectReference>) -> RawVecOfObjRef {
        // Note: Vec::into_raw_parts is unstable. We implement it manually.
        let mut vec = std::mem::ManuallyDrop::new(vec);
        let (ptr, len, capa) = (vec.as_mut_ptr(), vec.len(), vec.capacity());

        RawVecOfObjRef { ptr, len, capa }
    }

    /// Check invariant conditions for RawVecOfObjRef.
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Validate instance invariants:
    /// - `len <= capa`
    /// - `ptr` must not be null when `len > 0` or `capa > 0`
    /// - `ptr` must be aligned to `align_of::<ObjectReference>()` when non-null
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.len > self.capa {
            return Err("RawVecOfObjRef len exceeds capa");
        }
        if (self.len > 0 || self.capa > 0) && self.ptr.is_null() {
            return Err("RawVecOfObjRef ptr is null with non-zero len or capa");
        }
        if !self.ptr.is_null()
            && (self.ptr as usize) % std::mem::align_of::<ObjectReference>() != 0
        {
            return Err("RawVecOfObjRef ptr is misaligned");
        }
        Ok(())
    }

    /// # Safety
    ///
    /// This function turns raw pointer into a Vec after verifying alignment,
    /// non-null pointer, and length invariants.
    pub unsafe fn into_vec(self) -> Vec<ObjectReference> {
        self.validate()
            .unwrap_or_else(|err| panic!("Invalid RawVecOfObjRef invariant: {err}"));
        if self.capa == 0 {
            return Vec::new();
        }
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.capa) }
    }

    /// Convert into Vec returning Result if invalid.
    ///
    /// # Safety
    ///
    /// This function turns raw pointer into a Vec after verifying alignment,
    /// non-null pointer, and length invariants.
    pub unsafe fn try_into_vec(self) -> Result<Vec<ObjectReference>, &'static str> {
        self.validate()?;
        if self.capa == 0 {
            Ok(Vec::new())
        } else {
            Ok(unsafe { Vec::from_raw_parts(self.ptr, self.len, self.capa) })
        }
    }
}

impl From<Vec<ObjectReference>> for RawVecOfObjRef {
    fn from(v: Vec<ObjectReference>) -> Self {
        Self::from_vec(v)
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct RubyBindingOptions {
    pub suffix_size: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn(gc_worker_tls: *mut GCThreadTLS),
    pub is_mutator: extern "C" fn() -> bool,
    pub stop_the_world: extern "C" fn(),
    pub resume_mutators: extern "C" fn(gc_may_move: bool),
    pub block_for_gc: extern "C" fn(tls: VMMutatorThread),
    pub before_updating_jit_code: extern "C" fn(),
    pub after_updating_jit_code: extern "C" fn(),
    pub number_of_mutators: extern "C" fn() -> usize,
    pub get_mutators: extern "C" fn(
        visit_mutator: extern "C" fn(*mut RubyMutator, *mut libc::c_void),
        data: *mut libc::c_void,
    ),
    pub scan_gc_roots: extern "C" fn(),
    pub scan_objspace: extern "C" fn(),
    pub move_obj_during_marking: extern "C" fn(from: ObjectReference, to: ObjectReference),
    pub update_object_references: extern "C" fn(object: ObjectReference),
    pub call_gc_mark_children: extern "C" fn(object: ObjectReference),
    pub handle_weak_references: extern "C" fn(object: ObjectReference, moving: bool),
    pub call_obj_free: extern "C" fn(object: ObjectReference),
    pub vm_live_bytes: extern "C" fn() -> usize,
    pub update_global_tables: extern "C" fn(tbl_idx: c_int, moving: bool),
    pub global_tables_count: extern "C" fn() -> c_int,
    pub update_finalizer_table: extern "C" fn(),
    pub special_const_p: extern "C" fn(object: ObjectReference) -> bool,
    pub mutator_thread_panic_handler: extern "C" fn(),
    pub gc_thread_panic_handler: extern "C" fn(),
}

impl RubyUpcalls {
    /// Validates that `upcalls` is non-null and that none of its mandatory callback function
    /// pointers are null before dereferencing/cloning from C FFI.
    pub unsafe fn validate_raw(upcalls: *const RubyUpcalls) -> Result<(), &'static str> {
        if upcalls.is_null() {
            return Err("RubyUpcalls pointer is null");
        }
        const MANDATORY_COUNT: usize = 23;
        let fn_ptrs = unsafe {
            std::slice::from_raw_parts(
                upcalls as *const *const libc::c_void,
                MANDATORY_COUNT,
            )
        };
        const NAMES: [&str; MANDATORY_COUNT] = [
            "init_gc_worker_thread",
            "is_mutator",
            "stop_the_world",
            "resume_mutators",
            "block_for_gc",
            "before_updating_jit_code",
            "after_updating_jit_code",
            "number_of_mutators",
            "get_mutators",
            "scan_gc_roots",
            "scan_objspace",
            "move_obj_during_marking",
            "update_object_references",
            "call_gc_mark_children",
            "handle_weak_references",
            "call_obj_free",
            "vm_live_bytes",
            "update_global_tables",
            "global_tables_count",
            "update_finalizer_table",
            "special_const_p",
            "mutator_thread_panic_handler",
            "gc_thread_panic_handler",
        ];
        for (idx, &ptr) in fn_ptrs.iter().enumerate() {
            if ptr.is_null() {
                return Err(NAMES[idx]);
            }
        }
        Ok(())
    }
}

unsafe impl Sync for RubyUpcalls {}

#[repr(C)]
#[derive(Clone)]
pub struct HeapBounds {
    pub start: *mut libc::c_void,
    pub end: *mut libc::c_void,
}

// Static compile-time memory layout, size, and alignment assertions
const _: () = {
    use std::mem::{align_of, offset_of, size_of};

    // 1. RawVecOfObjRef
    assert!(size_of::<RawVecOfObjRef>() == 3 * size_of::<usize>());
    assert!(align_of::<RawVecOfObjRef>() == align_of::<*mut ObjectReference>());
    assert!(offset_of!(RawVecOfObjRef, ptr) == 0);
    assert!(offset_of!(RawVecOfObjRef, len) == size_of::<usize>());
    assert!(offset_of!(RawVecOfObjRef, capa) == 2 * size_of::<usize>());

    // 2. RubyUpcalls
    assert!(size_of::<RubyUpcalls>() == 23 * size_of::<usize>());
    assert!(align_of::<RubyUpcalls>() == align_of::<usize>());
    assert!(offset_of!(RubyUpcalls, init_gc_worker_thread) == 0 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, is_mutator) == 1 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, stop_the_world) == 2 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, resume_mutators) == 3 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, block_for_gc) == 4 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, before_updating_jit_code) == 5 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, after_updating_jit_code) == 6 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, number_of_mutators) == 7 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, get_mutators) == 8 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, scan_gc_roots) == 9 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, scan_objspace) == 10 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, move_obj_during_marking) == 11 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, update_object_references) == 12 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, call_gc_mark_children) == 13 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, handle_weak_references) == 14 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, call_obj_free) == 15 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, vm_live_bytes) == 16 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, update_global_tables) == 17 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, global_tables_count) == 18 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, update_finalizer_table) == 19 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, special_const_p) == 20 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, mutator_thread_panic_handler) == 21 * size_of::<usize>());
    assert!(offset_of!(RubyUpcalls, gc_thread_panic_handler) == 22 * size_of::<usize>());

    // 3. GCThreadTLS
    assert!(size_of::<GCThreadTLS>() == 4 * size_of::<usize>());
    assert!(align_of::<GCThreadTLS>() == align_of::<usize>());
    assert!(offset_of!(GCThreadTLS, kind) == 0);
    assert!(offset_of!(GCThreadTLS, gc_context) == size_of::<usize>());
    assert!(offset_of!(GCThreadTLS, object_closure) == 2 * size_of::<usize>());

    // 4. ObjectClosure
    assert!(size_of::<ObjectClosure>() == 2 * size_of::<usize>());
    assert!(align_of::<ObjectClosure>() == align_of::<usize>());
    assert!(offset_of!(ObjectClosure, c_function) == 0);
    assert!(offset_of!(ObjectClosure, rust_closure) == size_of::<usize>());

    // 5. RubyBindingOptions
    assert!(size_of::<RubyBindingOptions>() == 1 * size_of::<usize>());
    assert!(align_of::<RubyBindingOptions>() == align_of::<usize>());
    assert!(offset_of!(RubyBindingOptions, suffix_size) == 0);
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn test_raw_vec_of_obj_ref_layout() {
        assert_eq!(size_of::<RawVecOfObjRef>(), 3 * size_of::<usize>());
        assert_eq!(align_of::<RawVecOfObjRef>(), align_of::<*mut ObjectReference>());
        assert_eq!(offset_of!(RawVecOfObjRef, ptr), 0);
        assert_eq!(offset_of!(RawVecOfObjRef, len), size_of::<usize>());
        assert_eq!(offset_of!(RawVecOfObjRef, capa), 2 * size_of::<usize>());
    }

    #[test]
    fn test_raw_vec_of_obj_ref_invariants() {
        let valid_empty = RawVecOfObjRef {
            ptr: std::ptr::null_mut(),
            len: 0,
            capa: 0,
        };
        assert!(valid_empty.is_valid());
        assert!(unsafe { valid_empty.into_vec() }.is_empty());

        let mut dummy = [std::ptr::null_mut::<libc::c_void>(); 2];
        let valid_nonempty = RawVecOfObjRef {
            ptr: dummy.as_mut_ptr() as *mut ObjectReference,
            len: 1,
            capa: 2,
        };
        assert!(valid_nonempty.is_valid());

        // Null ptr with len > 0
        let invalid_null = RawVecOfObjRef {
            ptr: std::ptr::null_mut(),
            len: 1,
            capa: 1,
        };
        assert!(!invalid_null.is_valid());

        // len > capa
        let invalid_len = RawVecOfObjRef {
            ptr: dummy.as_mut_ptr() as *mut ObjectReference,
            len: 3,
            capa: 2,
        };
        assert!(!invalid_len.is_valid());

        // Misaligned pointer
        let misaligned_ptr = (dummy.as_mut_ptr() as usize + 1) as *mut ObjectReference;
        let invalid_align = RawVecOfObjRef {
            ptr: misaligned_ptr,
            len: 1,
            capa: 1,
        };
        assert!(!invalid_align.is_valid());
    }

    #[test]
    fn test_ruby_upcalls_layout() {
        assert_eq!(size_of::<RubyUpcalls>(), 23 * size_of::<usize>());
        assert_eq!(align_of::<RubyUpcalls>(), align_of::<usize>());
        assert_eq!(offset_of!(RubyUpcalls, init_gc_worker_thread), 0);
        assert_eq!(offset_of!(RubyUpcalls, gc_thread_panic_handler), 22 * size_of::<usize>());
    }

    #[test]
    fn test_ruby_upcalls_validation() {
        assert!(unsafe { RubyUpcalls::validate_raw(std::ptr::null()) }.is_err());

        let null_upcalls = [std::ptr::null::<libc::c_void>(); 23];
        assert_eq!(
            unsafe { RubyUpcalls::validate_raw(null_upcalls.as_ptr() as *const RubyUpcalls) },
            Err("init_gc_worker_thread")
        );
    }

    #[test]
    fn test_gc_thread_tls_layout() {
        assert_eq!(size_of::<GCThreadTLS>(), 4 * size_of::<usize>());
        assert_eq!(align_of::<GCThreadTLS>(), align_of::<usize>());
        assert_eq!(offset_of!(GCThreadTLS, kind), 0);
        assert_eq!(offset_of!(GCThreadTLS, gc_context), size_of::<usize>());
        assert_eq!(offset_of!(GCThreadTLS, object_closure), 2 * size_of::<usize>());
    }

    #[test]
    fn test_object_closure_layout() {
        assert_eq!(size_of::<ObjectClosure>(), 2 * size_of::<usize>());
        assert_eq!(align_of::<ObjectClosure>(), align_of::<usize>());
        assert_eq!(offset_of!(ObjectClosure, c_function), 0);
        assert_eq!(offset_of!(ObjectClosure, rust_closure), size_of::<usize>());
    }

    #[test]
    fn test_ruby_binding_options_layout() {
        assert_eq!(size_of::<RubyBindingOptions>(), 1 * size_of::<usize>());
        assert_eq!(align_of::<RubyBindingOptions>(), align_of::<usize>());
        assert_eq!(offset_of!(RubyBindingOptions, suffix_size), 0);
    }
}

