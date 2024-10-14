#![no_std]
#![cfg_attr(not(test), no_main)]

use core::arch::asm;
use core::assert;
use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

use NyauxKT::gdt::init_gdt;
use NyauxKT::idt::InterruptManager;

use NyauxKT::mem::pmm::pmm_init;
use NyauxKT::mem::vmm;
use NyauxKT::smp::bootstrap;
use NyauxKT::{acpi, println};

use build_timestamp::build_time;
use NyauxKT::term::TERMGBL;
/// Sets the base revision to the latest revision supported by the crate.
/// See specification for further info.
/// Be sure to mark all limine requests with #[used], otherwise they may be removed by the compiler.
#[used]
// The .requests section allows limine to find the requests faster and more safely.
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[link_section = ".requests_start_marker"]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[link_section = ".requests_end_marker"]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();
extern crate alloc;
use alloc::string::ToString;
build_time!("%H:%M:%S on %A %Y-%m-%d");
#[no_mangle]

// #[cfg(not(test))]

unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.

    use NyauxKT::{elf::symbol, timers::init_timers};
    assert!(BASE_REVISION.is_supported());

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            init_gdt();

            InterruptManager::start_idt();
            TERMGBL.lock().init(&framebuffer);
            println!("Booting Kernel...");
            println!("Nyaux KT. Built at: {}", BUILD_TIME);
            pmm_init();

            symbol::load();
            vmm::PageMap::new_inital();

            acpi::init_acpi();
            init_timers().expect("Kernel does not have any timers, btw timer.rs wants to say hi");
            bootstrap();
        }
    }

    hcf();
}
#[cfg(miri)]
#[no_mangle]
fn miri_start(argc: isize, argv: *const *const u8) -> isize {
    // Call the actual start function that your project implements, based on your target's conventions.

    use NyauxKT::{idt::Registers, sched};
    unsafe {
        sched::sched_init();
    };
    sched::create_kentry();
    argc
}
#[cfg_attr(not(test), panic_handler)]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    println!("{}", _info);
    hcf();
}
#[test]
fn demo() {
    extern crate std;
    std::println!("hi");
    let mut x = std::vec::Vec::new();
    x.push(1);
    assert_eq!([1], *x);
}
pub fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
