use limine::{
    request::{self, HhdmRequest, MemoryMapRequest},
    response::MemoryMapResponse,
};
use vmm::{VMMFlags, KERMAP};
pub mod pmm;
use spin::Mutex;
#[used]
#[link_section = ".requests"]
pub static MEMMAP: Mutex<limine::request::MemoryMapRequest> = Mutex::new(MemoryMapRequest::new());
#[used]
#[link_section = ".requests"]
pub static HHDM: limine::request::HhdmRequest = HhdmRequest::new();
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

use core::alloc::GlobalAlloc;
#[cfg_attr(not(test), no_main, no_std)]
#[cfg_attr(target_os = "none", global_allocator)]

static MGR: MemoryManagement = MemoryManagement;
struct MemoryManagement;

use crate::{mem::pmm::cool, println};

unsafe impl GlobalAlloc for MemoryManagement {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        match  cool.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                let q = q as *mut u8;
                
                
                return q;
            }
            None => {let x = KERMAP
                .lock()
                .as_mut()
                .unwrap()
                .vmm_region_alloc(
                    layout.size(),
                    VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED,
                )
                .unwrap();
            
            x},
        }
    }
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        match cool.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                let q = q as *mut u8;
                
                return q;
            }
            None => KERMAP
                .lock()
                .as_mut()
                .unwrap()
                .vmm_region_alloc(
                    layout.size(),
                    VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED,
                )
                .unwrap(),
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.size() > 4096 {
            KERMAP
                .lock()
                .as_mut()
                .unwrap()
                .vmm_region_dealloc(ptr);
        } else {
            return  cool.lock().as_mut().unwrap().free(ptr);
        }
    }
}
pub mod vmm;
