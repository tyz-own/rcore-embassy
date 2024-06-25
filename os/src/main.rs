//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`syscall`]: System call handling and implementation
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`batch::run_next_app()`] and for the first time go to
//! userspace.

#![deny(missing_docs)]
// #![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(impl_trait_in_assoc_type)]
#[macro_use]
extern crate log;

use core::arch::global_asm;
#[path = "boards/qemu.rs"]
mod board;
use log::*;
#[macro_use]
mod console;
pub mod batch;
pub mod lang_items;
pub mod logging;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod trap;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize).fill(0);
    }
}

/// the rust entry-point of os
#[no_mangle]
pub fn rust_main() -> ! {
    extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_top(); // stack top
    }
    clear_bss();
    logging::init();
    println!("[kernel] Hello, world!");
    trace!("[kernel] .text [{:#x}, {:#x})", stext as usize, etext as usize);
    debug!("[kernel] .rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!("[kernel] .data [{:#x}, {:#x})", sdata as usize, edata as usize);
    warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
    error!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    trap::init();
    batch::init();
    // batch::run_next_app();
    embassy_runtime();
}

fn embassy_runtime() -> ! {
    use core::sync::atomic::{AtomicU8, Ordering};
    use embassy_executor::raw::Executor;
    use embassy_time_driver::{AlarmHandle, Driver};

    struct MyDriver {} // not public!

    impl Driver for MyDriver {
        fn now(&self) -> u64 {
            riscv::register::time::read64()
        }
        unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
            static ALARM: AtomicU8 = AtomicU8::new(0);
            Some(AlarmHandle::new(ALARM.fetch_add(1, Ordering::Relaxed)))
        }
        fn set_alarm_callback(&self, alarm: AlarmHandle, _callback: fn(*mut ()), _ctx: *mut ()) {
            debug!("now={} alarm_id={} set_alarm_callback", self.now(), alarm.id(),);
            // unsafe { riscv::register::sstatus::set_sie() };
            
        }
        fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool {
            let now = self.now();
            // debug!("now={now} timestamp={timestamp} alarm_id={} set_alarm", alarm.id());
            let set = now < timestamp;
            if set {
                info!("now={now} set_timer for timestamp={timestamp}");
                // set timer interrupt to wake up CPU from wfi
                sbi::set_timer(timestamp as usize);
                unsafe { riscv::register::sstatus::set_sie() };
            }
            set
            // false
        }
    }

    embassy_time_driver::time_driver_impl!(static DRIVER: MyDriver = MyDriver{});

    static mut RUNTIME: Option<Executor> = None;
    let runtime = unsafe { RUNTIME.get_or_insert_with(|| Executor::new(&mut ())) };
    info!("runtime init");
    let spawner = runtime.spawner();
    info!("runtime starts");
    // spawner.spawn(run1(1, || warn!("[task 1] tick for 1 sec"))).unwrap();
    // spawner.spawn(run(2, || warn!("[task 2] tick for 2 sec"))).unwrap();
    spawner.spawn(batch_run()).unwrap();
    for _ in 0..10 {
        debug!("polled once");
        unsafe { runtime.poll() };

        let sstatus = riscv::register::sstatus::read();
        let timer = riscv::register::sie::read().stimer();
        info!("[wfi 1] sie.timer={timer} spie={} sie={}", sstatus.spie(), sstatus.sie());

        // The SPIE bit indicates whether supervisor interrupts were enabled prior to trapping into supervisor
        // mode. When a trap is taken into supervisor mode, SPIE is set to SIE, and SIE is set to 0.
        // 每次陷入，SIE 状态会被清零，为了在 S 态响应下次计时器中断，需要开启 SIE
        // unsafe { riscv::register::sstatus::set_sie() };

        unsafe { core::arch::asm!("wfi") };

        let sstatus = riscv::register::sstatus::read();
        let timer = riscv::register::sie::read().stimer();
        info!("[wfi 2] sie.timer={timer} spie={} sie={}", sstatus.spie(), sstatus.sie());

        // unsafe { riscv::register::sstatus::clear_sie() };

        info!("awake from wfi");
    }
    let sstatus = riscv::register::sstatus::read();
    info!("spie={} sie={}", sstatus.spie(), sstatus.sie());
    board::exit_success();
}

#[embassy_executor::task]
async fn run(sec: u64, f: fn()) {
    loop {
        embassy_time::Timer::after_secs(sec).await;
        f();
        // info!("tick for 1 sec");
    }
}

#[embassy_executor::task]
async fn run1(sec: u64, f: fn()) {
    loop {
        embassy_time::Timer::after_secs(sec).await;
        f();
        // info!("tick for 1 sec");
    }
}
#[embassy_executor::task]
async fn batch_run() -> ! {
    // embassy_time::Timer::after_secs(4).await;
    // unsafe { riscv::register::sstatus::clear_sie() };
    batch::run_next_app();
    panic!("batch_run");

}

#[no_mangle]
fn __pender(_ctx: *mut ()) {
    info!("call __pender_");
}
