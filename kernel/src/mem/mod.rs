use limine::{
    request::{self, HhdmRequest, MemoryMapRequest},
    response::MemoryMapResponse,
};
use vmm::{VMMFlags, KERMAP};
pub mod pmm;
#[used]
#[link_section = ".requests"]
pub static mut MEMMAP: limine::request::MemoryMapRequest = MemoryMapRequest::new();
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

#[global_allocator]
static MGR: MemoryManagement = MemoryManagement;
struct MemoryManagement;

use crate::{mem::pmm::cool, println};

unsafe impl GlobalAlloc for MemoryManagement {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        match cool.0.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                let q = q as *mut u8;
                q.write_bytes(0, layout.size());
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
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        match cool.0.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                let q = q as *mut u8;
                q.write_bytes(0, layout.size());
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
                .vmm_region_dealloc(ptr as usize);
        } else {
            return cool.0.lock().as_mut().unwrap().free(ptr as usize);
        }
    }
}
pub mod vmm;
