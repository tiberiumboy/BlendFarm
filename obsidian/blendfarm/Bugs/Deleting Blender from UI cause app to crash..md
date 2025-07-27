Seems like the code was not implemented to delete local content of blender file. 
We should provide a dialog asking user to disconnect blender link or delete local content where blender is store/installed.

Error log: 
thread 'main' panicked at src/routes/settings.rs:139:5:
not yet implemented: Impl function to delete blender and its local contents
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' panicked at library/core/src/panicking.rs:226:5:
panic in a function that cannot unwind
stack backtrace:
   0:     0x56fe637084da - std::backtrace_rs::backtrace::libunwind::trace::h74680e970b6e0712
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
   1:     0x56fe637084da - std::backtrace_rs::backtrace::trace_unsynchronized::ha3bf590e3565a312
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
   2:     0x56fe637084da - std::sys::backtrace::_print_fmt::hcf16024cbdd6c458
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/sys/backtrace.rs:66:9
   3:     0x56fe637084da - <std::sys::backtrace::BacktraceLock::print::DisplayBacktrace as core::fmt::Display>::fmt::h46a716bba2450163
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/sys/backtrace.rs:39:26
   4:     0x56fe6294a7fa - core::fmt::rt::Argument::fmt::ha695e732309707b7
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/fmt/rt.rs:181:76
   5:     0x56fe6294a7fa - core::fmt::write::h275e5980d7008551
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/fmt/mod.rs:1446:25
   6:     0x56fe636fd469 - std::io::default_write_fmt::hdc4119be3eb77042
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/io/mod.rs:639:11
   7:     0x56fe636fd469 - std::io::Write::write_fmt::h561a66a0340b6995
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/io/mod.rs:1914:13
   8:     0x56fe63708147 - std::sys::backtrace::BacktraceLock::print::hafb9d5969adc39a0
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/sys/backtrace.rs:42:9
   9:     0x56fe6370c05d - std::panicking::default_hook::{{closure}}::hae2e97a5c4b2b777
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:300:22
  10:     0x56fe6370bcf1 - std::panicking::default_hook::h3db1b505cfc4eb79
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:327:9
  11:     0x56fe6370d5d4 - std::panicking::rust_panic_with_hook::h409da73ddef13937
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:833:13
  12:     0x56fe6370d012 - std::panicking::begin_panic_handler::{{closure}}::h159b61b27f96a9c2
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:699:13
  13:     0x56fe63708d29 - std::sys::backtrace::__rust_end_short_backtrace::h5b56844d75e766fc
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/sys/backtrace.rs:168:18
  14:     0x56fe6370c8a5 - __rustc[4794b31dd7191200]::rust_begin_unwind
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:697:5
  15:     0x56fe629452c4 - core::panicking::panic_nounwind_fmt::runtime::h4c94eb695becba00
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/panicking.rs:117:22
  16:     0x56fe629452c4 - core::panicking::panic_nounwind_fmt::hc3cf3432011a3c3f
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/intrinsics/mod.rs:3196:9
  17:     0x56fe6294536c - core::panicking::panic_nounwind::h0c59dc9f7f043ead
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/panicking.rs:226:5
  18:     0x56fe6294555d - core::panicking::panic_cannot_unwind::hb8732afd89555502
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/panicking.rs:331:5
  19:     0x56fe62381f7f - webkit2gtk::auto::web_context::WebContextExt::register_uri_scheme::callback_func::h8fe0af92b8260675
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/webkit2gtk-2.0.1/src/auto/web_context.rs:534:5
  20:     0x70c213c31162 - <unknown>
  21:     0x70c213b64af1 - <unknown>
  22:     0x70c213b64d75 - <unknown>
  23:     0x70c213667981 - <unknown>
  24:     0x70c2136825fb - <unknown>
  25:     0x70c213a83969 - <unknown>
  26:     0x70c213ba1bdf - <unknown>
  27:     0x70c21368fbda - <unknown>
  28:     0x70c213a7e175 - <unknown>
  29:     0x70c213a7eb70 - <unknown>
  30:     0x70c211acab62 - <unknown>
  31:     0x70c211b6bf6d - <unknown>
  32:     0x70c211b6ce4d - <unknown>
  33:     0x70c21011449e - <unknown>
  34:     0x70c210173737 - <unknown>
  35:     0x70c210113a63 - g_main_context_iteration
  36:     0x70c2127feced - gtk_main_iteration_do
  37:     0x56fe62b12f06 - gtk::auto::functions::main_iteration_do::h270128f04301322a
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gtk-0.18.2/src/auto/functions.rs:392:24
  38:     0x56fe62299430 - tao::platform_impl::platform::event_loop::EventLoop<T>::run_return::{{closure}}::hcd650c02c0270bad
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.34.0/src/platform_impl/linux/event_loop.rs:1131:11
  39:     0x56fe6209bfdd - glib::main_context::<impl glib::auto::main_context::MainContext>::with_thread_default::hc5f182a0d134ca2f
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glib-0.18.5/src/main_context.rs:154:12
  40:     0x56fe62298e7a - tao::platform_impl::platform::event_loop::EventLoop<T>::run_return::h58348637986d0636
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.34.0/src/platform_impl/linux/event_loop.rs:1029:5
  41:     0x56fe6229a1c2 - tao::platform_impl::platform::event_loop::EventLoop<T>::run::h0d755a90eec56b5a
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.34.0/src/platform_impl/linux/event_loop.rs:983:21
  42:     0x56fe621b075e - tao::event_loop::EventLoop<T>::run::hee559644b11c98ad
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.34.0/src/event_loop.rs:215:5
  43:     0x56fe62539017 - <tauri_runtime_wry::Wry<T> as tauri_runtime::Runtime<T>>::run::ha78a1e8a8ae6cac2
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-runtime-wry-2.7.1/src/lib.rs:3013:5
  44:     0x56fe62755999 - tauri::app::App<R>::run::h70ffe936223722e3
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.6.2/src/app.rs:1228:5
  45:     0x56fe621c94e9 - <blenderfarm_lib::services::tauri_app::TauriApp as blenderfarm_lib::services::blend_farm::BlendFarm>::run::{{closure}}::haa95878b5a934c4b
                               at /home/oem/Documents/src/rust/BlendFarm/src-tauri/src/services/tauri_app.rs:748:9
  46:     0x56fe61df99e8 - <core::pin::Pin<P> as core::future::future::Future>::poll::h39b7691369c65b38
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/future/future.rs:124:9
  47:     0x56fe61d60847 - blenderfarm_lib::run::{{closure}}::h89cba7da89eea434
                               at /home/oem/Documents/src/rust/BlendFarm/src-tauri/src/lib.rs:97:14
  48:     0x56fe61e3a9ab - blendfarm::main::{{closure}}::hc1cd5edd9e091630
                               at /home/oem/Documents/src/rust/BlendFarm/src-tauri/src/main.rs:6:28
  49:     0x56fe61df9e96 - <core::pin::Pin<P> as core::future::future::Future>::poll::he7015f46e5ea4160
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/future/future.rs:124:9
  50:     0x56fe62aa6ee5 - tokio::runtime::park::CachedParkThread::block_on::{{closure}}::h389ef3b346ca552e
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/park.rs:285:60
  51:     0x56fe62aa62a6 - tokio::task::coop::with_budget::h72cee197898239cf
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/task/coop/mod.rs:167:5
  52:     0x56fe62aa62a6 - tokio::task::coop::budget::hbc43922e3f16b65a
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/task/coop/mod.rs:133:5
  53:     0x56fe62aa62a6 - tokio::runtime::park::CachedParkThread::block_on::h0b5e525ca8ad4151
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/park.rs:285:31
  54:     0x56fe62a36ebb - tokio::runtime::context::blocking::BlockingRegionGuard::block_on::hb728eb4d72a4fd00
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/context/blocking.rs:66:9
  55:     0x56fe61dbc201 - tokio::runtime::scheduler::multi_thread::MultiThread::block_on::{{closure}}::h022469cbf31ad7ed
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/scheduler/multi_thread/mod.rs:87:13
  56:     0x56fe61d8f55a - tokio::runtime::context::runtime::enter_runtime::h704bc2f73f22b9bf
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/context/runtime.rs:65:16
  57:     0x56fe61dbc19d - tokio::runtime::scheduler::multi_thread::MultiThread::block_on::h47fd685f100b211b
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/scheduler/multi_thread/mod.rs:86:9
  58:     0x56fe61dbf8dd - tokio::runtime::runtime::Runtime::block_on_inner::h1693313548f8bba8
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/runtime.rs:358:45
  59:     0x56fe61dbfd33 - tokio::runtime::runtime::Runtime::block_on::h2f4a7c23c7d9c7f9
                               at /home/oem/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.46.1/src/runtime/runtime.rs:328:13
  60:     0x56fe61dff13e - blendfarm::main::hb5b26bc1d924c0ed
                               at /home/oem/Documents/src/rust/BlendFarm/src-tauri/src/main.rs:6:5
  61:     0x56fe62a5c753 - core::ops::function::FnOnce::call_once::h0dba2a157be0e99e
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ops/function.rs:250:5
  62:     0x56fe61ceb286 - std::sys::backtrace::__rust_begin_short_backtrace::h2c8415a7e4b9be43
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys/backtrace.rs:152:18
  63:     0x56fe61e2f929 - std::rt::lang_start::{{closure}}::hf1b42969a1811d7c
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/rt.rs:199:18
  64:     0x56fe636e9049 - core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::hb4b7cf0559a1a53b
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/core/src/ops/function.rs:284:13
  65:     0x56fe636e9049 - std::panicking::try::do_call::h8e6004e979ada7de
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:589:40
  66:     0x56fe636e9049 - std::panicking::try::hc44a0c902e55fa8c
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:552:19
  67:     0x56fe636e9049 - std::panic::catch_unwind::h6a5f1ccd4faaed9e
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panic.rs:359:14
  68:     0x56fe636e9049 - std::rt::lang_start_internal::{{closure}}::h40fd26f9e7cfe6a7
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/rt.rs:168:24
  69:     0x56fe636e9049 - std::panicking::try::do_call::h047dd894cf3f6fd1
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:589:40
  70:     0x56fe636e9049 - std::panicking::try::h921841e1eaed56ce
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panicking.rs:552:19
  71:     0x56fe636e9049 - std::panic::catch_unwind::h108064a50ee785ec
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/panic.rs:359:14
  72:     0x56fe636e9049 - std::rt::lang_start_internal::ha8ef919ae4984948
                               at /rustc/6b00bc3880198600130e1cf62b8f8a93494488cc/library/std/src/rt.rs:164:5
  73:     0x56fe61e2f911 - std::rt::lang_start::h453680834249629d
                               at /home/oem/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/rt.rs:198:5
  74:     0x56fe61dff1ee - main
  75:     0x70c20fc2a1ca - __libc_start_call_main
                               at ./csu/../sysdeps/nptl/libc_start_call_main.h:58:16
  76:     0x70c20fc2a28b - __libc_start_main_impl
                               at ./csu/../csu/libc-start.c:360:3
  77:     0x56fe61cdb395 - _start
  78:                0x0 - <unknown>
thread caused non-unwinding panic. aborting.