#![no_std]
#![no_main]

use core::arch::asm;

use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

use NyauxKT::gdt::init_gdt;
use NyauxKT::idt::InterruptManager;

use owo_colors::OwoColorize;
use NyauxKT::mem::pmm::{pmm_alloc, pmm_dealloc, pmm_init, FREEPAGES};
use NyauxKT::println;
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
            println!("Booting Kernel...");
            pmm_init();
            println!("it worked!");
            let funny = &mut *(pmm_alloc().unwrap() as *mut usize);
            *funny = 5;

            println!("funny is !!! {}", *funny);
            pmm_dealloc(funny as *mut usize as usize);
            println!("trying out vectors");
            let mut funny = alloc::vec::Vec::new();
            funny.push(1);
            funny.push(2);
            funny.push(3);
            funny.push(4);
            funny.push(5);
            assert_eq!([1, 2, 3, 4, 5], *funny);
            println!("IT WORKED");
            println!("{:#?}", funny);
            drop(funny);
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
