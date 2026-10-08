use std::ffi::c_void;
use std::ptr::NonNull;
use crate::codegen::IseqCallRef;
use crate::options::{get_option, NumExits};
use crate::stats::CompileError;
use crate::{cruby::*, profile::IseqProfile, virtualmem::CodePtr};

pub use crate::jit_frame::JITFrame;

/// This is all the data ZJIT stores on an ISEQ. We mark objects in this struct on GC.
#[derive(Debug)]
pub struct IseqPayload {
    /// Type information of YARV instruction operands
    pub profile: IseqProfile,
    /// JIT code versions. Different versions should have different assumptions.
    pub versions: Vec<IseqVersionRef>,
    /// Whether a previous compilation of this ISEQ was invalidated due to
    /// singleton class creation (violation of [`crate::hir::Invariant::NoSingletonClass`]).
    pub was_invalidated_for_singleton_class_creation: bool,
    /// Whether `self` is guaranteed to be a heap (non-immediate) object for this
    /// ISEQ. Set at compile triggers (entry point / function stub hit) where the
    /// owning class is known via the method entry, and consumed in `iseq_to_hir`
    /// to type the `self`-producing instructions (`LoadSelf` / `SelfParam`
    /// `LoadArg`) as `HeapBasicObject`. Defaults to `false` (the conservative
    /// `BasicObject`) when the owner is unknown.
    /// See [`crate::cruby::iseq_self_is_heap_object`].
    pub self_is_heap_object: bool,
    /// Number of recompile exits before invalidating the current version. See `exit_recompile`.
    pub num_exits_until_invalidate: NumExits,
}

impl IseqPayload {
    fn new() -> Self {
        Self {
            profile: IseqProfile::new(),
            versions: vec![],
            was_invalidated_for_singleton_class_creation: false,
            self_is_heap_object: false,
            num_exits_until_invalidate: get_option!(num_exits_until_invalidate),
        }
    }
}

/// JIT code version. When the same ISEQ is compiled with a different assumption, a new version is created.
#[derive(Debug)]
pub struct IseqVersion {
    /// ISEQ pointer. Stored here to minimize the size of PatchPoint.
    pub iseq: IseqPtr,

    /// Compilation status of the ISEQ. It has the JIT code address of the first block if Compiled.
    pub status: IseqStatus,

    /// GC offsets of the JIT code. These are the addresses of objects that need to be marked.
    pub gc_offsets: Vec<CodePtr>,

    /// JIT-to-JIT calls from the ISEQ. The IseqPayload's ISEQ is the caller of it.
    pub outgoing: Vec<IseqCallRef>,

    /// JIT-to-JIT calls to the ISEQ. The IseqPayload's ISEQ is the callee of it.
    pub incoming: Vec<IseqCallRef>,
}

/// We use a raw pointer wrapper instead of Rc to save space for refcount.
/// Safe handle wrapping a non-null pointer to an IseqVersion, encapsulating unsafe
/// raw pointer dereferences behind audited boundary methods.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IseqVersionRef(NonNull<IseqVersion>);

impl IseqVersionRef {
    /// Create an `IseqVersionRef` from a `NonNull<IseqVersion>`.
    pub fn from_non_null(ptr: NonNull<IseqVersion>) -> Self {
        Self(ptr)
    }

    /// Return the underlying raw pointer.
    pub fn as_ptr(&self) -> *mut IseqVersion {
        self.0.as_ptr()
    }

    /// Returns a reference to the underlying `IseqVersion`.
    /// SAFETY: The caller must ensure the underlying `IseqVersion` pointer remains valid.
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a IseqVersion {
        unsafe { self.0.as_ref() }
    }

    /// Returns a mutable reference to the underlying `IseqVersion`.
    /// SAFETY: The caller must ensure exclusive mutable access to the underlying `IseqVersion`.
    #[inline]
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut IseqVersion {
        unsafe { self.0.as_mut() }
    }

    /// Get the associated ISEQ pointer.
    pub fn iseq(&self) -> IseqPtr {
        unsafe { self.0.as_ref().iseq }
    }

    /// Set the associated ISEQ pointer.
    pub fn set_iseq(&self, iseq: IseqPtr) {
        unsafe { (*self.0.as_ptr()).iseq = iseq; }
    }

    /// Clear the associated ISEQ pointer to null.
    pub fn clear_iseq(&self) {
        unsafe { (*self.0.as_ptr()).iseq = std::ptr::null(); }
    }

    /// Get the compilation status.
    pub fn status(&self) -> &IseqStatus {
        unsafe { &self.0.as_ref().status }
    }

    /// Set the compilation status.
    pub fn set_status(&self, status: IseqStatus) {
        unsafe { (*self.0.as_ptr()).status = status; }
    }

    /// Check if this version was invalidated.
    pub fn is_invalidated(&self) -> bool {
        unsafe { self.0.as_ref().status == IseqStatus::Invalidated }
    }

    /// Mark this version as invalidated.
    pub fn set_invalidated(&self) {
        self.set_status(IseqStatus::Invalidated);
    }

    /// Get a slice of GC offsets.
    pub fn gc_offsets(&self) -> &[CodePtr] {
        unsafe { &self.0.as_ref().gc_offsets }
    }

    /// Extend GC offsets with new code pointers.
    pub fn extend_gc_offsets(&self, offsets: &[CodePtr]) {
        unsafe { (*self.0.as_ptr()).gc_offsets.extend_from_slice(offsets); }
    }

    /// Retain GC offsets matching the predicate `f`.
    pub fn retain_gc_offsets<F>(&self, f: F)
    where
        F: FnMut(&CodePtr) -> bool,
    {
        unsafe { (*self.0.as_ptr()).gc_offsets.retain(f); }
    }

    /// Get incoming JIT-to-JIT calls.
    pub fn incoming(&self) -> &[IseqCallRef] {
        unsafe { &self.0.as_ref().incoming }
    }

    /// Push an incoming JIT-to-JIT call.
    pub fn push_incoming(&self, iseq_call: IseqCallRef) {
        unsafe { (*self.0.as_ptr()).incoming.push(iseq_call); }
    }

    /// Get outgoing JIT-to-JIT calls.
    pub fn outgoing(&self) -> &[IseqCallRef] {
        unsafe { &self.0.as_ref().outgoing }
    }

    /// Extend outgoing JIT-to-JIT calls.
    pub fn extend_outgoing(&self, iseq_calls: Vec<IseqCallRef>) {
        unsafe { (*self.0.as_ptr()).outgoing.extend(iseq_calls); }
    }

    /// Update ISEQ and call references for GC compaction.
    pub fn update_gc_references(&self, gc_location: unsafe fn(VALUE) -> VALUE) {
        unsafe {
            let version = &mut *self.0.as_ptr();
            version.iseq = gc_location(version.iseq.into()).as_iseq();

            for iseq_call in version.incoming.iter_mut() {
                let old_iseq = iseq_call.iseq.get();
                let new_iseq = gc_location(VALUE(old_iseq as usize)).0 as IseqPtr;
                if old_iseq != new_iseq {
                    iseq_call.iseq.set(new_iseq);
                }
            }

            for iseq_call in version.outgoing.iter_mut() {
                let old_iseq = iseq_call.iseq.get();
                let new_iseq = gc_location(VALUE(old_iseq as usize)).0 as IseqPtr;
                if old_iseq != new_iseq {
                    iseq_call.iseq.set(new_iseq);
                }
            }
        }
    }
}

impl IseqVersion {
    /// Check if this version was invalidated
    pub fn is_invalidated(&self) -> bool {
        self.status == IseqStatus::Invalidated
    }

    /// Allocate a new IseqVersion to be compiled
    pub fn new(iseq: IseqPtr) -> IseqVersionRef {
        let version = Self {
            iseq,
            status: IseqStatus::NotCompiled,
            gc_offsets: vec![],
            outgoing: vec![],
            incoming: vec![],
        };
        let version_ptr = Box::into_raw(Box::new(version));
        let non_null = NonNull::new(version_ptr).expect("no null from Box");
        IseqVersionRef::from_non_null(non_null)
    }
}

/// Set of CodePtrs for an ISEQ
#[derive(Clone, Debug, PartialEq)]
pub struct IseqCodePtrs {
    /// Entry for the interpreter
    pub start_ptr: CodePtr,
    /// Entries for JIT-to-JIT calls
    pub jit_entry_ptrs: Vec<CodePtr>,
}

#[derive(Debug, PartialEq)]
pub enum IseqStatus {
    Compiled(IseqCodePtrs),
    CantCompile(CompileError),
    NotCompiled,
    Invalidated,
}

/// Get a pointer to the payload object associated with an ISEQ. Create one if none exists.
pub fn get_or_create_iseq_payload_ptr(iseq: IseqPtr) -> *mut IseqPayload {
    type VoidPtr = *mut c_void;

    unsafe {
        let payload = rb_iseq_get_jit_payload(iseq);
        if payload.is_null() {
            // Allocate a new payload with Box and transfer ownership to the GC.
            // We drop the payload with Box::from_raw when the GC frees the ISEQ and calls us.
            // NOTE(alan): Sometimes we read from an ISEQ without ever writing to it.
            // We allocate in those cases anyways.
            let new_payload = IseqPayload::new();
            let new_payload = Box::into_raw(Box::new(new_payload));
            rb_iseq_set_jit_payload(iseq, new_payload as VoidPtr);

            new_payload
        } else {
            payload as *mut IseqPayload
        }
    }
}

/// Get a pointer to the payload object associated with an ISEQ, or null if never allocated.
pub fn get_iseq_payload_ptr(iseq: IseqPtr) -> *mut IseqPayload {
    unsafe { rb_iseq_get_jit_payload(iseq) as *mut IseqPayload }
}

/// Get the payload object associated with an ISEQ. Create one if none exists.
pub fn get_or_create_iseq_payload(iseq: IseqPtr) -> &'static mut IseqPayload {
    let payload_non_null = get_or_create_iseq_payload_ptr(iseq);
    payload_ptr_as_mut(payload_non_null)
}

/// Convert an IseqPayload pointer to a mutable reference. Only one reference
/// should be kept at a time.
pub fn payload_ptr_as_mut(payload_ptr: *mut IseqPayload) -> &'static mut IseqPayload {
    // SAFETY: we should have the VM lock and all other Ruby threads should be asleep. So we have
    // exclusive mutable access.
    // Hmm, nothing seems to stop calling this on the same
    // iseq twice, though, which violates aliasing rules.
    unsafe { payload_ptr.as_mut() }.unwrap()
}
