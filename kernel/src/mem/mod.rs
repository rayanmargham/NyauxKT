pub mod pmm;
pub mod vmm;

use core::alloc::GlobalAlloc;

#[global_allocator]
static MGR: MemoryManagement = MemoryManagement;
struct MemoryManagement;

use crate::mem::pmm::KmallocManager;

unsafe impl GlobalAlloc for MemoryManagement {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        match KmallocManager.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                q.write_bytes(0, layout.size());
                return q;
            }
            None => {
                panic!("unimplmented the VMM");
            }
        }
    }
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        match KmallocManager.lock().as_mut().unwrap().alloc(layout.size()) {
            Some(q) => {
                q.write_bytes(0, layout.size());
                return q;
            }
            None => {
                panic!("no vmm");
            }
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.size() > 4096 {
            panic!("no vmm")
        } else {
            return KmallocManager.lock().as_mut().unwrap().free(ptr as u64);
        }
    }
}
