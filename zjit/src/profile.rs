//! Profiler for runtime information.

// We use the YARV bytecode constants which have a CRuby-style name
#![allow(non_upper_case_globals)]

use std::collections::HashMap;
use crate::{cruby::*, payload::get_or_create_iseq_payload, options::{get_option, NumProfiles}};
use crate::distribution::{Distribution, DistributionSummary};
use crate::stats::Counter::profile_time_ns;
use crate::stats::with_time_stat;

/// Safe visitor trait for extracting frame snapshot state during instruction profiling.
pub trait FrameVisitor {
    fn iseq(&self) -> IseqPtr;
    fn insn_idx(&self) -> YarvInsnIdx;
    fn insn_opnd(&self, idx: usize) -> VALUE;
    fn peek_at_stack(&self, n: isize) -> VALUE;
    fn peek_at_self(&self) -> VALUE;
    fn peek_at_block_handler(&self) -> VALUE;
    fn frame_method_entry(&self) -> *const rb_callable_method_entry_t;
}

/// Immutable snapshot of an execution frame captured during profiling.
pub struct FrameSnapshot {
    iseq: IseqPtr,
    insn_idx: YarvInsnIdx,
    self_val: VALUE,
    block_handler: VALUE,
    method_entry: *const rb_callable_method_entry_t,
    insn_opnds: [VALUE; 8],
    stack_opnds: [VALUE; 64],
}

impl FrameVisitor for FrameSnapshot {
    fn iseq(&self) -> IseqPtr {
        self.iseq
    }

    fn insn_idx(&self) -> YarvInsnIdx {
        self.insn_idx
    }

    fn insn_opnd(&self, idx: usize) -> VALUE {
        if idx < self.insn_opnds.len() {
            self.insn_opnds[idx]
        } else {
            VALUE(0)
        }
    }

    fn peek_at_stack(&self, n: isize) -> VALUE {
        if n >= 0 && (n as usize) < self.stack_opnds.len() {
            self.stack_opnds[n as usize]
        } else {
            VALUE(0)
        }
    }

    fn peek_at_self(&self) -> VALUE {
        self.self_val
    }

    fn peek_at_block_handler(&self) -> VALUE {
        self.block_handler
    }

    fn frame_method_entry(&self) -> *const rb_callable_method_entry_t {
        self.method_entry
    }
}

/// Profiling context that captures frame snapshots during profiling passes.
pub struct ProfilingContext {
    snapshot: FrameSnapshot,
}

impl ProfilingContext {
    /// Capture an immutable frame snapshot from the execution context.
    ///
    /// Unsafe frame pointer reads are strictly isolated to this capture step.
    pub fn capture(ec: EcPtr) -> Self {
        let snapshot = unsafe {
            let cfp = get_ec_cfp(ec);
            let iseq = get_cfp_iseq(cfp);
            let pc = get_cfp_pc(cfp);
            let encoded_pc = get_iseq_body_iseq_encoded(iseq);
            let insn_idx = pc.offset_from(encoded_pc) as usize;
            let self_val = rb_get_cfp_self(cfp);
            let block_handler = rb_vm_get_untagged_block_handler(cfp);
            let method_entry = rb_vm_frame_method_entry(cfp);

            let mut insn_opnds = [VALUE(0); 8];
            if !iseq.is_null() {
                let iseq_size = get_iseq_encoded_size(iseq) as usize;
                for i in 0..8 {
                    if insn_idx + 1 + i < iseq_size {
                        insn_opnds[i] = pc.add(1 + i).read();
                    }
                }
            }

            let sp = get_cfp_sp(cfp);
            let bp = get_cfp_bp(cfp);
            let mut stack_opnds = [VALUE(0); 64];
            if !bp.is_null() && sp > bp {
                let avail = sp.offset_from(bp) as usize;
                let copy_len = avail.min(64);
                for i in 0..copy_len {
                    stack_opnds[i] = *sp.offset(-1 - i as isize);
                }
            }

            FrameSnapshot {
                iseq,
                insn_idx,
                self_val,
                block_handler,
                method_entry,
                insn_opnds,
                stack_opnds,
            }
        };

        ProfilingContext { snapshot }
    }

    pub fn snapshot(&self) -> &FrameSnapshot {
        &self.snapshot
    }
}

/// API called from zjit_* instruction. opcode is the bare (non-zjit_*) instruction.
#[unsafe(no_mangle)]
pub extern "C" fn rb_zjit_profile_insn(bare_opcode: u32, ec: EcPtr) {
    with_vm_lock(src_loc!(), || {
        with_time_stat(profile_time_ns, || profile_insn(bare_opcode as ruby_vminsn_type, ec));
    });
}

/// Profile a YARV instruction
fn profile_insn_sample(
    bare_opcode: ruby_vminsn_type,
    visitor: &impl FrameVisitor,
    profile: &mut IseqProfile,
) -> bool {
    match bare_opcode {
        YARVINSN_opt_nil_p => profile_operands(visitor, profile, 1),
        YARVINSN_opt_plus  => profile_operands(visitor, profile, 2),
        YARVINSN_opt_minus => profile_operands(visitor, profile, 2),
        YARVINSN_opt_mult  => profile_operands(visitor, profile, 2),
        YARVINSN_opt_div   => profile_operands(visitor, profile, 2),
        YARVINSN_opt_mod   => profile_operands(visitor, profile, 2),
        YARVINSN_opt_eq    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_neq   => profile_operands(visitor, profile, 2),
        YARVINSN_opt_lt    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_le    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_gt    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_ge    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_and   => profile_operands(visitor, profile, 2),
        YARVINSN_opt_or    => profile_operands(visitor, profile, 2),
        YARVINSN_opt_empty_p => profile_operands(visitor, profile, 1),
        YARVINSN_opt_aref  => profile_operands(visitor, profile, 2),
        YARVINSN_opt_ltlt  => profile_operands(visitor, profile, 2),
        YARVINSN_opt_aset  => profile_operands(visitor, profile, 3),
        YARVINSN_opt_not   => profile_operands(visitor, profile, 1),
        YARVINSN_getinstancevariable => profile_self(visitor, profile),
        YARVINSN_setinstancevariable => profile_self(visitor, profile),
        YARVINSN_definedivar   => profile_self(visitor, profile),
        YARVINSN_opt_regexpmatch2    => profile_operands(visitor, profile, 2),
        YARVINSN_objtostring   => profile_operands(visitor, profile, 1),
        YARVINSN_opt_length    => profile_operands(visitor, profile, 1),
        YARVINSN_opt_size      => profile_operands(visitor, profile, 1),
        YARVINSN_opt_succ      => profile_operands(visitor, profile, 1),
        YARVINSN_invokeblock   => profile_block_handler(visitor, profile),
        YARVINSN_invokesuper   => profile_invokesuper(visitor, profile),
        YARVINSN_opt_send_without_block | YARVINSN_send => {
            let cd: *const rb_call_data = visitor.insn_opnd(0).as_ptr();
            let argc = num_arguments_on_stack(cd);
            // Profile all the arguments and self (+1).
            profile_operands(visitor, profile, argc + 1);
            profile_splat_length(visitor, profile, unsafe { (*cd).ci });
        }
        YARVINSN_splatkw => profile_operands(visitor, profile, 2),
        _ => return false,
    }

    true
}

/// Profile a YARV instruction
fn profile_insn(bare_opcode: ruby_vminsn_type, ec: EcPtr) {
    let context = ProfilingContext::capture(ec);
    let visitor = context.snapshot();
    let profile = &mut get_or_create_iseq_payload(visitor.iseq()).profile;
    let _ = profile_insn_sample(bare_opcode, visitor, profile);

    // Once we profile the instruction enough times, we stop profiling it.
    let entry = profile.entry_mut(visitor.insn_idx());
    entry.profiles_remaining = entry.profiles_remaining.saturating_sub(1);
    if entry.profiles_remaining == 0 {
        unsafe { rb_zjit_iseq_insn_set(visitor.iseq(), visitor.insn_idx() as u32, bare_opcode); }
    }
}

/// Reset existing profile counters and install profiling instructions throughout an ISEQ.
/// Newly reached instructions initialize their counters from the same option.
pub(crate) fn reset_profiles_remaining(iseq: IseqPtr) {
    let profile = &mut get_or_create_iseq_payload(iseq).profile;
    let num_profiles = get_option!(num_profiles);
    for entry in &mut profile.entries {
        entry.profiles_remaining = num_profiles;
    }
    unsafe { rb_zjit_profile_enable(iseq) };
}

/// Return the argc as stated in the calldata plus:
/// * 1 if there is an explicit blockarg, since that will be passed on the stack
pub fn num_arguments_on_stack(cd: *const rb_call_data) -> usize {
    let ci = unsafe { (*cd).ci };
    let flags = unsafe { rb_vm_ci_flag(ci) };
    let has_blockarg = (flags & VM_CALL_ARGS_BLOCKARG) != 0;
    (unsafe { vm_ci_argc(ci) }) as usize + has_blockarg as usize
}

pub const DISTRIBUTION_SIZE: usize = 8;

pub type TypeDistribution = Distribution<ProfiledType, DISTRIBUTION_SIZE>;

pub type TypeDistributionSummary = DistributionSummary<ProfiledType, DISTRIBUTION_SIZE>;

pub type SplatLength = u32;

/// `None` records an unknown length so this distribution covers the same
/// executions as the operand type profile.
pub type SplatLengthDistribution = Distribution<Option<SplatLength>, DISTRIBUTION_SIZE>;

pub type SplatLengthDistributionSummary = DistributionSummary<Option<SplatLength>, DISTRIBUTION_SIZE>;

/// Profile the Type of top-`n` stack operands
fn profile_operands(visitor: &impl FrameVisitor, profile: &mut IseqProfile, n: usize) {
    let entry = profile.entry_mut(visitor.insn_idx());
    if entry.opnd_types.is_empty() {
        // Allocate exactly `n` distributions. A plain `resize` on an empty Vec rounds the capacity
        // up to 4 elements, which might waste space.
        entry.opnd_types = vec![TypeDistribution::new(); n];
    }

    for (i, profile_type) in entry.opnd_types.iter_mut().enumerate() {
        let obj = visitor.peek_at_stack((n - i - 1) as isize);
        // TODO(max): Handle GC-hidden classes like Array, Hash, etc and make them look normal or
        // drop them or something
        let ty = ProfiledType::new(obj);
        VALUE::from(visitor.iseq()).write_barrier(ty.class());
        profile_type.observe(ty);
    }
}

fn profile_splat_length(visitor: &impl FrameVisitor, profile: &mut IseqProfile, ci: *const rb_callinfo) {
    let flags = unsafe { rb_vm_ci_flag(ci) };
    // Only call sites with VM_CALL_ARGS_SPLAT have a splat array on the stack.
    if flags & VM_CALL_ARGS_SPLAT == 0 {
        return;
    }

    let kwarg = unsafe { rb_vm_ci_kwarg(ci) };
    let caller_kw_count = if kwarg.is_null() { 0 } else { (unsafe { get_cikw_keyword_len(kwarg) }) as usize };
    // Starting at the top of the stack, skip the block argument, keyword-splat
    // hash, and explicit keyword values to reach the splat array.
    let splat_pos = usize::from(flags & VM_CALL_ARGS_BLOCKARG != 0)
        + usize::from(flags & VM_CALL_KW_SPLAT != 0)
        + caller_kw_count;
    let splat_array = visitor.peek_at_stack(splat_pos as isize);
    let length = if unsafe { RB_TYPE_P(splat_array, RUBY_T_ARRAY) } {
        SplatLength::try_from(unsafe { rb_jit_array_len(splat_array) }).ok()
    } else {
        None
    };
    profile.splat_lengths.entry(visitor.insn_idx())
        .or_insert_with(SplatLengthDistribution::new).observe(length);
}

fn profile_self(visitor: &impl FrameVisitor, profile: &mut IseqProfile) {
    let entry = profile.entry_mut(visitor.insn_idx());
    if entry.opnd_types.is_empty() {
        entry.opnd_types = vec![TypeDistribution::new()];
    }
    let obj = visitor.peek_at_self();
    // TODO(max): Handle GC-hidden classes like Array, Hash, etc and make them look normal or
    // drop them or something
    let ty = ProfiledType::new(obj);
    VALUE::from(visitor.iseq()).write_barrier(ty.class());
    entry.opnd_types[0].observe(ty);
}

fn profile_block_handler(visitor: &impl FrameVisitor, profile: &mut IseqProfile) {
    let entry = profile.entry_mut(visitor.insn_idx());
    if entry.opnd_types.is_empty() {
        entry.opnd_types = vec![TypeDistribution::new()];
    }
    let ty = ProfiledType::block_handler(visitor.peek_at_block_handler());
    VALUE::from(visitor.iseq()).write_barrier(ty.class());
    entry.opnd_types[0].observe(ty);
}

fn profile_invokesuper(visitor: &impl FrameVisitor, profile: &mut IseqProfile) {
    let cme = visitor.frame_method_entry();
    let cme_value = VALUE(cme as usize);  // CME is a T_IMEMO, which is a VALUE

    profile.super_cme.entry(visitor.insn_idx())
        .or_insert_with(|| TypeDistribution::new()).observe(ProfiledType::object(cme_value));

    unsafe { rb_gc_writebarrier(visitor.iseq().into(), cme_value) };

    let cd: *const rb_call_data = visitor.insn_opnd(0).as_ptr();
    let argc = num_arguments_on_stack(cd);

    // Profile all the arguments and self (+1).
    profile_operands(visitor, profile, (argc + 1) as usize);
    profile_splat_length(visitor, profile, unsafe { (*cd).ci });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags(u32);

impl Flags {
    const NONE: u32 = 0;
    const IS_IMMEDIATE: u32 = 1 << 0;
    /// Object is embedded and the ivar index lands within the object
    const IS_EMBEDDED: u32 = 1 << 1;
    /// Object is a T_OBJECT
    const IS_T_OBJECT: u32 = 1 << 2;
    /// Object is a struct with embedded fields
    const IS_STRUCT_EMBEDDED: u32 = 1 << 3;
    /// Set if the ProfiledType is used for profiling specific objects, not just classes/shapes
    const IS_OBJECT_PROFILING: u32 = 1 << 4;
    /// The profiled block handler is an IFUNC. The imemo itself is not retained.
    const IS_IFUNC_BLOCK_HANDLER: u32 = 1 << 5;
    /// The profiled block handler is a Proc. The Proc object itself is not retained.
    const IS_PROC_BLOCK_HANDLER: u32 = 1 << 6;

    pub fn none() -> Self { Self(Self::NONE) }

    pub fn immediate() -> Self { Self(Self::IS_IMMEDIATE) }
    pub fn is_immediate(self) -> bool { (self.0 & Self::IS_IMMEDIATE) != 0 }
    pub fn is_embedded(self) -> bool { (self.0 & Self::IS_EMBEDDED) != 0 }
    pub fn is_t_object(self) -> bool { (self.0 & Self::IS_T_OBJECT) != 0 }
    pub fn is_struct_embedded(self) -> bool { (self.0 & Self::IS_STRUCT_EMBEDDED) != 0 }
    pub fn is_object_profiling(self) -> bool { (self.0 & Self::IS_OBJECT_PROFILING) != 0 }
    pub fn is_ifunc_block_handler(self) -> bool { (self.0 & Self::IS_IFUNC_BLOCK_HANDLER) != 0 }
    pub fn is_proc_block_handler(self) -> bool { (self.0 & Self::IS_PROC_BLOCK_HANDLER) != 0 }
}

/// opt_send_without_block/opt_plus/... should store:
/// * the class of the receiver, so we can do method lookup
/// * the shape of the receiver, so we can optimize ivar lookup
///
/// with those two, pieces of information, we can also determine when an object is an immediate:
/// * Integer + IS_IMMEDIATE == Fixnum
/// * Float + IS_IMMEDIATE == Flonum
/// * Symbol + IS_IMMEDIATE == StaticSymbol
/// * NilClass == Nil
/// * TrueClass == True
/// * FalseClass == False
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfiledType {
    class: VALUE,
    shape: ShapeId,
    flags: Flags,
}

impl Default for ProfiledType {
    fn default() -> Self {
        Self::empty()
    }
}

impl ProfiledType {
    /// Profile the object itself
    fn object(obj: VALUE) -> Self {
        let mut flags = Flags::none();
        flags.0 |= Flags::IS_OBJECT_PROFILING;
        Self { class: obj, shape: INVALID_SHAPE_ID, flags }
    }

    /// Profile an untagged block handler. ISEQ, Symbol, and no-block handlers are profiled as the object itself.
    /// IFUNC and Proc handlers are recorded only as a flag to avoid retaining them and to reduce GC marking pressure.
    fn block_handler(block_handler: VALUE) -> Self {
        let flag = if unsafe { rb_IMEMO_TYPE_P(block_handler, imemo_ifunc) == 1 } {
            Flags::IS_IFUNC_BLOCK_HANDLER
        } else if unsafe { rb_obj_is_proc(block_handler).test() } {
            Flags::IS_PROC_BLOCK_HANDLER
        } else {
            return Self::object(block_handler);
        };
        let mut ty = Self::object(Qnil);
        ty.flags.0 |= flag;
        ty
    }

    /// Profile the class and shape of the given object
    fn new(obj: VALUE) -> Self {
        // Qundef must never escape the VM internals; rb_class_of(Qundef) is undefined
        debug_assert_ne!(obj, Qundef, "should not profile Qundef");
        if obj.special_const_p() {
            return Self { class: obj.class_of(),
                          shape: INVALID_SHAPE_ID,
                          flags: Flags::immediate() };
        }
        let mut flags = Flags::none();
        let shape = obj.shape_id_of();
        if shape.layout() == ShapeLayout::RObject {
            flags.0 |= Flags::IS_EMBEDDED;
        }
        if obj.struct_embedded_p() {
            flags.0 |= Flags::IS_STRUCT_EMBEDDED;
        }
        if unsafe { RB_TYPE_P(obj, RUBY_T_OBJECT) } {
            flags.0 |= Flags::IS_T_OBJECT;
        }
        Self { class: obj.class_of(), shape, flags }
    }

    pub fn empty() -> Self {
        Self { class: VALUE(0), shape: INVALID_SHAPE_ID, flags: Flags::none() }
    }

    pub fn is_empty(&self) -> bool {
        self.class == VALUE(0)
    }

    pub fn class(&self) -> VALUE {
        self.class
    }

    pub fn shape(&self) -> ShapeId {
        self.shape
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn is_fixnum(&self) -> bool {
        self.class == unsafe { rb_cInteger } && self.flags.is_immediate()
    }

    /// Whether the profiled class is exactly Proc (subclasses return false).
    ///
    /// This is stricter than the interpreter, which accepts anything for which
    /// `rb_obj_is_proc()` is true. That checks the object's typed data type, not its
    /// class, so Proc subclasses pass. We use the class as a conservative approximation:
    /// Proc has no allocator, so an object whose class is exactly Proc always has
    /// `proc_data_type`. Subclasses fall back to a dynamic send.
    pub fn is_proc(&self) -> bool {
        self.class == unsafe { rb_cProc }
    }

    pub fn is_string(&self) -> bool {
        if self.flags.is_object_profiling() {
            panic!("should not call is_string on object-profiled ProfiledType");
        }
        // Fast paths for immediates and exact-class
        if self.flags.is_immediate() {
            return false;
        }

        let string = unsafe { rb_cString };
        if self.class == string{
            return true;
        }

        self.class.is_subclass_of(string) == ClassRelationship::Subclass
    }

    pub fn is_flonum(&self) -> bool {
        self.class == unsafe { rb_cFloat } && self.flags.is_immediate()
    }

    pub fn is_static_symbol(&self) -> bool {
        self.class == unsafe { rb_cSymbol } && self.flags.is_immediate()
    }

    pub fn is_nil(&self) -> bool {
        self.class == unsafe { rb_cNilClass } && self.flags.is_immediate()
    }

    pub fn is_true(&self) -> bool {
        self.class == unsafe { rb_cTrueClass } && self.flags.is_immediate()
    }

    pub fn is_false(&self) -> bool {
        self.class == unsafe { rb_cFalseClass } && self.flags.is_immediate()
    }
}

/// Per-instruction profile entry, stored sparsely in a sorted Vec.
#[derive(Debug)]
pub struct ProfileEntry {
    /// YARV instruction index
    insn_idx: u32,
    /// Type information of YARV instruction operands
    opnd_types: Vec<TypeDistribution>,
    /// Number of profiles remaining before recompilation. Counts down from --zjit-num-profiles.
    profiles_remaining: NumProfiles,
}

#[derive(Debug)]
pub struct IseqProfile {
    /// Sparse storage of per-instruction profile data, sorted by instruction index.
    /// Only instructions that have actually been profiled have entries here.
    entries: Vec<ProfileEntry>,

    /// Method entries for `super` calls (stored as VALUE to be GC-safe)
    super_cme: HashMap<YarvInsnIdx, TypeDistribution>,

    /// Observed lengths of caller splat arrays for call instructions.
    splat_lengths: HashMap<YarvInsnIdx, SplatLengthDistribution>,
}

impl IseqProfile {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            super_cme: HashMap::new(),
            splat_lengths: HashMap::new(),
        }
    }

    /// Get or create a mutable profile entry for the given instruction index.
    pub fn entry_mut(&mut self, insn_idx: YarvInsnIdx) -> &mut ProfileEntry {
        let idx = insn_idx as u32;
        match self.entries.binary_search_by_key(&idx, |e| e.insn_idx) {
            Ok(i) => &mut self.entries[i],
            Err(i) => {
                self.entries.insert(i, ProfileEntry {
                    insn_idx: idx,
                    opnd_types: Vec::new(),
                    profiles_remaining: get_option!(num_profiles),
                });
                &mut self.entries[i]
            }
        }
    }

    /// Get a profile entry for the given instruction index (read-only).
    fn entry(&self, insn_idx: YarvInsnIdx) -> Option<&ProfileEntry> {
        let idx = insn_idx as u32;
        self.entries.binary_search_by_key(&idx, |e| e.insn_idx)
            .ok().map(|i| &self.entries[i])
    }

    /// Get profiled operand types for a given instruction index
    pub fn get_operand_types(&self, insn_idx: YarvInsnIdx) -> Option<&[TypeDistribution]> {
        self.entry(insn_idx).map(|e| e.opnd_types.as_slice()).filter(|s| !s.is_empty())
    }

    pub fn get_splat_length_summary(&self, insn_idx: YarvInsnIdx) -> Option<SplatLengthDistributionSummary> {
        self.splat_lengths.get(&insn_idx)
            .map(SplatLengthDistributionSummary::new)
    }

    pub fn get_super_method_entry(&self, insn_idx: YarvInsnIdx) -> Option<*const rb_callable_method_entry_t> {
        let Some(entry) = self.super_cme.get(&insn_idx) else { return None };
        let summary = TypeDistributionSummary::new(entry);

        if summary.is_monomorphic() {
            Some(summary.bucket(0).class.0 as *const rb_callable_method_entry_t)
        } else {
            None
        }
    }

    /// Run a given callback with every object in IseqProfile
    pub fn each_object(&self, callback: impl Fn(VALUE)) {
        for entry in &self.entries {
            for distribution in &entry.opnd_types {
                for profiled_type in distribution.each_item() {
                    // If the type is a GC object, call the callback
                    callback(profiled_type.class);
                }
            }
        }

        for super_cme_values in self.super_cme.values() {
            for profiled_type in super_cme_values.each_item() {
                callback(profiled_type.class)
            }
        }
    }

    /// Run a given callback with a mutable reference to every object in IseqProfile.
    pub fn each_object_mut(&mut self, callback: impl Fn(&mut VALUE)) {
        for entry in &mut self.entries {
            for distribution in &mut entry.opnd_types {
                for ref mut profiled_type in distribution.each_item_mut() {
                    // If the type is a GC object, call the callback
                    callback(&mut profiled_type.class);
                }
            }
        }

        // Update CME references if they move during compaction.
        for super_cme_values in self.super_cme.values_mut() {
            for ref mut profiled_type in super_cme_values.each_item_mut() {
                callback(&mut profiled_type.class)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cruby::*;

    #[test]
    fn can_profile_block_handler() {
        with_rubyvm(|| eval("
            def foo = yield
            foo rescue 0
            foo rescue 0
        "));
    }
}
