use crate::abi::RawVecOfObjRef;
use crate::abi::RubyBindingOptions;
use crate::abi::RubyUpcalls;
use crate::safe_api;
use mmtk::AllocationSemantics;
use mmtk::MMTKBuilder;
use mmtk::Mutator;
use mmtk::util::Address;
use mmtk::util::ObjectReference;
use mmtk::util::VMMutatorThread;
use mmtk::util::VMThread;
use mmtk::util::alloc::BumpPointer;

pub type RubyMutator = Mutator<crate::Ruby>;

/// Checks if the given object reference is live in MMTk.
///
/// # Safety
/// The caller must guarantee that `object` is a valid, non-null, properly aligned `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_is_live_object(object: ObjectReference) -> bool {
    safe_api::is_live_object(object)
}

/// Checks if the given object reference is reachable.
///
/// # Safety
/// The caller must guarantee that `object` is a valid, non-null, properly aligned `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_is_reachable(object: ObjectReference) -> bool {
    safe_api::is_reachable(object)
}

// =============== Bootup ===============

/// Creates a default `MMTKBuilder` instance and returns a raw pointer.
///
/// # Safety
/// The returned pointer is owned by the caller and must be freed by passing it to `mmtk_init_binding`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_builder_default() -> *mut MMTKBuilder {
    safe_api::builder_default()
}

/// Initializes the MMTk binding with the provided builder, options, and upcalls.
///
/// # Safety
/// - `builder` must be a valid raw pointer created by `mmtk_builder_default`.
/// - `binding_options` must point to a valid, initialized `RubyBindingOptions`.
/// - `upcalls` must point to a valid, initialized `RubyUpcalls` struct whose function pointers are non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_init_binding(
    builder: *mut MMTKBuilder,
    binding_options: *const RubyBindingOptions,
    upcalls: *const RubyUpcalls,
) {
    unsafe { safe_api::init_binding(builder, binding_options, upcalls) }
}

/// Returns the VO bit log region size.
///
/// # Safety
/// This function is safe to call from C as it only reads global static configuration.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_get_vo_bit_log_region_size() -> usize {
    safe_api::get_vo_bit_log_region_size()
}

/// Returns the base address of the VO bit side metadata.
///
/// # Safety
/// This function is safe to call from C as it returns static metadata address information.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_get_vo_bit_base_addr() -> usize {
    safe_api::get_vo_bit_base_addr()
}

/// Initializes garbage collection for the calling thread.
///
/// # Safety
/// `tls` must represent a valid VM thread context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_initialize_collection(tls: VMThread) {
    safe_api::initialize_collection(tls)
}

/// Binds a mutator thread to MMTk.
///
/// # Safety
/// `tls` must be a valid, non-null mutator thread pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_bind_mutator(tls: VMMutatorThread) -> *mut RubyMutator {
    safe_api::bind_mutator(tls)
}

/// Gets the bump pointer allocator for a mutator if using Immix plan.
///
/// # Safety
/// `m` must point to a valid, registered `RubyMutator`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_get_bump_pointer_allocator(m: *mut RubyMutator) -> *mut BumpPointer {
    unsafe { safe_api::get_bump_pointer_allocator(m) }
}

/// Destroys a mutator and reclaims its resources.
///
/// # Safety
/// `mutator` must point to a valid, previously allocated `RubyMutator` that is no longer in use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_destroy_mutator(mutator: *mut RubyMutator) {
    unsafe { safe_api::destroy_mutator(mutator) }
}

// =============== GC ===============

/// Triggers a garbage collection cycle requested by user code or GC.stress.
///
/// # Safety
/// `tls` must represent a valid VMMutatorThread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_handle_user_collection_request(
    tls: VMMutatorThread,
    force: bool,
    exhaustive: bool,
) {
    safe_api::handle_user_collection_request(tls, force, exhaustive)
}

/// Enables or disables garbage collection.
///
/// # Safety
/// MMTk binding must be initialized before calling this function.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_set_gc_enabled(enable: bool) {
    safe_api::set_gc_enabled(enable)
}

/// Returns whether garbage collection is currently enabled.
///
/// # Safety
/// MMTk binding must be initialized before calling this function.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_gc_enabled_p() -> bool {
    safe_api::gc_enabled_p()
}

// =============== Object allocation ===============

/// Returns the maximum allocation size for default non-LOS objects.
///
/// # Safety
/// MMTk binding must be initialized before calling this function.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_max_non_los_default_alloc_bytes() -> usize {
    safe_api::max_non_los_default_alloc_bytes()
}

/// Allocates memory for a Ruby object via MMTk.
///
/// # Safety
/// `mutator` must point to a valid, bound `RubyMutator`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_alloc(
    mutator: *mut RubyMutator,
    size: usize,
    align: usize,
    offset: usize,
    semantics: AllocationSemantics,
) -> Address {
    unsafe { safe_api::alloc(mutator, size, align, offset, semantics) }
}

/// Performs post-allocation initialization for an allocated object.
///
/// # Safety
/// - `mutator` must point to a valid `RubyMutator`.
/// - `refer` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_post_alloc(
    mutator: *mut RubyMutator,
    refer: ObjectReference,
    bytes: usize,
    semantics: AllocationSemantics,
) {
    unsafe { safe_api::post_alloc(mutator, refer, bytes, semantics) }
}

/// Registers candidate objects to be freed during GC.
///
/// # Safety
/// If `count > 0`, `objects` must point to a valid, contiguous array of `count` `ObjectReference` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_add_obj_free_candidates(
    objects: *const ObjectReference,
    count: usize,
    can_parallel_free: bool,
) {
    unsafe { safe_api::add_obj_free_candidates(objects, count, can_parallel_free) }
}

// =============== Weak references ===============

/// Registers an object as containing weak references.
///
/// # Safety
/// `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_declare_weak_references(object: ObjectReference) {
    safe_api::declare_weak_references(object)
}

/// Returns whether a weak reference object is still alive.
///
/// # Safety
/// `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_weak_references_alive_p(object: ObjectReference) -> bool {
    safe_api::weak_references_alive_p(object)
}

/// Returns the number of registered weak references.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_weak_references_count() -> usize {
    safe_api::weak_references_count()
}

// =============== Compaction ===============

/// Registers an object that pins its children from moving during GC.
///
/// # Safety
/// `obj` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_register_pinning_obj(obj: ObjectReference) {
    safe_api::register_pinning_obj(obj)
}

/// Returns whether the specified object is currently pinned.
///
/// # Safety
/// `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_is_pinned(object: ObjectReference) -> bool {
    safe_api::is_pinned(object)
}

// =============== Write barriers ===============

/// Executes write barrier post-store callback.
///
/// # Safety
/// - `mutator` must point to a valid `RubyMutator`.
/// - `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_object_reference_write_post(
    mutator: *mut RubyMutator,
    object: ObjectReference,
) {
    unsafe { safe_api::object_reference_write_post(mutator, object) }
}

/// Registers an object unprotected by write barriers.
///
/// # Safety
/// `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_register_wb_unprotected_object(object: ObjectReference) {
    safe_api::register_wb_unprotected_object(object)
}

/// Checks whether an object is registered as WB-unprotected.
///
/// # Safety
/// `object` must be a valid, non-null `ObjectReference`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_object_wb_unprotected_p(object: ObjectReference) -> bool {
    safe_api::object_wb_unprotected_p(object)
}

// =============== Heap walking ===============

/// Enumerates all live objects in the MMTk heap.
///
/// # Safety
/// `callback` must be a valid function pointer capable of being invoked for each object reference.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_enumerate_objects(
    callback: extern "C" fn(ObjectReference, *mut libc::c_void),
    data: *mut libc::c_void,
) {
    safe_api::enumerate_objects(callback, data)
}

// =============== Finalizers ===============

/// Retrieves all object free candidate references.
///
/// # Safety
/// The caller receives ownership of the raw vector buffer and must pass it to `mmtk_free_raw_vec_of_obj_ref`.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_get_all_obj_free_candidates() -> RawVecOfObjRef {
    safe_api::get_all_obj_free_candidates()
}

/// Frees a raw vector of object references returned by `mmtk_get_all_obj_free_candidates`.
///
/// # Safety
/// `raw_vec` must contain a valid raw vector buffer created by MMTk.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_free_raw_vec_of_obj_ref(raw_vec: RawVecOfObjRef) {
    safe_api::free_raw_vec_of_obj_ref(raw_vec)
}

// =============== Forking ===============

/// Prepares MMTk before process forking.
///
/// # Safety
/// Must be called on the main thread before calling `fork()`.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_before_fork() {
    safe_api::before_fork()
}

/// Re-initializes MMTk state after process forking.
///
/// # Safety
/// `tls` must represent a valid VMThread in the child process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_after_fork(tls: VMThread) {
    safe_api::after_fork(tls)
}

// =============== Statistics ===============

/// Returns total heap memory size in bytes.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_total_bytes() -> usize {
    safe_api::total_bytes()
}

/// Returns currently used heap memory in bytes.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_used_bytes() -> usize {
    safe_api::used_bytes()
}

/// Returns free heap memory available in bytes.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_free_bytes() -> usize {
    safe_api::free_bytes()
}

/// Returns the starting address of the heap.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_starting_heap_address() -> Address {
    safe_api::starting_heap_address()
}

/// Returns the end address of the heap.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_last_heap_address() -> Address {
    safe_api::last_heap_address()
}

/// Returns the number of MMTk worker threads.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_worker_count() -> usize {
    safe_api::worker_count()
}

/// Returns a C string pointer describing the active MMTk plan.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_plan() -> *const u8 {
    safe_api::plan()
}

/// Returns a C string pointer describing the heap trigger mode.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_heap_mode() -> *const u8 {
    safe_api::heap_mode()
}

/// Returns minimum heap size in bytes.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_heap_min() -> usize {
    safe_api::heap_min()
}

/// Returns maximum heap size in bytes.
///
/// # Safety
/// MMTk binding must be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mmtk_heap_max() -> usize {
    safe_api::heap_max()
}

// =============== Miscellaneous ===============

/// Checks whether `addr` points to an MMTk managed object.
///
/// # Safety
/// `addr` must be a valid, non-zero Address aligned to `VO_BIT_REGION_SIZE`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmtk_is_mmtk_object(addr: Address) -> bool {
    safe_api::is_mmtk_object(addr)
}
