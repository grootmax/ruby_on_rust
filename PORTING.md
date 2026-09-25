# C-to-Rust Porting Ledger

Automated migration ledger tracking C source files, line counts, and Rust porting status.
Generated automatically by `ruby tool/generate_porting_ledger.rb`. Do not edit manually.

## Summary Statistics

| Metric | File Count | Lines of Code (LOC) | % of Total LOC |
| :--- | :--- | :--- | :--- |
| **Total C Source Files** | 113 | 315,617 | 100.0% |
| **Not Started** | 101 | 303,478 | 96.2% |
| **In Progress** | 4 | 11,267 | 3.6% |
| **Ported** | 1 | 526 | 0.2% |
| **Blocked** | 0 | 0 | 0.0% |
| **N/A** | 7 | 346 | 0.1% |

## Migration Progress by Subsystem

### JIT Compiler
_Just-In-Time compilers and execution machinery_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `compile.c` | 15,387 | Not Started | `crate::compile` | Bytecode compiler |
| `iseq.c` | 4,740 | Not Started | `crate::iseq` | Instruction sequences |
| `jit.c` | 921 | In Progress | `jit` | JIT common interface layer |
| `yjit.c` | 526 | Ported | `yjit` | YJIT engine implemented in Rust |
| `zjit.c` | 389 | In Progress | `zjit` | ZJIT compiler framework in Rust |

### Core Data Structures
_Built-in data types, objects, and core classes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `array.c` | 9,119 | Not Started | `crate::array` | Array object implementation |
| `bignum.c` | 7,350 | Not Started | `crate::bignum` | Big integer support |
| `class.c` | 3,404 | Not Started | `crate::class` | Class and module hierarchy |
| `compar.c` | 355 | Not Started | `crate::compar` | Comparable mixin module |
| `complex.c` | 2,822 | Not Started | `crate::complex` | Complex number class |
| `enum.c` | 5,297 | Not Started | `crate::enum` | Enumerable module |
| `enumerator.c` | 4,802 | Not Started | `crate::enumerator` | Enumerator class |
| `hash.c` | 8,045 | Not Started | `crate::hash` | Hash map implementation |
| `numeric.c` | 6,817 | Not Started | `crate::numeric` | Numeric base classes and operations |
| `object.c` | 4,739 | Not Started | `crate::object` | Object class methods and operations |
| `pack.c` | 1,932 | Not Started | `crate::pack` | Array#pack and String#unpack |
| `proc.c` | 5,537 | Not Started | `crate::proc` | Proc, Method, and Binding objects |
| `range.c` | 3,006 | Not Started | `crate::range` | Range object implementation |
| `rational.c` | 2,847 | Not Started | `crate::rational` | Rational number implementation |
| `set.c` | 2,663 | Not Started | `crate::set` | Core Set class support |
| `string.c` | 14,460 | Not Started | `crate::string` | String object implementation |
| `struct.c` | 2,322 | Not Started | `crate::struct` | Struct class implementation |
| `symbol.c` | 1,467 | Not Started | `crate::symbol` | Symbol management |
| `time.c` | 6,101 | Not Started | `crate::time` | Time class and operations |
| `weakmap.c` | 999 | Not Started | `crate::weakmap` | ObjectSpace::WeakMap |

### Virtual Machine & Execution
_Interpreter VM loop, instruction helpers, and evaluation_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `cont.c` | 3,908 | Not Started | `crate::cont` | Continuation and Fiber core |
| `eval.c` | 2,344 | Not Started | `crate::eval` | Top-level evaluation entry points |
| `eval_error.c` | 588 | Not Started | `crate::eval::error` | Evaluation error handling |
| `eval_jump.c` | 144 | Not Started | `crate::eval::jump` | Control flow jumps (throw, break, return) |
| `vm.c` | 5,408 | Not Started | `crate::vm` | Core virtual machine engine |
| `vm_args.c` | 1,219 | Not Started | `crate::vm::args` | Method argument passing |
| `vm_backtrace.c` | 2,406 | Not Started | `crate::vm::backtrace` | Backtrace generation |
| `vm_dump.c` | 1,656 | Not Started | `crate::vm::dump` | VM state dump utilities |
| `vm_eval.c` | 2,981 | Not Started | `crate::vm::eval` | Method dispatch and evaluation |
| `vm_exec.c` | 154 | Not Started | `crate::vm::exec` | VM loop execution |
| `vm_insnhelper.c` | 7,797 | Not Started | `crate::vm::insnhelper` | Instruction execution helpers |
| `vm_method.c` | 3,786 | Not Started | `crate::vm::method` | Method table management |
| `vm_sync.c` | 282 | Not Started | `crate::vm::sync` | VM synchronization primitives |
| `vm_trace.c` | 1,971 | Not Started | `crate::vm::trace` | TracePoint and event hooks |

### Memory & Garbage Collection
_Garbage collector, object allocator, and memory views_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `gc.c` | 6,642 | In Progress | `gc::mmtk` | MMTk and GC integration |
| `imemo.c` | 726 | Not Started | `crate::imemo` | Internal memo objects |
| `memory_view.c` | 902 | Not Started | `crate::memory_view` | Memory view interface |
| `shape.c` | 1,716 | Not Started | `crate::shape` | Object shape / property layout tracking |

### Concurrency & Threads
_Threading, ractors, synchronization, and scheduling_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `concurrent_set.c` | 522 | Not Started | `crate::concurrent_set` | Lock-free concurrent set |
| `ractor.c` | 4,242 | Not Started | `crate::ractor` | Ractor actor model implementation |
| `ractor_sync.c` | 1,922 | Not Started | `crate::ractor::sync` | Ractor synchronization |
| `scheduler.c` | 1,395 | Not Started | `crate::scheduler` | Fiber scheduler interface |
| `signal.c` | 1,646 | Not Started | `crate::signal` | Signal handling |
| `thread.c` | 6,757 | Not Started | `crate::thread` | Thread management core |
| `thread_none.c` | 384 | Not Started | `crate::thread::none` | No-threads platform stubs |
| `thread_pthread.c` | 1,648 | Not Started | `crate::thread::pthread` | POSIX threads implementation |
| `thread_sched.c` | 2,686 | Not Started | `crate::thread::sched` | Thread scheduler |
| `thread_sched_mn.c` | 1,931 | Not Started | `crate::thread::sched_mn` | M:N thread scheduler |
| `thread_sync.c` | 1,545 | Not Started | `crate::thread::sync` | Thread synchronization primitives |
| `thread_win32.c` | 980 | Not Started | `crate::thread::win32` | Windows threads implementation |

### Parser & AST
_Syntax parser, AST nodes, and Prism integration_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `ast.c` | 1,280 | Not Started | `crate::parser::ast` | Ruby::AST module |
| `node.c` | 447 | Not Started | `crate::parser::node` | AST node construction |
| `node_dump.c` | 1,325 | Not Started | `crate::parser::node_dump` | AST dump utilities |
| `parser_st.c` | 171 | Not Started | `crate::parser::parser_st` | Parser symbol table |
| `prism_compile.c` | 11,269 | Not Started | `crate::parser::prism_compile` | Prism AST compiler |
| `prism_init.c` | 9 | Not Started | `crate::parser::prism_init` | Prism initialization |
| `ruby_parser.c` | 1,151 | Not Started | `crate::parser::ruby_parser` | Ruby parser driver |
| `universal_parser.c` | 215 | Not Started | `crate::parser::universal_parser` | Universal parser interface |

### IO & Filesystem
_Input/Output operations, files, directories, and processes_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `dir.c` | 4,153 | Not Started | `crate::dir` | Directory operations |
| `file.c` | 9,237 | Not Started | `crate::file` | File system operations |
| `io.c` | 16,196 | Not Started | `crate::io` | IO class operations |
| `io_buffer.c` | 4,604 | Not Started | `crate::io::buffer` | IO::Buffer implementation |
| `pathname.c` | 457 | Not Started | `crate::pathname` | Pathname standard helper |
| `process.c` | 9,698 | Not Started | `crate::process` | Process management |
| `random.c` | 1,862 | Not Started | `crate::random` | Random number generation |

### Encoding & Regex
_String encodings, transcoding, and regular expressions_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `encoding.c` | 2,105 | Not Started | `crate::encoding` | String encoding support |
| `re.c` | 5,186 | Not Started | `crate::re` | Regexp class interface |
| `regcomp.c` | 6,755 | Not Started | `crate::regcomp` | Onigmo regex compiler |
| `regenc.c` | 1,032 | Not Started | `crate::regenc` | Onigmo encoding engine |
| `regerror.c` | 406 | Not Started | `crate::regerror` | Onigmo regex error reporting |
| `regexec.c` | 5,376 | Not Started | `crate::regexec` | Onigmo regex execution engine |
| `regparse.c` | 6,853 | Not Started | `crate::regparse` | Onigmo regex parser |
| `regsyntax.c` | 388 | Not Started | `crate::regsyntax` | Onigmo syntax options |
| `transcode.c` | 4,709 | Not Started | `crate::transcode` | Character transcoding |

### Utilities & Support
_Internal utility functions, tables, and system helpers_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | 2,773 | Not Started | `crate::addr2line` | Address to source line resolution |
| `box.c` | 1,346 | Not Started | `crate::box` | Value boxing helpers |
| `builtin.c` | 145 | Not Started | `crate::builtin` | Builtin Ruby class loader |
| `debug.c` | 729 | Not Started | `crate::debug` | Debugging helpers |
| `debug_counter.c` | 150 | Not Started | `crate::debug_counter` | Performance debug counters |
| `dln.c` | 567 | Not Started | `crate::dln` | Dynamic linking loader |
| `dln_find.c` | 293 | Not Started | `crate::dln_find` | Dynamic loading path search |
| `error.c` | 4,412 | Not Started | `crate::error` | Exception and error handling |
| `id_table.c` | 569 | Not Started | `crate::id_table` | ID lookup table |
| `inits.c` | 116 | Not Started | `crate::inits` | Subsystem initializers |
| `load.c` | 1,828 | Not Started | `crate::load` | Require and load mechanism |
| `loadpath.c` | 91 | Not Started | `crate::loadpath` | LOAD_PATH initialization |
| `localeinit.c` | 137 | Not Started | `crate::localeinit` | Locale initialization |
| `main.c` | 63 | Not Started | `crate::main` | Main entry point |
| `marshal.c` | 2,695 | Not Started | `crate::marshal` | Marshal serialization |
| `math.c` | 1,216 | Not Started | `crate::math` | Math module functions |
| `mini_builtin.c` | 118 | Not Started | `crate::mini_builtin` | Miniruby builtin features |
| `miniinit.c` | 109 | Not Started | `crate::miniinit` | Miniruby initializers |
| `siphash.c` | 493 | Not Started | `crate::siphash` | SipHash hashing algorithm |
| `sprintf.c` | 1,283 | Not Started | `crate::sprintf` | Kernel#sprintf formatting |
| `st.c` | 3,391 | Not Started | `crate::st` | Symbol table / hash table internal implementation |
| `strftime.c` | 1,285 | Not Started | `crate::strftime` | Date/time formatting |
| `util.c` | 616 | Not Started | `crate::util` | Core utility functions |
| `variable.c` | 4,710 | Not Started | `crate::variable` | Global and instance variable access |
| `version.c` | 299 | Not Started | `crate::version` | Ruby version constants |
| `vsnprintf.c` | 1,303 | Not Started | `crate::vsnprintf` | Portable vsnprintf |

### Platform & Miscellaneous
_Platform stubs, runner wrappers, and target-specific code_

| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `dmydln.c` | 31 | N/A | `N/A` | Dummy dynamic linking stub |
| `dmyenc.c` | 24 | N/A | `N/A` | Dummy encoding stub |
| `dmyext.c` | 18 | N/A | `N/A` | Dummy extension stub |
| `goruby.c` | 68 | N/A | `N/A` | Golf Ruby executable wrapper |
| `ruby-runner.c` | 104 | N/A | `N/A` | Development runner binary |
| `ruby.c` | 3,315 | In Progress | `crate::ruby_cli` | Main Ruby CLI argument handling |
| `rubystub.c` | 61 | N/A | `N/A` | Embedded stub |
| `sparc.c` | 40 | N/A | `N/A` | SPARC architecture assembly helper |

## Complete C Source File Ledger

| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `addr2line.c` | Utilities & Support | 2,773 | Not Started | `crate::addr2line` | Address to source line resolution |
| `array.c` | Core Data Structures | 9,119 | Not Started | `crate::array` | Array object implementation |
| `ast.c` | Parser & AST | 1,280 | Not Started | `crate::parser::ast` | Ruby::AST module |
| `bignum.c` | Core Data Structures | 7,350 | Not Started | `crate::bignum` | Big integer support |
| `box.c` | Utilities & Support | 1,346 | Not Started | `crate::box` | Value boxing helpers |
| `builtin.c` | Utilities & Support | 145 | Not Started | `crate::builtin` | Builtin Ruby class loader |
| `class.c` | Core Data Structures | 3,404 | Not Started | `crate::class` | Class and module hierarchy |
| `compar.c` | Core Data Structures | 355 | Not Started | `crate::compar` | Comparable mixin module |
| `compile.c` | JIT Compiler | 15,387 | Not Started | `crate::compile` | Bytecode compiler |
| `complex.c` | Core Data Structures | 2,822 | Not Started | `crate::complex` | Complex number class |
| `concurrent_set.c` | Concurrency & Threads | 522 | Not Started | `crate::concurrent_set` | Lock-free concurrent set |
| `cont.c` | Virtual Machine & Execution | 3,908 | Not Started | `crate::cont` | Continuation and Fiber core |
| `debug.c` | Utilities & Support | 729 | Not Started | `crate::debug` | Debugging helpers |
| `debug_counter.c` | Utilities & Support | 150 | Not Started | `crate::debug_counter` | Performance debug counters |
| `dir.c` | IO & Filesystem | 4,153 | Not Started | `crate::dir` | Directory operations |
| `dln.c` | Utilities & Support | 567 | Not Started | `crate::dln` | Dynamic linking loader |
| `dln_find.c` | Utilities & Support | 293 | Not Started | `crate::dln_find` | Dynamic loading path search |
| `dmydln.c` | Platform & Miscellaneous | 31 | N/A | `N/A` | Dummy dynamic linking stub |
| `dmyenc.c` | Platform & Miscellaneous | 24 | N/A | `N/A` | Dummy encoding stub |
| `dmyext.c` | Platform & Miscellaneous | 18 | N/A | `N/A` | Dummy extension stub |
| `encoding.c` | Encoding & Regex | 2,105 | Not Started | `crate::encoding` | String encoding support |
| `enum.c` | Core Data Structures | 5,297 | Not Started | `crate::enum` | Enumerable module |
| `enumerator.c` | Core Data Structures | 4,802 | Not Started | `crate::enumerator` | Enumerator class |
| `error.c` | Utilities & Support | 4,412 | Not Started | `crate::error` | Exception and error handling |
| `eval.c` | Virtual Machine & Execution | 2,344 | Not Started | `crate::eval` | Top-level evaluation entry points |
| `eval_error.c` | Virtual Machine & Execution | 588 | Not Started | `crate::eval::error` | Evaluation error handling |
| `eval_jump.c` | Virtual Machine & Execution | 144 | Not Started | `crate::eval::jump` | Control flow jumps (throw, break, return) |
| `file.c` | IO & Filesystem | 9,237 | Not Started | `crate::file` | File system operations |
| `gc.c` | Memory & Garbage Collection | 6,642 | In Progress | `gc::mmtk` | MMTk and GC integration |
| `goruby.c` | Platform & Miscellaneous | 68 | N/A | `N/A` | Golf Ruby executable wrapper |
| `hash.c` | Core Data Structures | 8,045 | Not Started | `crate::hash` | Hash map implementation |
| `id_table.c` | Utilities & Support | 569 | Not Started | `crate::id_table` | ID lookup table |
| `imemo.c` | Memory & Garbage Collection | 726 | Not Started | `crate::imemo` | Internal memo objects |
| `inits.c` | Utilities & Support | 116 | Not Started | `crate::inits` | Subsystem initializers |
| `io.c` | IO & Filesystem | 16,196 | Not Started | `crate::io` | IO class operations |
| `io_buffer.c` | IO & Filesystem | 4,604 | Not Started | `crate::io::buffer` | IO::Buffer implementation |
| `iseq.c` | JIT Compiler | 4,740 | Not Started | `crate::iseq` | Instruction sequences |
| `jit.c` | JIT Compiler | 921 | In Progress | `jit` | JIT common interface layer |
| `load.c` | Utilities & Support | 1,828 | Not Started | `crate::load` | Require and load mechanism |
| `loadpath.c` | Utilities & Support | 91 | Not Started | `crate::loadpath` | LOAD_PATH initialization |
| `localeinit.c` | Utilities & Support | 137 | Not Started | `crate::localeinit` | Locale initialization |
| `main.c` | Utilities & Support | 63 | Not Started | `crate::main` | Main entry point |
| `marshal.c` | Utilities & Support | 2,695 | Not Started | `crate::marshal` | Marshal serialization |
| `math.c` | Utilities & Support | 1,216 | Not Started | `crate::math` | Math module functions |
| `memory_view.c` | Memory & Garbage Collection | 902 | Not Started | `crate::memory_view` | Memory view interface |
| `mini_builtin.c` | Utilities & Support | 118 | Not Started | `crate::mini_builtin` | Miniruby builtin features |
| `miniinit.c` | Utilities & Support | 109 | Not Started | `crate::miniinit` | Miniruby initializers |
| `node.c` | Parser & AST | 447 | Not Started | `crate::parser::node` | AST node construction |
| `node_dump.c` | Parser & AST | 1,325 | Not Started | `crate::parser::node_dump` | AST dump utilities |
| `numeric.c` | Core Data Structures | 6,817 | Not Started | `crate::numeric` | Numeric base classes and operations |
| `object.c` | Core Data Structures | 4,739 | Not Started | `crate::object` | Object class methods and operations |
| `pack.c` | Core Data Structures | 1,932 | Not Started | `crate::pack` | Array#pack and String#unpack |
| `parser_st.c` | Parser & AST | 171 | Not Started | `crate::parser::parser_st` | Parser symbol table |
| `pathname.c` | IO & Filesystem | 457 | Not Started | `crate::pathname` | Pathname standard helper |
| `prism_compile.c` | Parser & AST | 11,269 | Not Started | `crate::parser::prism_compile` | Prism AST compiler |
| `prism_init.c` | Parser & AST | 9 | Not Started | `crate::parser::prism_init` | Prism initialization |
| `proc.c` | Core Data Structures | 5,537 | Not Started | `crate::proc` | Proc, Method, and Binding objects |
| `process.c` | IO & Filesystem | 9,698 | Not Started | `crate::process` | Process management |
| `ractor.c` | Concurrency & Threads | 4,242 | Not Started | `crate::ractor` | Ractor actor model implementation |
| `ractor_sync.c` | Concurrency & Threads | 1,922 | Not Started | `crate::ractor::sync` | Ractor synchronization |
| `random.c` | IO & Filesystem | 1,862 | Not Started | `crate::random` | Random number generation |
| `range.c` | Core Data Structures | 3,006 | Not Started | `crate::range` | Range object implementation |
| `rational.c` | Core Data Structures | 2,847 | Not Started | `crate::rational` | Rational number implementation |
| `re.c` | Encoding & Regex | 5,186 | Not Started | `crate::re` | Regexp class interface |
| `regcomp.c` | Encoding & Regex | 6,755 | Not Started | `crate::regcomp` | Onigmo regex compiler |
| `regenc.c` | Encoding & Regex | 1,032 | Not Started | `crate::regenc` | Onigmo encoding engine |
| `regerror.c` | Encoding & Regex | 406 | Not Started | `crate::regerror` | Onigmo regex error reporting |
| `regexec.c` | Encoding & Regex | 5,376 | Not Started | `crate::regexec` | Onigmo regex execution engine |
| `regparse.c` | Encoding & Regex | 6,853 | Not Started | `crate::regparse` | Onigmo regex parser |
| `regsyntax.c` | Encoding & Regex | 388 | Not Started | `crate::regsyntax` | Onigmo syntax options |
| `ruby-runner.c` | Platform & Miscellaneous | 104 | N/A | `N/A` | Development runner binary |
| `ruby.c` | Platform & Miscellaneous | 3,315 | In Progress | `crate::ruby_cli` | Main Ruby CLI argument handling |
| `ruby_parser.c` | Parser & AST | 1,151 | Not Started | `crate::parser::ruby_parser` | Ruby parser driver |
| `rubystub.c` | Platform & Miscellaneous | 61 | N/A | `N/A` | Embedded stub |
| `scheduler.c` | Concurrency & Threads | 1,395 | Not Started | `crate::scheduler` | Fiber scheduler interface |
| `set.c` | Core Data Structures | 2,663 | Not Started | `crate::set` | Core Set class support |
| `shape.c` | Memory & Garbage Collection | 1,716 | Not Started | `crate::shape` | Object shape / property layout tracking |
| `signal.c` | Concurrency & Threads | 1,646 | Not Started | `crate::signal` | Signal handling |
| `siphash.c` | Utilities & Support | 493 | Not Started | `crate::siphash` | SipHash hashing algorithm |
| `sparc.c` | Platform & Miscellaneous | 40 | N/A | `N/A` | SPARC architecture assembly helper |
| `sprintf.c` | Utilities & Support | 1,283 | Not Started | `crate::sprintf` | Kernel#sprintf formatting |
| `st.c` | Utilities & Support | 3,391 | Not Started | `crate::st` | Symbol table / hash table internal implementation |
| `strftime.c` | Utilities & Support | 1,285 | Not Started | `crate::strftime` | Date/time formatting |
| `string.c` | Core Data Structures | 14,460 | Not Started | `crate::string` | String object implementation |
| `struct.c` | Core Data Structures | 2,322 | Not Started | `crate::struct` | Struct class implementation |
| `symbol.c` | Core Data Structures | 1,467 | Not Started | `crate::symbol` | Symbol management |
| `thread.c` | Concurrency & Threads | 6,757 | Not Started | `crate::thread` | Thread management core |
| `thread_none.c` | Concurrency & Threads | 384 | Not Started | `crate::thread::none` | No-threads platform stubs |
| `thread_pthread.c` | Concurrency & Threads | 1,648 | Not Started | `crate::thread::pthread` | POSIX threads implementation |
| `thread_sched.c` | Concurrency & Threads | 2,686 | Not Started | `crate::thread::sched` | Thread scheduler |
| `thread_sched_mn.c` | Concurrency & Threads | 1,931 | Not Started | `crate::thread::sched_mn` | M:N thread scheduler |
| `thread_sync.c` | Concurrency & Threads | 1,545 | Not Started | `crate::thread::sync` | Thread synchronization primitives |
| `thread_win32.c` | Concurrency & Threads | 980 | Not Started | `crate::thread::win32` | Windows threads implementation |
| `time.c` | Core Data Structures | 6,101 | Not Started | `crate::time` | Time class and operations |
| `transcode.c` | Encoding & Regex | 4,709 | Not Started | `crate::transcode` | Character transcoding |
| `universal_parser.c` | Parser & AST | 215 | Not Started | `crate::parser::universal_parser` | Universal parser interface |
| `util.c` | Utilities & Support | 616 | Not Started | `crate::util` | Core utility functions |
| `variable.c` | Utilities & Support | 4,710 | Not Started | `crate::variable` | Global and instance variable access |
| `version.c` | Utilities & Support | 299 | Not Started | `crate::version` | Ruby version constants |
| `vm.c` | Virtual Machine & Execution | 5,408 | Not Started | `crate::vm` | Core virtual machine engine |
| `vm_args.c` | Virtual Machine & Execution | 1,219 | Not Started | `crate::vm::args` | Method argument passing |
| `vm_backtrace.c` | Virtual Machine & Execution | 2,406 | Not Started | `crate::vm::backtrace` | Backtrace generation |
| `vm_dump.c` | Virtual Machine & Execution | 1,656 | Not Started | `crate::vm::dump` | VM state dump utilities |
| `vm_eval.c` | Virtual Machine & Execution | 2,981 | Not Started | `crate::vm::eval` | Method dispatch and evaluation |
| `vm_exec.c` | Virtual Machine & Execution | 154 | Not Started | `crate::vm::exec` | VM loop execution |
| `vm_insnhelper.c` | Virtual Machine & Execution | 7,797 | Not Started | `crate::vm::insnhelper` | Instruction execution helpers |
| `vm_method.c` | Virtual Machine & Execution | 3,786 | Not Started | `crate::vm::method` | Method table management |
| `vm_sync.c` | Virtual Machine & Execution | 282 | Not Started | `crate::vm::sync` | VM synchronization primitives |
| `vm_trace.c` | Virtual Machine & Execution | 1,971 | Not Started | `crate::vm::trace` | TracePoint and event hooks |
| `vsnprintf.c` | Utilities & Support | 1,303 | Not Started | `crate::vsnprintf` | Portable vsnprintf |
| `weakmap.c` | Core Data Structures | 999 | Not Started | `crate::weakmap` | ObjectSpace::WeakMap |
| `yjit.c` | JIT Compiler | 526 | Ported | `yjit` | YJIT engine implemented in Rust |
| `zjit.c` | JIT Compiler | 389 | In Progress | `zjit` | ZJIT compiler framework in Rust |
