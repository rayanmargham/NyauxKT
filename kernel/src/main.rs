#![no_std]
#![no_main]

use core::arch::asm;

use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

use NyauxKT::gdt::init_gdt;
use NyauxKT::idt::InterruptManager;

use NyauxKT::mem::pmm::PhysicalAllocator;
use NyauxKT::mem::vmm::PageMap;
use NyauxKT::println;
use NyauxKT::term::TERMGBL;
extern crate alloc;
use alloc::vec::Vec;
use owo_colors::OwoColorize;
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

#[no_mangle]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            init_gdt();

            InterruptManager::start_idt();
            TERMGBL.lock().init(&framebuffer);
            println!("hello world");
            PhysicalAllocator::new().unwrap();
            PageMap::new_inital();
            let mut test: Vec<i32> = Vec::new();
            test.push(5);
            test.push(4);
            test.push(3);
            test.push(2);
            test.push(1);
            test.push(0);
            assert_eq!([5, 4, 3, 2, 1, 0], *test);
            drop(test);
            println!("{}", "Yippie".green())
        }
    }

    hcf();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    println!("{}", _info);
    hcf();
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
