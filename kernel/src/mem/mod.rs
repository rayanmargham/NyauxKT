use limine::{
    request::{self, HhdmRequest, MemoryMapRequest},
    response::MemoryMapResponse,
};
pub mod pmm;
#[used]
#[link_section = ".requests"]
pub static MEMMAP: limine::request::MemoryMapRequest = MemoryMapRequest::new();
#[used]
#[link_section = ".requests"]
pub static HHDM: limine::request::HhdmRequest = HhdmRequest::new();
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}
