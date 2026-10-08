# C-to-Rust Porting Ledger

Automated migration ledger tracking C source files, line counts, and Rust porting status.
Generated automatically by `ruby tool/generate_porting_ledger.rb`. Do not edit manually.

## Summary Statistics

| Metric | File Count | Lines of Code (LOC) | % of Total LOC |
| :--- | :--- | :--- | :--- |
| **Total C Source Files** | 113 | 317,465 | 100.0% |
| **Not Started** | 103 | 308,463 | 97.2% |
| **In Progress** | 3 | 8,656 | 2.7% |
| **Ported** | 0 | 0 | 0.0% |
| **Blocked** | 0 | 0 | 0.0% |
| **N/A** | 7 | 346 | 0.1% |

## Rust Unsafe Block Density Metrics

| Target Rust Crate/Module | Rust Lines (LOC) | Unsafe Blocks | Unsafe Lines | Unsafe Line % |
| :--- | :--- | :--- | :--- | :--- |
| `core_rs::complex` | 386 | 21 | 50 | 13.0% |
| `core_rs::re` | 339 | 9 | 70 | 20.6% |
| `core_rs::util` | 411 | 30 | 73 | 17.8% |
| `gc::mmtk` | 3,079 | 57 | 130 | 4.2% |
| `jit` | 38 | 9 | 25 | 65.8% |
| `yjit` | 36,691 | 614 | 2,221 | 6.1% |
| `zjit` | 95,856 | 961 | 3,056 | 3.2% |

## Migration Progress by Subsystem

### JIT Compiler
_Just-In-Time compilers and execution machinery_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `compile.c` | 15,441 | Not Started | `crate::compile` | 0 | 0.0% | Bytecode compiler |
| `iseq.c` | 4,735 | Not Started | `crate::iseq` | 0 | 0.0% | Instruction sequences |
| `jit.c` | 965 | Not Started | `jit` | 9 | 65.8% | C helpers shared by upstream YJIT/ZJIT |
| `yjit.c` | 603 | Not Started | `yjit` | 614 | 6.1% | yjit.c is the C side of upstream YJIT; YJIT itself is upstream Rust, not a port by this project |
| `zjit.c` | 406 | Not Started | `zjit` | 961 | 3.2% | zjit.c is the C side of upstream ZJIT; ZJIT itself is upstream Rust, not a port by this project |

### Core Data Structures
_Built-in data types, objects, and core classes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `array.c` | 9,137 | Not Started | `crate::array` | 0 | 0.0% | Array object implementation |
| `bignum.c` | 7,350 | Not Started | `crate::bignum` | 0 | 0.0% | Big integer support |
| `class.c` | 3,412 | Not Started | `crate::class` | 0 | 0.0% | Class and module hierarchy |
| `compar.c` | 355 | Not Started | `crate::compar` | 0 | 0.0% | Comparable mixin module |
| `complex.c` | 2,839 | In Progress | `core_rs::complex` | 21 | 13.0% | Ported (complex-A-01, string scanner statics): issign, read_sign, isdecimal, read_rat_nos, read_rat, isimagunit, skip_ws and helpers. Remaining: Complex class methods |
| `enum.c` | 5,310 | Not Started | `crate::enum` | 0 | 0.0% | Enumerable module |
| `enumerator.c` | 4,794 | Not Started | `crate::enumerator` | 0 | 0.0% | Enumerator class |
| `hash.c` | 7,993 | Not Started | `crate::hash` | 0 | 0.0% | Hash map implementation |
| `numeric.c` | 6,813 | Not Started | `crate::numeric` | 0 | 0.0% | Numeric base classes and operations |
| `object.c` | 4,741 | Not Started | `crate::object` | 0 | 0.0% | Object class methods and operations |
| `pack.c` | 1,936 | Not Started | `crate::pack` | 0 | 0.0% | Array#pack and String#unpack |
| `proc.c` | 5,543 | Not Started | `crate::proc` | 0 | 0.0% | Proc, Method, and Binding objects |
| `range.c` | 3,006 | Not Started | `crate::range` | 0 | 0.0% | Range object implementation |
| `rational.c` | 2,847 | Not Started | `crate::rational` | 0 | 0.0% | Rational number implementation |
| `set.c` | 2,677 | Not Started | `crate::set` | 0 | 0.0% | Core Set class support |
| `string.c` | 14,476 | Not Started | `crate::string` | 0 | 0.0% | String object implementation |
| `struct.c` | 2,352 | Not Started | `crate::struct` | 0 | 0.0% | Struct class implementation |
| `symbol.c` | 1,467 | Not Started | `crate::symbol` | 0 | 0.0% | Symbol management |
| `time.c` | 6,101 | Not Started | `crate::time` | 0 | 0.0% | Time class and operations |
| `weakmap.c` | 999 | Not Started | `crate::weakmap` | 0 | 0.0% | ObjectSpace::WeakMap |

### Virtual Machine & Execution
_Interpreter VM loop, instruction helpers, and evaluation_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `cont.c` | 3,908 | Not Started | `crate::cont` | 0 | 0.0% | Continuation and Fiber core |
| `eval.c` | 2,344 | Not Started | `crate::eval` | 0 | 0.0% | Top-level evaluation entry points |
| `eval_error.c` | 588 | Not Started | `crate::eval::error` | 0 | 0.0% | Evaluation error handling |
| `eval_jump.c` | 144 | Not Started | `crate::eval::jump` | 0 | 0.0% | Control flow jumps (throw, break, return) |
| `vm.c` | 5,451 | Not Started | `crate::vm` | 0 | 0.0% | Core virtual machine engine |
| `vm_args.c` | 1,226 | Not Started | `crate::vm::args` | 0 | 0.0% | Method argument passing |
| `vm_backtrace.c` | 2,406 | Not Started | `crate::vm::backtrace` | 0 | 0.0% | Backtrace generation |
| `vm_dump.c` | 1,651 | Not Started | `crate::vm::dump` | 0 | 0.0% | VM state dump utilities |
| `vm_eval.c` | 2,966 | Not Started | `crate::vm::eval` | 0 | 0.0% | Method dispatch and evaluation |
| `vm_exec.c` | 146 | Not Started | `crate::vm::exec` | 0 | 0.0% | VM loop execution |
| `vm_insnhelper.c` | 7,802 | Not Started | `crate::vm::insnhelper` | 0 | 0.0% | Instruction execution helpers |
| `vm_method.c` | 3,796 | Not Started | `crate::vm::method` | 0 | 0.0% | Method table management |
| `vm_sync.c` | 282 | Not Started | `crate::vm::sync` | 0 | 0.0% | VM synchronization primitives |
| `vm_trace.c` | 1,983 | Not Started | `crate::vm::trace` | 0 | 0.0% | TracePoint and event hooks |

### Memory & Garbage Collection
_Garbage collector, object allocator, and memory views_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `gc.c` | 7,115 | Not Started | `gc::mmtk` | 57 | 4.2% | MMTk support is upstream modular GC, not a port by this project |
| `imemo.c` | 726 | Not Started | `crate::imemo` | 0 | 0.0% | Internal memo objects |
| `memory_view.c` | 902 | Not Started | `crate::memory_view` | 0 | 0.0% | Memory view interface |
| `shape.c` | 1,716 | Not Started | `crate::shape` | 0 | 0.0% | Object shape / property layout tracking |

### Concurrency & Threads
_Threading, ractors, synchronization, and scheduling_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `concurrent_set.c` | 522 | Not Started | `crate::concurrent_set` | 0 | 0.0% | Lock-free concurrent set |
| `ractor.c` | 4,321 | Not Started | `crate::ractor` | 0 | 0.0% | Ractor actor model implementation |
| `ractor_sync.c` | 1,940 | Not Started | `crate::ractor::sync` | 0 | 0.0% | Ractor synchronization |
| `scheduler.c` | 1,395 | Not Started | `crate::scheduler` | 0 | 0.0% | Fiber scheduler interface |
| `signal.c` | 1,641 | Not Started | `crate::signal` | 0 | 0.0% | Signal handling |
| `thread.c` | 6,759 | Not Started | `crate::thread` | 0 | 0.0% | Thread management core |
| `thread_none.c` | 385 | Not Started | `crate::thread::none` | 0 | 0.0% | No-threads platform stubs |
| `thread_pthread.c` | 1,620 | Not Started | `crate::thread::pthread` | 0 | 0.0% | POSIX threads implementation |
| `thread_sched.c` | 2,686 | Not Started | `crate::thread::sched` | 0 | 0.0% | Thread scheduler |
| `thread_sched_mn.c` | 1,931 | Not Started | `crate::thread::sched_mn` | 0 | 0.0% | M:N thread scheduler |
| `thread_sync.c` | 1,547 | Not Started | `crate::thread::sync` | 0 | 0.0% | Thread synchronization primitives |
| `thread_win32.c` | 1,004 | Not Started | `crate::thread::win32` | 0 | 0.0% | Windows threads implementation |

### Parser & AST
_Syntax parser, AST nodes, and Prism integration_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `ast.c` | 1,280 | Not Started | `crate::parser::ast` | 0 | 0.0% | Ruby::AST module |
| `node.c` | 447 | Not Started | `crate::parser::node` | 0 | 0.0% | AST node construction |
| `node_dump.c` | 1,325 | Not Started | `crate::parser::node_dump` | 0 | 0.0% | AST dump utilities |
| `parser_st.c` | 173 | Not Started | `crate::parser::parser_st` | 0 | 0.0% | Parser symbol table |
| `prism_compile.c` | 11,336 | Not Started | `crate::parser::prism_compile` | 0 | 0.0% | Prism AST compiler |
| `prism_init.c` | 9 | Not Started | `crate::parser::prism_init` | 0 | 0.0% | Prism initialization |
| `ruby_parser.c` | 1,151 | Not Started | `crate::parser::ruby_parser` | 0 | 0.0% | Ruby parser driver |
| `universal_parser.c` | 215 | Not Started | `crate::parser::universal_parser` | 0 | 0.0% | Universal parser interface |

### IO & Filesystem
_Input/Output operations, files, directories, and processes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `dir.c` | 4,197 | Not Started | `crate::dir` | 0 | 0.0% | Directory operations |
| `file.c` | 9,503 | Not Started | `crate::file` | 0 | 0.0% | File system operations |
| `io.c` | 16,324 | Not Started | `crate::io` | 0 | 0.0% | IO class operations |
| `io_buffer.c` | 5,060 | Not Started | `crate::io::buffer` | 0 | 0.0% | IO::Buffer implementation |
| `pathname.c` | 457 | Not Started | `crate::pathname` | 0 | 0.0% | Pathname standard helper |
| `process.c` | 9,651 | Not Started | `crate::process` | 0 | 0.0% | Process management |
| `random.c` | 1,862 | Not Started | `crate::random` | 0 | 0.0% | Random number generation |

### Encoding & Regex
_String encodings, transcoding, and regular expressions_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `encoding.c` | 2,105 | Not Started | `crate::encoding` | 0 | 0.0% | String encoding support |
| `re.c` | 5,195 | In Progress | `core_rs::re` | 9 | 20.6% | Ported: rb_memsearch (all search algorithms), rb_memcicmp. Remaining: Regexp class interface |
| `regcomp.c` | 6,763 | Not Started | `crate::regcomp` | 0 | 0.0% | Onigmo regex compiler |
| `regenc.c` | 1,032 | Not Started | `crate::regenc` | 0 | 0.0% | Onigmo encoding engine |
| `regerror.c` | 408 | Not Started | `crate::regerror` | 0 | 0.0% | Onigmo regex error reporting |
| `regexec.c` | 5,376 | Not Started | `crate::regexec` | 0 | 0.0% | Onigmo regex execution engine |
| `regparse.c` | 6,855 | Not Started | `crate::regparse` | 0 | 0.0% | Onigmo regex parser |
| `regsyntax.c` | 388 | Not Started | `crate::regsyntax` | 0 | 0.0% | Onigmo syntax options |
| `transcode.c` | 4,709 | Not Started | `crate::transcode` | 0 | 0.0% | Character transcoding |

### Utilities & Support
_Internal utility functions, tables, and system helpers_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | 2,778 | Not Started | `crate::addr2line` | 0 | 0.0% | Address to source line resolution |
| `box.c` | 1,346 | Not Started | `crate::box` | 0 | 0.0% | Value boxing helpers |
| `builtin.c` | 145 | Not Started | `crate::builtin` | 0 | 0.0% | Builtin Ruby class loader |
| `debug.c` | 731 | Not Started | `crate::debug` | 0 | 0.0% | Debugging helpers |
| `debug_counter.c` | 150 | Not Started | `crate::debug_counter` | 0 | 0.0% | Performance debug counters |
| `dln.c` | 540 | Not Started | `crate::dln` | 0 | 0.0% | Dynamic linking loader |
| `dln_find.c` | 294 | Not Started | `crate::dln_find` | 0 | 0.0% | Dynamic loading path search |
| `error.c` | 4,406 | Not Started | `crate::error` | 0 | 0.0% | Exception and error handling |
| `id_table.c` | 573 | Not Started | `crate::id_table` | 0 | 0.0% | ID lookup table |
| `inits.c` | 116 | Not Started | `crate::inits` | 0 | 0.0% | Subsystem initializers |
| `load.c` | 1,828 | Not Started | `crate::load` | 0 | 0.0% | Require and load mechanism |
| `loadpath.c` | 91 | Not Started | `crate::loadpath` | 0 | 0.0% | LOAD_PATH initialization |
| `localeinit.c` | 137 | Not Started | `crate::localeinit` | 0 | 0.0% | Locale initialization |
| `main.c` | 63 | Not Started | `crate::main` | 0 | 0.0% | Main entry point |
| `marshal.c` | 2,733 | Not Started | `crate::marshal` | 0 | 0.0% | Marshal serialization |
| `math.c` | 1,212 | Not Started | `crate::math` | 0 | 0.0% | Math module functions |
| `mini_builtin.c` | 118 | Not Started | `crate::mini_builtin` | 0 | 0.0% | Miniruby builtin features |
| `miniinit.c` | 109 | Not Started | `crate::miniinit` | 0 | 0.0% | Miniruby initializers |
| `siphash.c` | 493 | Not Started | `crate::siphash` | 0 | 0.0% | SipHash hashing algorithm |
| `sprintf.c` | 1,283 | Not Started | `crate::sprintf` | 0 | 0.0% | Kernel#sprintf formatting |
| `st.c` | 3,396 | Not Started | `crate::st` | 0 | 0.0% | Symbol table / hash table internal implementation |
| `strftime.c` | 1,288 | Not Started | `crate::strftime` | 0 | 0.0% | Date/time formatting |
| `util.c` | 622 | In Progress | `core_rs::util` | 30 | 17.8% | Ported: ruby_scan_digits, ruby_scan_oct, ruby_scan_hex, ruby_strtoul, ruby_each_words. Remaining: qsort, getcwd, dtoa/strtod |
| `variable.c` | 4,725 | Not Started | `crate::variable` | 0 | 0.0% | Global and instance variable access |
| `version.c` | 306 | Not Started | `crate::version` | 0 | 0.0% | Ruby version constants |
| `vsnprintf.c` | 1,298 | Not Started | `crate::vsnprintf` | 0 | 0.0% | Portable vsnprintf |

### Platform & Miscellaneous
_Platform stubs, runner wrappers, and target-specific code_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `dmydln.c` | 31 | N/A | `N/A` | 0 | 0.0% | Dummy dynamic linking stub |
| `dmyenc.c` | 24 | N/A | `N/A` | 0 | 0.0% | Dummy encoding stub |
| `dmyext.c` | 18 | N/A | `N/A` | 0 | 0.0% | Dummy extension stub |
| `goruby.c` | 68 | N/A | `N/A` | 0 | 0.0% | Golf Ruby executable wrapper |
| `ruby-runner.c` | 104 | N/A | `N/A` | 0 | 0.0% | Development runner binary |
| `ruby.c` | 3,316 | Not Started | `crate::ruby_cli` | 0 | 0.0% | Main Ruby CLI argument handling |
| `rubystub.c` | 61 | N/A | `N/A` | 0 | 0.0% | Embedded stub |
| `sparc.c` | 40 | N/A | `N/A` | 0 | 0.0% | SPARC architecture assembly helper |

## Complete C Source File Ledger

| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | Utilities & Support | 2,778 | Not Started | `crate::addr2line` | 0 | 0.0% | Address to source line resolution |
| `array.c` | Core Data Structures | 9,137 | Not Started | `crate::array` | 0 | 0.0% | Array object implementation |
| `ast.c` | Parser & AST | 1,280 | Not Started | `crate::parser::ast` | 0 | 0.0% | Ruby::AST module |
| `bignum.c` | Core Data Structures | 7,350 | Not Started | `crate::bignum` | 0 | 0.0% | Big integer support |
| `box.c` | Utilities & Support | 1,346 | Not Started | `crate::box` | 0 | 0.0% | Value boxing helpers |
| `builtin.c` | Utilities & Support | 145 | Not Started | `crate::builtin` | 0 | 0.0% | Builtin Ruby class loader |
| `class.c` | Core Data Structures | 3,412 | Not Started | `crate::class` | 0 | 0.0% | Class and module hierarchy |
| `compar.c` | Core Data Structures | 355 | Not Started | `crate::compar` | 0 | 0.0% | Comparable mixin module |
| `compile.c` | JIT Compiler | 15,441 | Not Started | `crate::compile` | 0 | 0.0% | Bytecode compiler |
| `complex.c` | Core Data Structures | 2,839 | In Progress | `core_rs::complex` | 21 | 13.0% | Ported (complex-A-01, string scanner statics): issign, read_sign, isdecimal, read_rat_nos, read_rat, isimagunit, skip_ws and helpers. Remaining: Complex class methods |
| `concurrent_set.c` | Concurrency & Threads | 522 | Not Started | `crate::concurrent_set` | 0 | 0.0% | Lock-free concurrent set |
| `cont.c` | Virtual Machine & Execution | 3,908 | Not Started | `crate::cont` | 0 | 0.0% | Continuation and Fiber core |
| `debug.c` | Utilities & Support | 731 | Not Started | `crate::debug` | 0 | 0.0% | Debugging helpers |
| `debug_counter.c` | Utilities & Support | 150 | Not Started | `crate::debug_counter` | 0 | 0.0% | Performance debug counters |
| `dir.c` | IO & Filesystem | 4,197 | Not Started | `crate::dir` | 0 | 0.0% | Directory operations |
| `dln.c` | Utilities & Support | 540 | Not Started | `crate::dln` | 0 | 0.0% | Dynamic linking loader |
| `dln_find.c` | Utilities & Support | 294 | Not Started | `crate::dln_find` | 0 | 0.0% | Dynamic loading path search |
| `dmydln.c` | Platform & Miscellaneous | 31 | N/A | `N/A` | 0 | 0.0% | Dummy dynamic linking stub |
| `dmyenc.c` | Platform & Miscellaneous | 24 | N/A | `N/A` | 0 | 0.0% | Dummy encoding stub |
| `dmyext.c` | Platform & Miscellaneous | 18 | N/A | `N/A` | 0 | 0.0% | Dummy extension stub |
| `encoding.c` | Encoding & Regex | 2,105 | Not Started | `crate::encoding` | 0 | 0.0% | String encoding support |
| `enum.c` | Core Data Structures | 5,310 | Not Started | `crate::enum` | 0 | 0.0% | Enumerable module |
| `enumerator.c` | Core Data Structures | 4,794 | Not Started | `crate::enumerator` | 0 | 0.0% | Enumerator class |
| `error.c` | Utilities & Support | 4,406 | Not Started | `crate::error` | 0 | 0.0% | Exception and error handling |
| `eval.c` | Virtual Machine & Execution | 2,344 | Not Started | `crate::eval` | 0 | 0.0% | Top-level evaluation entry points |
| `eval_error.c` | Virtual Machine & Execution | 588 | Not Started | `crate::eval::error` | 0 | 0.0% | Evaluation error handling |
| `eval_jump.c` | Virtual Machine & Execution | 144 | Not Started | `crate::eval::jump` | 0 | 0.0% | Control flow jumps (throw, break, return) |
| `file.c` | IO & Filesystem | 9,503 | Not Started | `crate::file` | 0 | 0.0% | File system operations |
| `gc.c` | Memory & Garbage Collection | 7,115 | Not Started | `gc::mmtk` | 57 | 4.2% | MMTk support is upstream modular GC, not a port by this project |
| `goruby.c` | Platform & Miscellaneous | 68 | N/A | `N/A` | 0 | 0.0% | Golf Ruby executable wrapper |
| `hash.c` | Core Data Structures | 7,993 | Not Started | `crate::hash` | 0 | 0.0% | Hash map implementation |
| `id_table.c` | Utilities & Support | 573 | Not Started | `crate::id_table` | 0 | 0.0% | ID lookup table |
| `imemo.c` | Memory & Garbage Collection | 726 | Not Started | `crate::imemo` | 0 | 0.0% | Internal memo objects |
| `inits.c` | Utilities & Support | 116 | Not Started | `crate::inits` | 0 | 0.0% | Subsystem initializers |
| `io.c` | IO & Filesystem | 16,324 | Not Started | `crate::io` | 0 | 0.0% | IO class operations |
| `io_buffer.c` | IO & Filesystem | 5,060 | Not Started | `crate::io::buffer` | 0 | 0.0% | IO::Buffer implementation |
| `iseq.c` | JIT Compiler | 4,735 | Not Started | `crate::iseq` | 0 | 0.0% | Instruction sequences |
| `jit.c` | JIT Compiler | 965 | Not Started | `jit` | 9 | 65.8% | C helpers shared by upstream YJIT/ZJIT |
| `load.c` | Utilities & Support | 1,828 | Not Started | `crate::load` | 0 | 0.0% | Require and load mechanism |
| `loadpath.c` | Utilities & Support | 91 | Not Started | `crate::loadpath` | 0 | 0.0% | LOAD_PATH initialization |
| `localeinit.c` | Utilities & Support | 137 | Not Started | `crate::localeinit` | 0 | 0.0% | Locale initialization |
| `main.c` | Utilities & Support | 63 | Not Started | `crate::main` | 0 | 0.0% | Main entry point |
| `marshal.c` | Utilities & Support | 2,733 | Not Started | `crate::marshal` | 0 | 0.0% | Marshal serialization |
| `math.c` | Utilities & Support | 1,212 | Not Started | `crate::math` | 0 | 0.0% | Math module functions |
| `memory_view.c` | Memory & Garbage Collection | 902 | Not Started | `crate::memory_view` | 0 | 0.0% | Memory view interface |
| `mini_builtin.c` | Utilities & Support | 118 | Not Started | `crate::mini_builtin` | 0 | 0.0% | Miniruby builtin features |
| `miniinit.c` | Utilities & Support | 109 | Not Started | `crate::miniinit` | 0 | 0.0% | Miniruby initializers |
| `node.c` | Parser & AST | 447 | Not Started | `crate::parser::node` | 0 | 0.0% | AST node construction |
| `node_dump.c` | Parser & AST | 1,325 | Not Started | `crate::parser::node_dump` | 0 | 0.0% | AST dump utilities |
| `numeric.c` | Core Data Structures | 6,813 | Not Started | `crate::numeric` | 0 | 0.0% | Numeric base classes and operations |
| `object.c` | Core Data Structures | 4,741 | Not Started | `crate::object` | 0 | 0.0% | Object class methods and operations |
| `pack.c` | Core Data Structures | 1,936 | Not Started | `crate::pack` | 0 | 0.0% | Array#pack and String#unpack |
| `parser_st.c` | Parser & AST | 173 | Not Started | `crate::parser::parser_st` | 0 | 0.0% | Parser symbol table |
| `pathname.c` | IO & Filesystem | 457 | Not Started | `crate::pathname` | 0 | 0.0% | Pathname standard helper |
| `prism_compile.c` | Parser & AST | 11,336 | Not Started | `crate::parser::prism_compile` | 0 | 0.0% | Prism AST compiler |
| `prism_init.c` | Parser & AST | 9 | Not Started | `crate::parser::prism_init` | 0 | 0.0% | Prism initialization |
| `proc.c` | Core Data Structures | 5,543 | Not Started | `crate::proc` | 0 | 0.0% | Proc, Method, and Binding objects |
| `process.c` | IO & Filesystem | 9,651 | Not Started | `crate::process` | 0 | 0.0% | Process management |
| `ractor.c` | Concurrency & Threads | 4,321 | Not Started | `crate::ractor` | 0 | 0.0% | Ractor actor model implementation |
| `ractor_sync.c` | Concurrency & Threads | 1,940 | Not Started | `crate::ractor::sync` | 0 | 0.0% | Ractor synchronization |
| `random.c` | IO & Filesystem | 1,862 | Not Started | `crate::random` | 0 | 0.0% | Random number generation |
| `range.c` | Core Data Structures | 3,006 | Not Started | `crate::range` | 0 | 0.0% | Range object implementation |
| `rational.c` | Core Data Structures | 2,847 | Not Started | `crate::rational` | 0 | 0.0% | Rational number implementation |
| `re.c` | Encoding & Regex | 5,195 | In Progress | `core_rs::re` | 9 | 20.6% | Ported: rb_memsearch (all search algorithms), rb_memcicmp. Remaining: Regexp class interface |
| `regcomp.c` | Encoding & Regex | 6,763 | Not Started | `crate::regcomp` | 0 | 0.0% | Onigmo regex compiler |
| `regenc.c` | Encoding & Regex | 1,032 | Not Started | `crate::regenc` | 0 | 0.0% | Onigmo encoding engine |
| `regerror.c` | Encoding & Regex | 408 | Not Started | `crate::regerror` | 0 | 0.0% | Onigmo regex error reporting |
| `regexec.c` | Encoding & Regex | 5,376 | Not Started | `crate::regexec` | 0 | 0.0% | Onigmo regex execution engine |
| `regparse.c` | Encoding & Regex | 6,855 | Not Started | `crate::regparse` | 0 | 0.0% | Onigmo regex parser |
| `regsyntax.c` | Encoding & Regex | 388 | Not Started | `crate::regsyntax` | 0 | 0.0% | Onigmo syntax options |
| `ruby-runner.c` | Platform & Miscellaneous | 104 | N/A | `N/A` | 0 | 0.0% | Development runner binary |
| `ruby.c` | Platform & Miscellaneous | 3,316 | Not Started | `crate::ruby_cli` | 0 | 0.0% | Main Ruby CLI argument handling |
| `ruby_parser.c` | Parser & AST | 1,151 | Not Started | `crate::parser::ruby_parser` | 0 | 0.0% | Ruby parser driver |
| `rubystub.c` | Platform & Miscellaneous | 61 | N/A | `N/A` | 0 | 0.0% | Embedded stub |
| `scheduler.c` | Concurrency & Threads | 1,395 | Not Started | `crate::scheduler` | 0 | 0.0% | Fiber scheduler interface |
| `set.c` | Core Data Structures | 2,677 | Not Started | `crate::set` | 0 | 0.0% | Core Set class support |
| `shape.c` | Memory & Garbage Collection | 1,716 | Not Started | `crate::shape` | 0 | 0.0% | Object shape / property layout tracking |
| `signal.c` | Concurrency & Threads | 1,641 | Not Started | `crate::signal` | 0 | 0.0% | Signal handling |
| `siphash.c` | Utilities & Support | 493 | Not Started | `crate::siphash` | 0 | 0.0% | SipHash hashing algorithm |
| `sparc.c` | Platform & Miscellaneous | 40 | N/A | `N/A` | 0 | 0.0% | SPARC architecture assembly helper |
| `sprintf.c` | Utilities & Support | 1,283 | Not Started | `crate::sprintf` | 0 | 0.0% | Kernel#sprintf formatting |
| `st.c` | Utilities & Support | 3,396 | Not Started | `crate::st` | 0 | 0.0% | Symbol table / hash table internal implementation |
| `strftime.c` | Utilities & Support | 1,288 | Not Started | `crate::strftime` | 0 | 0.0% | Date/time formatting |
| `string.c` | Core Data Structures | 14,476 | Not Started | `crate::string` | 0 | 0.0% | String object implementation |
| `struct.c` | Core Data Structures | 2,352 | Not Started | `crate::struct` | 0 | 0.0% | Struct class implementation |
| `symbol.c` | Core Data Structures | 1,467 | Not Started | `crate::symbol` | 0 | 0.0% | Symbol management |
| `thread.c` | Concurrency & Threads | 6,759 | Not Started | `crate::thread` | 0 | 0.0% | Thread management core |
| `thread_none.c` | Concurrency & Threads | 385 | Not Started | `crate::thread::none` | 0 | 0.0% | No-threads platform stubs |
| `thread_pthread.c` | Concurrency & Threads | 1,620 | Not Started | `crate::thread::pthread` | 0 | 0.0% | POSIX threads implementation |
| `thread_sched.c` | Concurrency & Threads | 2,686 | Not Started | `crate::thread::sched` | 0 | 0.0% | Thread scheduler |
| `thread_sched_mn.c` | Concurrency & Threads | 1,931 | Not Started | `crate::thread::sched_mn` | 0 | 0.0% | M:N thread scheduler |
| `thread_sync.c` | Concurrency & Threads | 1,547 | Not Started | `crate::thread::sync` | 0 | 0.0% | Thread synchronization primitives |
| `thread_win32.c` | Concurrency & Threads | 1,004 | Not Started | `crate::thread::win32` | 0 | 0.0% | Windows threads implementation |
| `time.c` | Core Data Structures | 6,101 | Not Started | `crate::time` | 0 | 0.0% | Time class and operations |
| `transcode.c` | Encoding & Regex | 4,709 | Not Started | `crate::transcode` | 0 | 0.0% | Character transcoding |
| `universal_parser.c` | Parser & AST | 215 | Not Started | `crate::parser::universal_parser` | 0 | 0.0% | Universal parser interface |
| `util.c` | Utilities & Support | 622 | In Progress | `core_rs::util` | 30 | 17.8% | Ported: ruby_scan_digits, ruby_scan_oct, ruby_scan_hex, ruby_strtoul, ruby_each_words. Remaining: qsort, getcwd, dtoa/strtod |
| `variable.c` | Utilities & Support | 4,725 | Not Started | `crate::variable` | 0 | 0.0% | Global and instance variable access |
| `version.c` | Utilities & Support | 306 | Not Started | `crate::version` | 0 | 0.0% | Ruby version constants |
| `vm.c` | Virtual Machine & Execution | 5,451 | Not Started | `crate::vm` | 0 | 0.0% | Core virtual machine engine |
| `vm_args.c` | Virtual Machine & Execution | 1,226 | Not Started | `crate::vm::args` | 0 | 0.0% | Method argument passing |
| `vm_backtrace.c` | Virtual Machine & Execution | 2,406 | Not Started | `crate::vm::backtrace` | 0 | 0.0% | Backtrace generation |
| `vm_dump.c` | Virtual Machine & Execution | 1,651 | Not Started | `crate::vm::dump` | 0 | 0.0% | VM state dump utilities |
| `vm_eval.c` | Virtual Machine & Execution | 2,966 | Not Started | `crate::vm::eval` | 0 | 0.0% | Method dispatch and evaluation |
| `vm_exec.c` | Virtual Machine & Execution | 146 | Not Started | `crate::vm::exec` | 0 | 0.0% | VM loop execution |
| `vm_insnhelper.c` | Virtual Machine & Execution | 7,802 | Not Started | `crate::vm::insnhelper` | 0 | 0.0% | Instruction execution helpers |
| `vm_method.c` | Virtual Machine & Execution | 3,796 | Not Started | `crate::vm::method` | 0 | 0.0% | Method table management |
| `vm_sync.c` | Virtual Machine & Execution | 282 | Not Started | `crate::vm::sync` | 0 | 0.0% | VM synchronization primitives |
| `vm_trace.c` | Virtual Machine & Execution | 1,983 | Not Started | `crate::vm::trace` | 0 | 0.0% | TracePoint and event hooks |
| `vsnprintf.c` | Utilities & Support | 1,298 | Not Started | `crate::vsnprintf` | 0 | 0.0% | Portable vsnprintf |
| `weakmap.c` | Core Data Structures | 999 | Not Started | `crate::weakmap` | 0 | 0.0% | ObjectSpace::WeakMap |
| `yjit.c` | JIT Compiler | 603 | Not Started | `yjit` | 614 | 6.1% | yjit.c is the C side of upstream YJIT; YJIT itself is upstream Rust, not a port by this project |
| `zjit.c` | JIT Compiler | 406 | Not Started | `zjit` | 961 | 3.2% | zjit.c is the C side of upstream ZJIT; ZJIT itself is upstream Rust, not a port by this project |
