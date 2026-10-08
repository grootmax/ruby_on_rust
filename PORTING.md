# C-to-Rust Porting Ledger

Automated migration ledger tracking C source files, line counts, and Rust porting status.
Generated automatically by `ruby tool/generate_porting_ledger.rb`. Do not edit manually.

## Summary Statistics

| Metric | File Count | Lines of Code (LOC) | % of Total LOC |
| :--- | :--- | :--- | :--- |
| **Total C Source Files** | 113 | 317,474 | 100.0% |
| **Not Started** | 43 | 96,474 | 30.4% |
| **In Progress** | 1 | 2,839 | 0.9% |
| **Ported** | 0 | 0 | 0.0% |
| **Blocked** | 62 | 217,815 | 68.6% |
| **N/A** | 7 | 346 | 0.1% |

## Migration Progress by Subsystem

### JIT Compiler
_Just-In-Time compilers and execution machinery_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `compile.c` | 15,441 | Not Started | `crate::compile` | Bytecode compiler |
| `iseq.c` | 4,735 | Not Started | `crate::iseq` | Instruction sequences |
| `jit.c` | 965 | Not Started | `jit` | C helpers shared by upstream YJIT/ZJIT |
| `yjit.c` | 603 | Not Started | `yjit` | yjit.c is the C side of upstream YJIT; YJIT itself is upstream Rust, not a port by this project |
| `zjit.c` | 406 | Not Started | `zjit` | zjit.c is the C side of upstream ZJIT; ZJIT itself is upstream Rust, not a port by this project |

### Core Data Structures
_Built-in data types, objects, and core classes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `array.c` | 9,137 | Blocked | `crate::array` | Blocked: Verification build or test failure (unit array-A-01) |
| `bignum.c` | 7,350 | Blocked | `crate::bignum` | Blocked: Verification build or test failure (unit bignum-A-05) |
| `class.c` | 3,412 | Blocked | `crate::class` | Blocked: Verification build or test failure (unit class-A-01) |
| `compar.c` | 355 | Not Started | `crate::compar` | Comparable mixin module |
| `complex.c` | 2,839 | In Progress | `core_rs::complex` | Ported (complex-A-01, string scanner statics): issign, read_sign, isdecimal, read_rat_nos, read_rat, isimagunit, skip_ws and helpers. Remaining: Complex class methods |
| `enum.c` | 5,310 | Blocked | `crate::enum` | Blocked: Verification build or test failure (unit enum-A-01) |
| `enumerator.c` | 4,794 | Blocked | `crate::enumerator` | Blocked: Verification build or test failure (unit enumerator-A-01) |
| `hash.c` | 7,993 | Blocked | `crate::hash` | Blocked: Verification build or test failure (unit hash-A-02) |
| `numeric.c` | 6,813 | Blocked | `crate::numeric` | Blocked: Verification build or test failure (unit numeric-A-02) |
| `object.c` | 4,740 | Blocked | `crate::object` | Blocked: Verification build or test failure (unit object-A-01) |
| `pack.c` | 1,936 | Blocked | `crate::pack` | Blocked: Verification build or test failure (unit pack-A-01) |
| `proc.c` | 5,538 | Blocked | `crate::proc` | Blocked: Verification build or test failure (unit proc-A-01) |
| `range.c` | 3,006 | Blocked | `crate::range` | Blocked: Verification build or test failure (unit range-A-01) |
| `rational.c` | 2,847 | Blocked | `crate::rational` | Blocked: Verification build or test failure (unit rational-A-01) |
| `set.c` | 2,677 | Blocked | `crate::set` | Blocked: Verification build or test failure (unit set-A-01) |
| `string.c` | 14,476 | Blocked | `crate::string` | Blocked: Verification build or test failure (unit string-A-06) |
| `struct.c` | 2,352 | Blocked | `crate::struct` | Blocked: Verification build or test failure (unit struct-A-01) |
| `symbol.c` | 1,467 | Blocked | `crate::symbol` | Blocked: Verification build or test failure (unit symbol-A-01) |
| `time.c` | 6,101 | Blocked | `crate::time` | Blocked: Verification build or test failure (unit time-A-04) |
| `weakmap.c` | 999 | Blocked | `crate::weakmap` | Blocked: Verification build or test failure (unit weakmap-A-01) |

### Virtual Machine & Execution
_Interpreter VM loop, instruction helpers, and evaluation_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `cont.c` | 3,908 | Blocked | `crate::cont` | Blocked: Verification build or test failure (unit cont-A-04) |
| `eval.c` | 2,344 | Blocked | `crate::eval` | Blocked: Verification build or test failure (unit eval-A-01) |
| `eval_error.c` | 588 | Not Started | `crate::eval::error` | Evaluation error handling |
| `eval_jump.c` | 144 | Not Started | `crate::eval::jump` | Control flow jumps (throw, break, return) |
| `vm.c` | 5,420 | Not Started | `crate::vm` | Core virtual machine engine |
| `vm_args.c` | 1,219 | Not Started | `crate::vm::args` | Method argument passing |
| `vm_backtrace.c` | 2,406 | Not Started | `crate::vm::backtrace` | Backtrace generation |
| `vm_dump.c` | 1,660 | Not Started | `crate::vm::dump` | VM state dump utilities |
| `vm_eval.c` | 2,966 | Not Started | `crate::vm::eval` | Method dispatch and evaluation |
| `vm_exec.c` | 146 | Not Started | `crate::vm::exec` | VM loop execution |
| `vm_insnhelper.c` | 7,802 | Not Started | `crate::vm::insnhelper` | Instruction execution helpers |
| `vm_method.c` | 3,796 | Not Started | `crate::vm::method` | Method table management |
| `vm_sync.c` | 282 | Not Started | `crate::vm::sync` | VM synchronization primitives |
| `vm_trace.c` | 1,983 | Not Started | `crate::vm::trace` | TracePoint and event hooks |

### Memory & Garbage Collection
_Garbage collector, object allocator, and memory views_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `gc.c` | 7,106 | Not Started | `gc::mmtk` | MMTk support is upstream modular GC, not a port by this project |
| `imemo.c` | 726 | Blocked | `crate::imemo` | Blocked: Verification build or test failure (unit imemo-A-01) |
| `memory_view.c` | 902 | Blocked | `crate::memory_view` | Blocked: Verification build or test failure (unit memory_view-A-01) |
| `shape.c` | 1,716 | Blocked | `crate::shape` | Blocked: Verification build or test failure (unit shape-A-02) |

### Concurrency & Threads
_Threading, ractors, synchronization, and scheduling_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `concurrent_set.c` | 522 | Blocked | `crate::concurrent_set` | Blocked: Verification build or test failure (unit concurrent_set-A-01) |
| `ractor.c` | 4,321 | Blocked | `crate::ractor` | Blocked: Verification build or test failure (unit ractor-A-02) |
| `ractor_sync.c` | 1,940 | Blocked | `crate::ractor::sync` | Blocked: Verification build or test failure (unit ractor_sync-A-03) |
| `scheduler.c` | 1,395 | Blocked | `crate::scheduler` | Blocked: Verification build or test failure (unit scheduler-A-01) |
| `signal.c` | 1,646 | Blocked | `crate::signal` | Blocked: Verification build or test failure (unit signal-A-03) |
| `thread.c` | 6,759 | Blocked | `crate::thread` | Blocked: Verification build or test failure (unit thread-A-02) |
| `thread_none.c` | 385 | Blocked | `crate::thread::none` | Blocked: Verification build or test failure (unit thread_none-A-03) |
| `thread_pthread.c` | 1,648 | Blocked | `crate::thread::pthread` | Blocked: Verification build or test failure (unit thread_pthread-A-04) |
| `thread_sched.c` | 2,686 | Blocked | `crate::thread::sched` | Blocked: Verification build or test failure (unit thread_sched-A-01) |
| `thread_sched_mn.c` | 1,931 | Blocked | `crate::thread::sched_mn` | Blocked: Verification build or test failure (unit thread_sched_mn-A-02) |
| `thread_sync.c` | 1,547 | Blocked | `crate::thread::sync` | Blocked: Verification build or test failure (unit thread_sync-A-01) |
| `thread_win32.c` | 1,004 | Blocked | `crate::thread::win32` | Blocked: Verification build or test failure (unit thread_win32-A-03) |

### Parser & AST
_Syntax parser, AST nodes, and Prism integration_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `ast.c` | 1,280 | Not Started | `crate::parser::ast` | Ruby::AST module |
| `node.c` | 447 | Not Started | `crate::parser::node` | AST node construction |
| `node_dump.c` | 1,325 | Not Started | `crate::parser::node_dump` | AST dump utilities |
| `parser_st.c` | 173 | Not Started | `crate::parser::parser_st` | Parser symbol table |
| `prism_compile.c` | 11,349 | Not Started | `crate::parser::prism_compile` | Prism AST compiler |
| `prism_init.c` | 9 | Not Started | `crate::parser::prism_init` | Prism initialization |
| `ruby_parser.c` | 1,151 | Not Started | `crate::parser::ruby_parser` | Ruby parser driver |
| `universal_parser.c` | 215 | Not Started | `crate::parser::universal_parser` | Universal parser interface |

### IO & Filesystem
_Input/Output operations, files, directories, and processes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `dir.c` | 4,197 | Blocked | `crate::dir` | Blocked: Verification build or test failure (unit dir-A-03) |
| `file.c` | 9,477 | Blocked | `crate::file` | Blocked: Verification build or test failure (unit file-A-05) |
| `io.c` | 16,334 | Blocked | `crate::io` | Blocked: Verification build or test failure (unit io-A-04) |
| `io_buffer.c` | 5,060 | Blocked | `crate::io::buffer` | Blocked: Verification build or test failure (unit io_buffer-A-03) |
| `pathname.c` | 457 | Not Started | `crate::pathname` | Pathname standard helper |
| `process.c` | 9,651 | Blocked | `crate::process` | Blocked: Verification build or test failure (unit process-A-05) |
| `random.c` | 1,862 | Blocked | `crate::random` | Blocked: Verification build or test failure (unit random-A-02) |

### Encoding & Regex
_String encodings, transcoding, and regular expressions_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `encoding.c` | 2,105 | Blocked | `crate::encoding` | Blocked: Verification build or test failure (unit encoding-A-04) |
| `re.c` | 5,195 | Blocked | `core_rs::re` | Blocked: Verification build or test failure (unit re-A-02) |
| `regcomp.c` | 6,763 | Not Started | `crate::regcomp` | Onigmo regex compiler |
| `regenc.c` | 1,032 | Not Started | `crate::regenc` | Onigmo encoding engine |
| `regerror.c` | 408 | Not Started | `crate::regerror` | Onigmo regex error reporting |
| `regexec.c` | 5,376 | Not Started | `crate::regexec` | Onigmo regex execution engine |
| `regparse.c` | 6,855 | Not Started | `crate::regparse` | Onigmo regex parser |
| `regsyntax.c` | 388 | Not Started | `crate::regsyntax` | Onigmo syntax options |
| `transcode.c` | 4,709 | Blocked | `crate::transcode` | Blocked: Verification build or test failure (unit transcode-A-02) |

### Utilities & Support
_Internal utility functions, tables, and system helpers_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | 2,778 | Blocked | `crate::addr2line` | Blocked: Verification build or test failure (unit addr2line-A-08) |
| `box.c` | 1,346 | Blocked | `crate::box` | Blocked: Verification build or test failure (unit box-A-02) |
| `builtin.c` | 145 | Not Started | `crate::builtin` | Builtin Ruby class loader |
| `debug.c` | 731 | Blocked | `crate::debug` | Blocked: Verification build or test failure (unit debug-A-02) |
| `debug_counter.c` | 150 | Not Started | `crate::debug_counter` | Performance debug counters |
| `dln.c` | 540 | Blocked | `crate::dln` | Blocked: Verification build or test failure (unit dln-A-01) |
| `dln_find.c` | 294 | Not Started | `crate::dln_find` | Dynamic loading path search |
| `error.c` | 4,422 | Blocked | `crate::error` | Blocked: Verification build or test failure (unit error-A-02) |
| `id_table.c` | 573 | Blocked | `crate::id_table` | Blocked: Verification build or test failure (unit id_table-A-01) |
| `inits.c` | 116 | Not Started | `crate::inits` | Subsystem initializers |
| `load.c` | 1,828 | Blocked | `crate::load` | Blocked: Verification build or test failure (unit load-A-01) |
| `loadpath.c` | 91 | Not Started | `crate::loadpath` | LOAD_PATH initialization |
| `localeinit.c` | 137 | Not Started | `crate::localeinit` | Locale initialization |
| `main.c` | 63 | Not Started | `crate::main` | Main entry point |
| `marshal.c` | 2,733 | Blocked | `crate::marshal` | Blocked: Verification build or test failure (unit marshal-A-02) |
| `math.c` | 1,212 | Blocked | `crate::math` | Blocked: Verification build or test failure (unit math-A-01) |
| `mini_builtin.c` | 118 | Not Started | `crate::mini_builtin` | Miniruby builtin features |
| `miniinit.c` | 109 | Not Started | `crate::miniinit` | Miniruby initializers |
| `siphash.c` | 493 | Blocked | `crate::siphash` | Blocked: Verification build or test failure (unit siphash-A-03) |
| `sprintf.c` | 1,283 | Blocked | `crate::sprintf` | Blocked: Verification build or test failure (unit sprintf-A-01) |
| `st.c` | 3,405 | Blocked | `crate::st` | Blocked: Verification build or test failure (unit st-A-11) |
| `strftime.c` | 1,286 | Blocked | `crate::strftime` | Blocked: Verification build or test failure (unit strftime-A-01) |
| `util.c` | 622 | Blocked | `core_rs::util` | Blocked: Verification build or test failure (unit util-A-01) |
| `variable.c` | 4,725 | Blocked | `crate::variable` | Blocked: Verification build or test failure (unit variable-A-02) |
| `version.c` | 306 | Blocked | `crate::version` | Blocked: Verification build or test failure (unit version-A-01) |
| `vsnprintf.c` | 1,298 | Blocked | `crate::vsnprintf` | Blocked: Verification build or test failure (unit vsnprintf-A-01) |

### Platform & Miscellaneous
_Platform stubs, runner wrappers, and target-specific code_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `dmydln.c` | 31 | N/A | `N/A` | Dummy dynamic linking stub |
| `dmyenc.c` | 24 | N/A | `N/A` | Dummy encoding stub |
| `dmyext.c` | 18 | N/A | `N/A` | Dummy extension stub |
| `goruby.c` | 68 | N/A | `N/A` | Golf Ruby executable wrapper |
| `ruby-runner.c` | 104 | N/A | `N/A` | Development runner binary |
| `ruby.c` | 3,316 | Blocked | `crate::ruby_cli` | Blocked: Verification build or test failure (unit ruby-A-02) |
| `rubystub.c` | 61 | N/A | `N/A` | Embedded stub |
| `sparc.c` | 40 | N/A | `N/A` | SPARC architecture assembly helper |

## Complete C Source File Ledger

| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | Utilities & Support | 2,778 | Blocked | `crate::addr2line` | Blocked: Verification build or test failure (unit addr2line-A-08) |
| `array.c` | Core Data Structures | 9,137 | Blocked | `crate::array` | Blocked: Verification build or test failure (unit array-A-01) |
| `ast.c` | Parser & AST | 1,280 | Not Started | `crate::parser::ast` | Ruby::AST module |
| `bignum.c` | Core Data Structures | 7,350 | Blocked | `crate::bignum` | Blocked: Verification build or test failure (unit bignum-A-05) |
| `box.c` | Utilities & Support | 1,346 | Blocked | `crate::box` | Blocked: Verification build or test failure (unit box-A-02) |
| `builtin.c` | Utilities & Support | 145 | Not Started | `crate::builtin` | Builtin Ruby class loader |
| `class.c` | Core Data Structures | 3,412 | Blocked | `crate::class` | Blocked: Verification build or test failure (unit class-A-01) |
| `compar.c` | Core Data Structures | 355 | Not Started | `crate::compar` | Comparable mixin module |
| `compile.c` | JIT Compiler | 15,441 | Not Started | `crate::compile` | Bytecode compiler |
| `complex.c` | Core Data Structures | 2,839 | In Progress | `core_rs::complex` | Ported (complex-A-01, string scanner statics): issign, read_sign, isdecimal, read_rat_nos, read_rat, isimagunit, skip_ws and helpers. Remaining: Complex class methods |
| `concurrent_set.c` | Concurrency & Threads | 522 | Blocked | `crate::concurrent_set` | Blocked: Verification build or test failure (unit concurrent_set-A-01) |
| `cont.c` | Virtual Machine & Execution | 3,908 | Blocked | `crate::cont` | Blocked: Verification build or test failure (unit cont-A-04) |
| `debug.c` | Utilities & Support | 731 | Blocked | `crate::debug` | Blocked: Verification build or test failure (unit debug-A-02) |
| `debug_counter.c` | Utilities & Support | 150 | Not Started | `crate::debug_counter` | Performance debug counters |
| `dir.c` | IO & Filesystem | 4,197 | Blocked | `crate::dir` | Blocked: Verification build or test failure (unit dir-A-03) |
| `dln.c` | Utilities & Support | 540 | Blocked | `crate::dln` | Blocked: Verification build or test failure (unit dln-A-01) |
| `dln_find.c` | Utilities & Support | 294 | Not Started | `crate::dln_find` | Dynamic loading path search |
| `dmydln.c` | Platform & Miscellaneous | 31 | N/A | `N/A` | Dummy dynamic linking stub |
| `dmyenc.c` | Platform & Miscellaneous | 24 | N/A | `N/A` | Dummy encoding stub |
| `dmyext.c` | Platform & Miscellaneous | 18 | N/A | `N/A` | Dummy extension stub |
| `encoding.c` | Encoding & Regex | 2,105 | Blocked | `crate::encoding` | Blocked: Verification build or test failure (unit encoding-A-04) |
| `enum.c` | Core Data Structures | 5,310 | Blocked | `crate::enum` | Blocked: Verification build or test failure (unit enum-A-01) |
| `enumerator.c` | Core Data Structures | 4,794 | Blocked | `crate::enumerator` | Blocked: Verification build or test failure (unit enumerator-A-01) |
| `error.c` | Utilities & Support | 4,422 | Blocked | `crate::error` | Blocked: Verification build or test failure (unit error-A-02) |
| `eval.c` | Virtual Machine & Execution | 2,344 | Blocked | `crate::eval` | Blocked: Verification build or test failure (unit eval-A-01) |
| `eval_error.c` | Virtual Machine & Execution | 588 | Not Started | `crate::eval::error` | Evaluation error handling |
| `eval_jump.c` | Virtual Machine & Execution | 144 | Not Started | `crate::eval::jump` | Control flow jumps (throw, break, return) |
| `file.c` | IO & Filesystem | 9,477 | Blocked | `crate::file` | Blocked: Verification build or test failure (unit file-A-05) |
| `gc.c` | Memory & Garbage Collection | 7,106 | Not Started | `gc::mmtk` | MMTk support is upstream modular GC, not a port by this project |
| `goruby.c` | Platform & Miscellaneous | 68 | N/A | `N/A` | Golf Ruby executable wrapper |
| `hash.c` | Core Data Structures | 7,993 | Blocked | `crate::hash` | Blocked: Verification build or test failure (unit hash-A-02) |
| `id_table.c` | Utilities & Support | 573 | Blocked | `crate::id_table` | Blocked: Verification build or test failure (unit id_table-A-01) |
| `imemo.c` | Memory & Garbage Collection | 726 | Blocked | `crate::imemo` | Blocked: Verification build or test failure (unit imemo-A-01) |
| `inits.c` | Utilities & Support | 116 | Not Started | `crate::inits` | Subsystem initializers |
| `io.c` | IO & Filesystem | 16,334 | Blocked | `crate::io` | Blocked: Verification build or test failure (unit io-A-04) |
| `io_buffer.c` | IO & Filesystem | 5,060 | Blocked | `crate::io::buffer` | Blocked: Verification build or test failure (unit io_buffer-A-03) |
| `iseq.c` | JIT Compiler | 4,735 | Not Started | `crate::iseq` | Instruction sequences |
| `jit.c` | JIT Compiler | 965 | Not Started | `jit` | C helpers shared by upstream YJIT/ZJIT |
| `load.c` | Utilities & Support | 1,828 | Blocked | `crate::load` | Blocked: Verification build or test failure (unit load-A-01) |
| `loadpath.c` | Utilities & Support | 91 | Not Started | `crate::loadpath` | LOAD_PATH initialization |
| `localeinit.c` | Utilities & Support | 137 | Not Started | `crate::localeinit` | Locale initialization |
| `main.c` | Utilities & Support | 63 | Not Started | `crate::main` | Main entry point |
| `marshal.c` | Utilities & Support | 2,733 | Blocked | `crate::marshal` | Blocked: Verification build or test failure (unit marshal-A-02) |
| `math.c` | Utilities & Support | 1,212 | Blocked | `crate::math` | Blocked: Verification build or test failure (unit math-A-01) |
| `memory_view.c` | Memory & Garbage Collection | 902 | Blocked | `crate::memory_view` | Blocked: Verification build or test failure (unit memory_view-A-01) |
| `mini_builtin.c` | Utilities & Support | 118 | Not Started | `crate::mini_builtin` | Miniruby builtin features |
| `miniinit.c` | Utilities & Support | 109 | Not Started | `crate::miniinit` | Miniruby initializers |
| `node.c` | Parser & AST | 447 | Not Started | `crate::parser::node` | AST node construction |
| `node_dump.c` | Parser & AST | 1,325 | Not Started | `crate::parser::node_dump` | AST dump utilities |
| `numeric.c` | Core Data Structures | 6,813 | Blocked | `crate::numeric` | Blocked: Verification build or test failure (unit numeric-A-02) |
| `object.c` | Core Data Structures | 4,740 | Blocked | `crate::object` | Blocked: Verification build or test failure (unit object-A-01) |
| `pack.c` | Core Data Structures | 1,936 | Blocked | `crate::pack` | Blocked: Verification build or test failure (unit pack-A-01) |
| `parser_st.c` | Parser & AST | 173 | Not Started | `crate::parser::parser_st` | Parser symbol table |
| `pathname.c` | IO & Filesystem | 457 | Not Started | `crate::pathname` | Pathname standard helper |
| `prism_compile.c` | Parser & AST | 11,349 | Not Started | `crate::parser::prism_compile` | Prism AST compiler |
| `prism_init.c` | Parser & AST | 9 | Not Started | `crate::parser::prism_init` | Prism initialization |
| `proc.c` | Core Data Structures | 5,538 | Blocked | `crate::proc` | Blocked: Verification build or test failure (unit proc-A-01) |
| `process.c` | IO & Filesystem | 9,651 | Blocked | `crate::process` | Blocked: Verification build or test failure (unit process-A-05) |
| `ractor.c` | Concurrency & Threads | 4,321 | Blocked | `crate::ractor` | Blocked: Verification build or test failure (unit ractor-A-02) |
| `ractor_sync.c` | Concurrency & Threads | 1,940 | Blocked | `crate::ractor::sync` | Blocked: Verification build or test failure (unit ractor_sync-A-03) |
| `random.c` | IO & Filesystem | 1,862 | Blocked | `crate::random` | Blocked: Verification build or test failure (unit random-A-02) |
| `range.c` | Core Data Structures | 3,006 | Blocked | `crate::range` | Blocked: Verification build or test failure (unit range-A-01) |
| `rational.c` | Core Data Structures | 2,847 | Blocked | `crate::rational` | Blocked: Verification build or test failure (unit rational-A-01) |
| `re.c` | Encoding & Regex | 5,195 | Blocked | `core_rs::re` | Blocked: Verification build or test failure (unit re-A-02) |
| `regcomp.c` | Encoding & Regex | 6,763 | Not Started | `crate::regcomp` | Onigmo regex compiler |
| `regenc.c` | Encoding & Regex | 1,032 | Not Started | `crate::regenc` | Onigmo encoding engine |
| `regerror.c` | Encoding & Regex | 408 | Not Started | `crate::regerror` | Onigmo regex error reporting |
| `regexec.c` | Encoding & Regex | 5,376 | Not Started | `crate::regexec` | Onigmo regex execution engine |
| `regparse.c` | Encoding & Regex | 6,855 | Not Started | `crate::regparse` | Onigmo regex parser |
| `regsyntax.c` | Encoding & Regex | 388 | Not Started | `crate::regsyntax` | Onigmo syntax options |
| `ruby-runner.c` | Platform & Miscellaneous | 104 | N/A | `N/A` | Development runner binary |
| `ruby.c` | Platform & Miscellaneous | 3,316 | Blocked | `crate::ruby_cli` | Blocked: Verification build or test failure (unit ruby-A-02) |
| `ruby_parser.c` | Parser & AST | 1,151 | Not Started | `crate::parser::ruby_parser` | Ruby parser driver |
| `rubystub.c` | Platform & Miscellaneous | 61 | N/A | `N/A` | Embedded stub |
| `scheduler.c` | Concurrency & Threads | 1,395 | Blocked | `crate::scheduler` | Blocked: Verification build or test failure (unit scheduler-A-01) |
| `set.c` | Core Data Structures | 2,677 | Blocked | `crate::set` | Blocked: Verification build or test failure (unit set-A-01) |
| `shape.c` | Memory & Garbage Collection | 1,716 | Blocked | `crate::shape` | Blocked: Verification build or test failure (unit shape-A-02) |
| `signal.c` | Concurrency & Threads | 1,646 | Blocked | `crate::signal` | Blocked: Verification build or test failure (unit signal-A-03) |
| `siphash.c` | Utilities & Support | 493 | Blocked | `crate::siphash` | Blocked: Verification build or test failure (unit siphash-A-03) |
| `sparc.c` | Platform & Miscellaneous | 40 | N/A | `N/A` | SPARC architecture assembly helper |
| `sprintf.c` | Utilities & Support | 1,283 | Blocked | `crate::sprintf` | Blocked: Verification build or test failure (unit sprintf-A-01) |
| `st.c` | Utilities & Support | 3,405 | Blocked | `crate::st` | Blocked: Verification build or test failure (unit st-A-11) |
| `strftime.c` | Utilities & Support | 1,286 | Blocked | `crate::strftime` | Blocked: Verification build or test failure (unit strftime-A-01) |
| `string.c` | Core Data Structures | 14,476 | Blocked | `crate::string` | Blocked: Verification build or test failure (unit string-A-06) |
| `struct.c` | Core Data Structures | 2,352 | Blocked | `crate::struct` | Blocked: Verification build or test failure (unit struct-A-01) |
| `symbol.c` | Core Data Structures | 1,467 | Blocked | `crate::symbol` | Blocked: Verification build or test failure (unit symbol-A-01) |
| `thread.c` | Concurrency & Threads | 6,759 | Blocked | `crate::thread` | Blocked: Verification build or test failure (unit thread-A-02) |
| `thread_none.c` | Concurrency & Threads | 385 | Blocked | `crate::thread::none` | Blocked: Verification build or test failure (unit thread_none-A-03) |
| `thread_pthread.c` | Concurrency & Threads | 1,648 | Blocked | `crate::thread::pthread` | Blocked: Verification build or test failure (unit thread_pthread-A-04) |
| `thread_sched.c` | Concurrency & Threads | 2,686 | Blocked | `crate::thread::sched` | Blocked: Verification build or test failure (unit thread_sched-A-01) |
| `thread_sched_mn.c` | Concurrency & Threads | 1,931 | Blocked | `crate::thread::sched_mn` | Blocked: Verification build or test failure (unit thread_sched_mn-A-02) |
| `thread_sync.c` | Concurrency & Threads | 1,547 | Blocked | `crate::thread::sync` | Blocked: Verification build or test failure (unit thread_sync-A-01) |
| `thread_win32.c` | Concurrency & Threads | 1,004 | Blocked | `crate::thread::win32` | Blocked: Verification build or test failure (unit thread_win32-A-03) |
| `time.c` | Core Data Structures | 6,101 | Blocked | `crate::time` | Blocked: Verification build or test failure (unit time-A-04) |
| `transcode.c` | Encoding & Regex | 4,709 | Blocked | `crate::transcode` | Blocked: Verification build or test failure (unit transcode-A-02) |
| `universal_parser.c` | Parser & AST | 215 | Not Started | `crate::parser::universal_parser` | Universal parser interface |
| `util.c` | Utilities & Support | 622 | Blocked | `core_rs::util` | Blocked: Verification build or test failure (unit util-A-01) |
| `variable.c` | Utilities & Support | 4,725 | Blocked | `crate::variable` | Blocked: Verification build or test failure (unit variable-A-02) |
| `version.c` | Utilities & Support | 306 | Blocked | `crate::version` | Blocked: Verification build or test failure (unit version-A-01) |
| `vm.c` | Virtual Machine & Execution | 5,420 | Not Started | `crate::vm` | Core virtual machine engine |
| `vm_args.c` | Virtual Machine & Execution | 1,219 | Not Started | `crate::vm::args` | Method argument passing |
| `vm_backtrace.c` | Virtual Machine & Execution | 2,406 | Not Started | `crate::vm::backtrace` | Backtrace generation |
| `vm_dump.c` | Virtual Machine & Execution | 1,660 | Not Started | `crate::vm::dump` | VM state dump utilities |
| `vm_eval.c` | Virtual Machine & Execution | 2,966 | Not Started | `crate::vm::eval` | Method dispatch and evaluation |
| `vm_exec.c` | Virtual Machine & Execution | 146 | Not Started | `crate::vm::exec` | VM loop execution |
| `vm_insnhelper.c` | Virtual Machine & Execution | 7,802 | Not Started | `crate::vm::insnhelper` | Instruction execution helpers |
| `vm_method.c` | Virtual Machine & Execution | 3,796 | Not Started | `crate::vm::method` | Method table management |
| `vm_sync.c` | Virtual Machine & Execution | 282 | Not Started | `crate::vm::sync` | VM synchronization primitives |
| `vm_trace.c` | Virtual Machine & Execution | 1,983 | Not Started | `crate::vm::trace` | TracePoint and event hooks |
| `vsnprintf.c` | Utilities & Support | 1,298 | Blocked | `crate::vsnprintf` | Blocked: Verification build or test failure (unit vsnprintf-A-01) |
| `weakmap.c` | Core Data Structures | 999 | Blocked | `crate::weakmap` | Blocked: Verification build or test failure (unit weakmap-A-01) |
| `yjit.c` | JIT Compiler | 603 | Not Started | `yjit` | yjit.c is the C side of upstream YJIT; YJIT itself is upstream Rust, not a port by this project |
| `zjit.c` | JIT Compiler | 406 | Not Started | `zjit` | zjit.c is the C side of upstream ZJIT; ZJIT itself is upstream Rust, not a port by this project |
