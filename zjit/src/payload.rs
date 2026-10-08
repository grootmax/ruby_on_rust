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

/// We use a raw pointer instead of Rc to save space for refcount
pub type IseqVersionRef = NonNull<IseqVersion>;

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
        NonNull::new(version_ptr).expect("no null from Box")
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

/// Lifetime-bounded encapsulation of an `IseqPayload` raw pointer.
///
/// Ensures VM lock acquisition checks and encapsulates raw FFI pointer conversions,
/// lookup, creation, and deallocation logic.
#[derive(Debug)]
pub struct IseqPayloadHandle<'a> {
    ptr: NonNull<IseqPayload>,
    _marker: std::marker::PhantomData<&'a mut IseqPayload>,
}

impl<'a> IseqPayloadHandle<'a> {
    /// Wrap a raw C void pointer from GC callbacks into a lifetime-bounded payload handle.
    /// Performs null validation and asserts VM lock acquisition before returning.
    pub fn from_raw_void(ptr: *mut c_void) -> Option<Self> {
        let non_null = NonNull::new(ptr as *mut IseqPayload)?;
        unsafe {
            rb_assert_holding_vm_lock();
        }
        Some(Self {
            ptr: non_null,
            _marker: std::marker::PhantomData,
        })
    }

    /// Get the payload handle for an ISEQ, creating a new payload if none exists.
    pub fn get_or_create(iseq: IseqPtr) -> Self {
        type VoidPtr = *mut c_void;
        unsafe {
            let payload = rb_iseq_get_jit_payload(iseq);
            let ptr = if payload.is_null() {
                let new_payload = Box::into_raw(Box::new(IseqPayload::new()));
                rb_iseq_set_jit_payload(iseq, new_payload as VoidPtr);
                NonNull::new(new_payload).expect("non-null from Box")
            } else {
                NonNull::new(payload as *mut IseqPayload).expect("non-null payload")
            };
            Self {
                ptr,
                _marker: std::marker::PhantomData,
            }
        }
    }

    /// Get the payload handle for an ISEQ if it exists, or None if not allocated.
    pub fn get(iseq: IseqPtr) -> Option<Self> {
        unsafe {
            let payload = rb_iseq_get_jit_payload(iseq);
            let ptr = NonNull::new(payload as *mut IseqPayload)?;
            Some(Self {
                ptr,
                _marker: std::marker::PhantomData,
            })
        }
    }

    /// Free the payload associated with an ISEQ and nullify version references.
    pub fn free_for_iseq(iseq: IseqPtr) {
        if let Some(handle) = Self::get(iseq) {
            unsafe {
                rb_iseq_clear_jit_payload(iseq);
                for &version in handle.versions.iter() {
                    (*version.as_ptr()).iseq = std::ptr::null();
                }
                let payload = Box::from_raw(handle.ptr.as_ptr());
                drop(payload);
            }
        }
    }
}

impl<'a> std::ops::Deref for IseqPayloadHandle<'a> {
    type Target = IseqPayload;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a> std::ops::DerefMut for IseqPayloadHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

/// Get the payload handle associated with an ISEQ. Create one if none exists.
pub fn get_or_create_iseq_payload<'a>(iseq: IseqPtr) -> IseqPayloadHandle<'a> {
    IseqPayloadHandle::get_or_create(iseq)
}
