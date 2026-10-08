use mmtk::util::alloc::BumpPointer;
use mmtk::util::alloc::ImmixAllocator;
use mmtk::util::conversions;
use mmtk::util::options::PlanSelector;
use std::str::FromStr;

use crate::Ruby;
use crate::RubySlot;
use crate::abi::MIN_OBJ_ALIGN;
use crate::abi::RawVecOfObjRef;
use crate::abi::RubyBindingOptions;
use crate::abi::RubyUpcalls;
use crate::api::RubyMutator;
use crate::binding;
use crate::binding::RubyBinding;
use crate::heap::CPU_HEAP_TRIGGER_CONFIG;
use crate::heap::CpuHeapTriggerConfig;
use crate::heap::RUBY_HEAP_TRIGGER_CONFIG;
use crate::heap::RubyHeapTriggerConfig;
use crate::mmtk;
use crate::utils::default_heap_max;
use crate::utils::parse_capacity;
use mmtk::AllocationSemantics;
use mmtk::MMTKBuilder;
use mmtk::memory_manager;
use mmtk::memory_manager::mmtk_init;
use mmtk::util::Address;
use mmtk::util::ObjectReference;
use mmtk::util::VMMutatorThread;
use mmtk::util::VMThread;
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::options::GCTriggerSelector;

// =============== Precondition Assertions ===============

/// Asserts that an `ObjectReference` is non-null and aligned to `MIN_OBJ_ALIGN`.
#[inline(always)]
pub fn assert_object_reference(obj: ObjectReference) {
    assert!(
        !obj.to_raw_address().is_zero(),
        "ObjectReference must not be null"
    );
    assert!(
        obj.to_raw_address().is_aligned_to(MIN_OBJ_ALIGN),
        "ObjectReference address {:?} is not aligned to MIN_OBJ_ALIGN ({MIN_OBJ_ALIGN})",
        obj.to_raw_address()
    );
}

/// Asserts that an `Address` is non-zero (non-null).
#[inline(always)]
pub fn assert_address(addr: Address) {
    assert!(!addr.is_zero(), "Address must not be zero");
}

/// Asserts that an `Address` is non-zero and aligned to `align`.
#[inline(always)]
pub fn assert_address_aligned(addr: Address, align: usize) {
    assert_address(addr);
    if align > 0 {
        assert!(
            addr.is_aligned_to(align),
            "Address {:?} is not aligned to {align}",
            addr
        );
    }
}

/// Asserts that a pointer is non-null and returns a mutable reference to `T`.
///
/// # Safety
/// Caller must ensure `ptr` points to a valid, properly aligned instance of `T`.
#[inline(always)]
pub unsafe fn assert_non_null_mut<'a, T>(ptr: *mut T, name: &str) -> &'a mut T {
    assert!(!ptr.is_null(), "{name} pointer must not be null");
    unsafe { &mut *ptr }
}

/// Asserts that a pointer is non-null and returns a shared reference to `T`.
///
/// # Safety
/// Caller must ensure `ptr` points to a valid, properly aligned instance of `T`.
#[inline(always)]
pub unsafe fn assert_non_null_ref<'a, T>(ptr: *const T, name: &str) -> &'a T {
    assert!(!ptr.is_null(), "{name} pointer must not be null");
    unsafe { &*ptr }
}

// =============== Liveness & Queries ===============

#[inline(always)]
pub fn is_live_object(object: ObjectReference) -> bool {
    assert_object_reference(object);
    memory_manager::is_live_object(object)
}

#[inline(always)]
pub fn is_reachable(object: ObjectReference) -> bool {
    assert_object_reference(object);
    object.is_reachable()
}

// =============== Bootup ===============

fn parse_env_var_with<T, F: FnOnce(&str) -> Option<T>>(key: &str, parse: F) -> Option<T> {
    let val = match std::env::var(key) {
        Ok(val) => val,
        Err(std::env::VarError::NotPresent) => return None,
        Err(std::env::VarError::NotUnicode(os_string)) => {
            eprintln!("[FATAL] Invalid {key} {os_string:?}");
            std::process::exit(1);
        }
    };

    let parsed = parse(&val).unwrap_or_else(|| {
        eprintln!("[FATAL] Invalid {key} {val}");
        std::process::exit(1);
    });

    Some(parsed)
}

fn parse_env_var<T: FromStr>(key: &str) -> Option<T> {
    parse_env_var_with(key, |s| s.parse().ok())
}

fn mmtk_builder_default_parse_threads() -> Option<usize> {
    parse_env_var("MMTK_THREADS")
}

fn mmtk_builder_default_parse_heap_min() -> usize {
    const DEFAULT_HEAP_MIN: usize = 1 << 20;
    parse_env_var_with("MMTK_HEAP_MIN", parse_capacity).unwrap_or(DEFAULT_HEAP_MIN)
}

fn mmtk_builder_default_parse_heap_max() -> usize {
    parse_env_var_with("MMTK_HEAP_MAX", parse_capacity).unwrap_or_else(default_heap_max)
}

fn parse_float_env_var(key: &str, default: f64, min: f64, max: f64) -> f64 {
    parse_env_var_with(key, |s| {
        let mut float = f64::from_str(s).unwrap_or(default);

        if float <= min {
            eprintln!(
                "{key} has value {float} which must be greater than {min}, using default instead"
            );
            float = default;
        }

        if float >= max {
            eprintln!(
                "{key} has value {float} which must be less than {max}, using default instead"
            );
            float = default;
        }

        Some(float)
    })
    .unwrap_or(default)
}

fn mmtk_builder_default_parse_heap_mode(heap_min: usize, heap_max: usize) -> GCTriggerSelector {
    let make_fixed = || GCTriggerSelector::FixedHeapSize(heap_max);
    let make_dynamic = || GCTriggerSelector::DynamicHeapSize(heap_min, heap_max);

    parse_env_var_with("MMTK_HEAP_MODE", |s| match s {
        "fixed" => Some(make_fixed()),
        "dynamic" => Some(make_dynamic()),
        "ruby" => {
            let min_ratio = parse_float_env_var("RUBY_GC_HEAP_FREE_SLOTS_MIN_RATIO", 0.2, 0.0, 1.0);
            let goal_ratio =
                parse_float_env_var("RUBY_GC_HEAP_FREE_SLOTS_GOAL_RATIO", 0.4, min_ratio, 1.0);
            let max_ratio =
                parse_float_env_var("RUBY_GC_HEAP_FREE_SLOTS_MAX_RATIO", 0.65, goal_ratio, 1.0);

            crate::heap::RUBY_HEAP_TRIGGER_CONFIG
                .set(RubyHeapTriggerConfig {
                    min_heap_pages: conversions::bytes_to_pages_up(heap_min),
                    max_heap_pages: conversions::bytes_to_pages_up(heap_max),
                    heap_pages_min_ratio: min_ratio,
                    heap_pages_goal_ratio: goal_ratio,
                    heap_pages_max_ratio: max_ratio,
                })
                .unwrap_or_else(|_| panic!("RUBY_HEAP_TRIGGER_CONFIG is already set"));

            Some(GCTriggerSelector::Delegated)
        }
        "cpu" => {
            let target_percent = parse_float_env_var("MMTK_GC_CPU_TARGET", 5.0, 0.0, 100.0);
            let window_size = parse_env_var::<usize>("MMTK_GC_CPU_WINDOW").unwrap_or(3);
            let window_size = window_size.max(1);

            let min_heap_pages = conversions::bytes_to_pages_up(heap_min);
            let max_heap_pages = conversions::bytes_to_pages_up(heap_max);
            let initial_heap_pages = min_heap_pages;

            CPU_HEAP_TRIGGER_CONFIG
                .set(CpuHeapTriggerConfig {
                    min_heap_pages,
                    max_heap_pages,
                    initial_heap_pages,
                    target_gc_cpu: target_percent / 100.0,
                    window_size,
                })
                .unwrap_or_else(|_| panic!("CPU_HEAP_TRIGGER_CONFIG is already set"));

            Some(GCTriggerSelector::Delegated)
        }
        _ => None,
    })
    .unwrap_or_else(make_dynamic)
}

fn mmtk_builder_default_parse_plan() -> PlanSelector {
    parse_env_var_with("MMTK_PLAN", |s| match s {
        "NoGC" => Some(PlanSelector::NoGC),
        "MarkSweep" => Some(PlanSelector::MarkSweep),
        "Immix" => Some(PlanSelector::Immix),
        "StickyImmix" => Some(PlanSelector::StickyImmix),
        _ => None,
    })
    .unwrap_or(PlanSelector::StickyImmix)
}

#[inline]
pub fn builder_default() -> *mut MMTKBuilder {
    let mut builder = MMTKBuilder::new_no_env_vars();
    builder.options.no_finalizer.set(true);

    if let Some(threads) = mmtk_builder_default_parse_threads()
        && !builder.options.threads.set(threads)
    {
        eprintln!("[FATAL] Failed to set the number of MMTk threads to {threads}");
        std::process::exit(1);
    }

    let heap_min = mmtk_builder_default_parse_heap_min();
    let heap_max = mmtk_builder_default_parse_heap_max();

    if heap_min >= heap_max {
        eprintln!("[FATAL] MMTK_HEAP_MIN({heap_min}) >= MMTK_HEAP_MAX({heap_max})");
        std::process::exit(1);
    }

    builder
        .options
        .gc_trigger
        .set(mmtk_builder_default_parse_heap_mode(heap_min, heap_max));

    builder.options.plan.set(mmtk_builder_default_parse_plan());

    Box::into_raw(Box::new(builder))
}

/// # Safety
/// Caller must pass valid, properly allocated pointers.
#[inline]
pub unsafe fn init_binding(
    builder: *mut MMTKBuilder,
    binding_options: *const RubyBindingOptions,
    upcalls: *const RubyUpcalls,
) {
    let upcalls_ref = unsafe { assert_non_null_ref(upcalls, "upcalls") };
    let binding_options_ref = unsafe { assert_non_null_ref(binding_options, "binding_options") };

    crate::MUTATOR_THREAD_PANIC_HANDLER
        .set(upcalls_ref.mutator_thread_panic_handler)
        .unwrap_or_else(|_| panic!("MUTATOR_THREAD_PANIC_HANDLER is already initialized"));

    crate::set_panic_hook();

    assert!(!builder.is_null(), "builder pointer must not be null");
    let builder: Box<MMTKBuilder> = unsafe { Box::from_raw(builder) };
    let binding_options = binding_options_ref.clone();
    let mmtk_boxed = mmtk_init(&builder);
    let mmtk_static = Box::leak(Box::new(mmtk_boxed));

    let mut binding = RubyBinding::new(mmtk_static, &binding_options, upcalls);
    binding
        .weak_proc
        .init_parallel_obj_free_candidates(memory_manager::num_of_workers(binding.mmtk));

    crate::BINDING
        .set(binding)
        .unwrap_or_else(|_| panic!("Binding is already initialized"));
}

#[inline(always)]
pub fn get_vo_bit_log_region_size() -> usize {
    mmtk::util::is_mmtk_object::VO_BIT_REGION_SIZE.trailing_zeros() as usize
}

#[inline(always)]
pub fn get_vo_bit_base_addr() -> usize {
    mmtk::util::metadata::side_metadata::vo_bit_side_metadata_addr().as_usize()
}

#[inline(always)]
pub fn initialize_collection(tls: VMThread) {
    memory_manager::initialize_collection(mmtk(), tls)
}

#[inline(always)]
pub fn bind_mutator(tls: VMMutatorThread) -> *mut RubyMutator {
    Box::into_raw(memory_manager::bind_mutator(mmtk(), tls))
}

/// # Safety
/// `m` must point to a valid `RubyMutator`.
#[inline(always)]
pub unsafe fn get_bump_pointer_allocator(m: *mut RubyMutator) -> *mut BumpPointer {
    let mutator = unsafe { assert_non_null_mut(m, "RubyMutator") };
    match *crate::BINDING.get().unwrap().mmtk.get_options().plan {
        PlanSelector::Immix | PlanSelector::StickyImmix => {
            let allocator =
                unsafe { mutator.allocator_mut(mmtk::util::alloc::AllocatorSelector::Immix(0)) };

            if let Some(immix_allocator) = allocator.downcast_mut::<ImmixAllocator<Ruby>>() {
                &mut immix_allocator.bump_pointer as *mut BumpPointer
            } else {
                panic!("Failed to get bump pointer allocator");
            }
        }
        _ => std::ptr::null_mut(),
    }
}

/// # Safety
/// `mutator` must point to a valid, heap-allocated `RubyMutator`.
#[inline(always)]
pub unsafe fn destroy_mutator(mutator: *mut RubyMutator) {
    let mutator_ref = unsafe { assert_non_null_mut(mutator, "RubyMutator") };
    memory_manager::destroy_mutator(mutator_ref);
    let _ = unsafe { Box::from_raw(mutator) };
}

// =============== GC ===============

#[inline(always)]
pub fn handle_user_collection_request(tls: VMMutatorThread, force: bool, exhaustive: bool) {
    let gc_was_disabled = force && !crate::mmtk().is_collection_enabled();
    if gc_was_disabled {
        crate::mmtk().enable_collection();
    }

    crate::mmtk().handle_user_collection_request(tls, force, exhaustive);

    if gc_was_disabled {
        crate::mmtk()
            .disable_collection()
            .unwrap_or_else(|_| panic!("failed to re-disable GC after GC"));
    }
}

#[inline(always)]
pub fn set_gc_enabled(enable: bool) {
    if enable {
        crate::mmtk().enable_collection();
    } else if crate::mmtk().is_collection_enabled() {
        let _ = crate::mmtk().disable_collection();
    }
}

#[inline(always)]
pub fn gc_enabled_p() -> bool {
    crate::mmtk().is_collection_enabled()
}

// =============== Object allocation ===============

#[inline(always)]
pub fn max_non_los_default_alloc_bytes() -> usize {
    mmtk()
        .get_plan()
        .constraints()
        .max_non_los_default_alloc_bytes
}

/// # Safety
/// `mutator` must point to a valid `RubyMutator`.
#[inline(always)]
pub unsafe fn alloc(
    mutator: *mut RubyMutator,
    size: usize,
    align: usize,
    offset: usize,
    semantics: AllocationSemantics,
) -> Address {
    let mutator_ref = unsafe { assert_non_null_mut(mutator, "RubyMutator") };
    let clamped_size = size.max(MIN_OBJECT_SIZE);
    memory_manager::alloc::<Ruby>(mutator_ref, clamped_size, align, offset, semantics)
}

/// # Safety
/// `mutator` must point to a valid `RubyMutator`.
#[inline(always)]
pub unsafe fn post_alloc(
    mutator: *mut RubyMutator,
    refer: ObjectReference,
    bytes: usize,
    semantics: AllocationSemantics,
) {
    let mutator_ref = unsafe { assert_non_null_mut(mutator, "RubyMutator") };
    assert_object_reference(refer);
    memory_manager::post_alloc::<Ruby>(mutator_ref, refer, bytes, semantics)
}

/// # Safety
/// If `count > 0`, `objects` must point to a contiguous array of `count` `ObjectReference`s.
#[inline(always)]
pub unsafe fn add_obj_free_candidates(
    objects: *const ObjectReference,
    count: usize,
    can_parallel_free: bool,
) {
    if count > 0 {
        let _ = unsafe { assert_non_null_ref(objects, "objects") };
    }
    let objects_slice = unsafe { std::slice::from_raw_parts(objects, count) };
    binding()
        .weak_proc
        .add_obj_free_candidates_batch(objects_slice, can_parallel_free)
}

// =============== Weak references ===============

#[inline(always)]
pub fn declare_weak_references(object: ObjectReference) {
    assert_object_reference(object);
    binding().weak_proc.add_weak_reference(object);
}

#[inline(always)]
pub fn weak_references_alive_p(object: ObjectReference) -> bool {
    assert_object_reference(object);
    object.is_reachable()
}

#[inline(always)]
pub fn weak_references_count() -> usize {
    binding().weak_proc.weak_references_count()
}

// =============== Compaction ===============

#[inline(always)]
pub fn register_pinning_obj(obj: ObjectReference) {
    assert_object_reference(obj);
    crate::binding().pinning_registry.register(obj);
}

#[inline(always)]
pub fn is_pinned(object: ObjectReference) -> bool {
    assert_object_reference(object);
    memory_manager::is_pinned(object)
}

// =============== Write barriers ===============

/// # Safety
/// `mutator` must point to a valid `RubyMutator`.
#[inline(always)]
pub unsafe fn object_reference_write_post(mutator: *mut RubyMutator, object: ObjectReference) {
    let mutator_ref = unsafe { assert_non_null_mut(mutator, "RubyMutator") };
    assert_object_reference(object);
    let ignored_slot = RubySlot::from_address(Address::ZERO);
    let ignored_target = ObjectReference::from_raw_address(Address::ZERO);
    mmtk::memory_manager::object_reference_write_post(
        mutator_ref,
        object,
        ignored_slot,
        ignored_target,
    )
}

#[inline(always)]
pub fn register_wb_unprotected_object(object: ObjectReference) {
    assert_object_reference(object);
    crate::binding().register_wb_unprotected_object(object)
}

#[inline(always)]
pub fn object_wb_unprotected_p(object: ObjectReference) -> bool {
    assert_object_reference(object);
    crate::binding().object_wb_unprotected_p(object)
}

// =============== Heap walking ===============

#[inline(always)]
pub fn enumerate_objects(
    callback: extern "C" fn(ObjectReference, *mut libc::c_void),
    data: *mut libc::c_void,
) {
    crate::mmtk().enumerate_objects(|object| {
        callback(object, data);
    })
}

// =============== Finalizers ===============

#[inline(always)]
pub fn get_all_obj_free_candidates() -> RawVecOfObjRef {
    let vec = binding().weak_proc.get_all_obj_free_candidates();
    RawVecOfObjRef::from_vec(vec)
}

#[inline(always)]
pub fn free_raw_vec_of_obj_ref(raw_vec: RawVecOfObjRef) {
    unsafe { raw_vec.into_vec() };
}

// =============== Forking ===============

#[inline(always)]
pub fn before_fork() {
    mmtk().prepare_to_fork();
    binding().join_all_gc_threads();
}

#[inline(always)]
pub fn after_fork(tls: VMThread) {
    mmtk().after_fork(tls);
}

// =============== Statistics ===============

#[inline(always)]
pub fn total_bytes() -> usize {
    memory_manager::total_bytes(mmtk())
}

#[inline(always)]
pub fn used_bytes() -> usize {
    memory_manager::used_bytes(mmtk())
}

#[inline(always)]
pub fn free_bytes() -> usize {
    memory_manager::free_bytes(mmtk())
}

#[inline(always)]
pub fn starting_heap_address() -> Address {
    memory_manager::starting_heap_address()
}

#[inline(always)]
pub fn last_heap_address() -> Address {
    memory_manager::last_heap_address()
}

#[inline(always)]
pub fn worker_count() -> usize {
    memory_manager::num_of_workers(mmtk())
}

#[inline(always)]
pub fn plan() -> *const u8 {
    static NO_GC: &[u8] = b"NoGC\0";
    static MARK_SWEEP: &[u8] = b"MarkSweep\0";
    static IMMIX: &[u8] = b"Immix\0";
    static STICKY_IMMIX: &[u8] = b"StickyImmix\0";

    match *crate::BINDING.get().unwrap().mmtk.get_options().plan {
        PlanSelector::NoGC => NO_GC.as_ptr(),
        PlanSelector::MarkSweep => MARK_SWEEP.as_ptr(),
        PlanSelector::Immix => IMMIX.as_ptr(),
        PlanSelector::StickyImmix => STICKY_IMMIX.as_ptr(),
        _ => panic!("Unknown plan"),
    }
}

#[inline(always)]
pub fn heap_mode() -> *const u8 {
    static FIXED_HEAP: &[u8] = b"fixed\0";
    static DYNAMIC_HEAP: &[u8] = b"dynamic\0";
    static RUBY_HEAP: &[u8] = b"ruby\0";
    static CPU_HEAP: &[u8] = b"cpu\0";

    match *crate::BINDING.get().unwrap().mmtk.get_options().gc_trigger {
        GCTriggerSelector::FixedHeapSize(_) => FIXED_HEAP.as_ptr(),
        GCTriggerSelector::DynamicHeapSize(_, _) => DYNAMIC_HEAP.as_ptr(),
        GCTriggerSelector::Delegated => {
            if CPU_HEAP_TRIGGER_CONFIG.get().is_some() {
                CPU_HEAP.as_ptr()
            } else {
                RUBY_HEAP.as_ptr()
            }
        }
    }
}

#[inline(always)]
pub fn heap_min() -> usize {
    match *crate::BINDING.get().unwrap().mmtk.get_options().gc_trigger {
        GCTriggerSelector::FixedHeapSize(_) => 0,
        GCTriggerSelector::DynamicHeapSize(min_size, _) => min_size,
        GCTriggerSelector::Delegated => {
            if let Some(cfg) = CPU_HEAP_TRIGGER_CONFIG.get() {
                conversions::pages_to_bytes(cfg.min_heap_pages)
            } else {
                conversions::pages_to_bytes(
                    RUBY_HEAP_TRIGGER_CONFIG
                        .get()
                        .expect("RUBY_HEAP_TRIGGER_CONFIG not set")
                        .min_heap_pages,
                )
            }
        }
    }
}

#[inline(always)]
pub fn heap_max() -> usize {
    match *crate::BINDING.get().unwrap().mmtk.get_options().gc_trigger {
        GCTriggerSelector::FixedHeapSize(max_size) => max_size,
        GCTriggerSelector::DynamicHeapSize(_, max_size) => max_size,
        GCTriggerSelector::Delegated => {
            if let Some(cfg) = CPU_HEAP_TRIGGER_CONFIG.get() {
                conversions::pages_to_bytes(cfg.max_heap_pages)
            } else {
                conversions::pages_to_bytes(
                    RUBY_HEAP_TRIGGER_CONFIG
                        .get()
                        .expect("RUBY_HEAP_TRIGGER_CONFIG not set")
                        .max_heap_pages,
                )
            }
        }
    }
}

// =============== Miscellaneous ===============

#[inline(always)]
pub fn is_mmtk_object(addr: Address) -> bool {
    assert_address_aligned(addr, mmtk::util::is_mmtk_object::VO_BIT_REGION_SIZE);
    memory_manager::is_mmtk_object(addr).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_object_reference_valid() {
        let valid_addr = unsafe { Address::from_usize(0x1000) };
        let obj = unsafe { ObjectReference::from_raw_address_unchecked(valid_addr) };
        assert_object_reference(obj);
    }

    #[test]
    #[should_panic]
    fn test_assert_object_reference_null_panics() {
        let null_addr = Address::ZERO;
        let obj = unsafe { ObjectReference::from_raw_address_unchecked(null_addr) };
        assert_object_reference(obj);
    }

    #[test]
    #[should_panic]
    fn test_assert_object_reference_unaligned_panics() {
        let unaligned_addr = unsafe { Address::from_usize(0x1003) };
        let obj = unsafe { ObjectReference::from_raw_address_unchecked(unaligned_addr) };
        assert_object_reference(obj);
    }

    #[test]
    #[should_panic(expected = "Address must not be zero")]
    fn test_assert_address_zero_panics() {
        assert_address(Address::ZERO);
    }

    #[test]
    #[should_panic(expected = "is not aligned to 8")]
    fn test_assert_address_unaligned_panics() {
        let unaligned_addr = unsafe { Address::from_usize(0x1003) };
        assert_address_aligned(unaligned_addr, 8);
    }

    #[test]
    #[should_panic(expected = "dummy pointer must not be null")]
    fn test_assert_non_null_ref_null_panics() {
        let null_ptr: *const u8 = std::ptr::null();
        unsafe { assert_non_null_ref(null_ptr, "dummy") };
    }
}
