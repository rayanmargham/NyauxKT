use core::{alloc::Layout, ffi::c_void, fmt};

use limine::request::RsdpRequest;
use owo_colors::OwoColorize;
use uacpi::{kernel_api::KernelApi, Handle, IOAddr, LogLevel, PhysAddr};
extern crate alloc;
use alloc::boxed::Box;
use crate::{mem::HHDM, println};
pub struct IOthingy {
    base: IOAddr,
    len: usize
}
#[used]
#[link_section = ".requests"]
static rsdp: limine::request::RsdpRequest = RsdpRequest::new();
struct KTUACPIAPI;
static mut o: i32 = 0;
impl KernelApi for KTUACPIAPI {
    fn acquire_mutex(&self, mutex: uacpi::Handle, timeout: u16) -> bool {
        true
    }
    fn acquire_spinlock(&self, lock: uacpi::Handle) -> uacpi::CpuFlags {
        uacpi::CpuFlags::new(lock.as_u64())
    }
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let q = alloc::alloc::alloc(layout);
        if !(q as u64 % layout.align() as u64 == 0) {
            panic!("FUCK");
        }
        q
    }
    fn create_event(&self) -> uacpi::Handle {
        uacpi::Handle::new(1)
    }
    fn create_mutex(&self) -> uacpi::Handle {
        unsafe {
            o = o + 1;
            uacpi::Handle::new(o as u64)
        }
    }
    fn create_spinlock(&self) -> uacpi::Handle {
        uacpi::Handle::new(1)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        alloc::alloc::dealloc(ptr, layout);
    }
    fn destroy_event(&self, event: uacpi::Handle) {
        
    }
    fn destroy_mutex(&self, mutex: uacpi::Handle) {
        
    }
    fn destroy_spinlock(&self, lock: uacpi::Handle) {
        
    }
    fn firmware_request(&self, req: uacpi::FirmwareRequest) -> Result<(), uacpi::Status> {
        Ok(())
    }
    fn get_thread_id(&self) -> uacpi::ThreadId {
        uacpi::ThreadId::new(0 as *mut c_void)
    }
    fn get_ticks(&self) -> u64 {
        46644
    }
    fn install_interrupt_handler(&self, irq: u32, handler: Box<dyn Fn()>,
        ) -> Result<uacpi::Handle, uacpi::Status> {
       Ok(uacpi::Handle::new(4))
    }
    unsafe fn io_map(&self, base: uacpi::IOAddr, len: usize) -> Result<uacpi::Handle, uacpi::Status> {
        let bro = alloc::alloc::alloc(Layout::new::<IOthingy>());
        Ok(
            
            Handle::new(bro.addr() as u64)
        )
    }
    unsafe fn io_read(&self, handle: uacpi::Handle, offset: usize, byte_width: u8) -> Result<u64, uacpi::Status> {
        let ok = handle.as_u64();
        
       
        let ok = core::ptr::with_exposed_provenance_mut::<IOthingy>(ok as usize);
        
        self.raw_io_read((*ok).base, byte_width)
    }
    unsafe fn io_unmap(&self, handle: uacpi::Handle) {
        let ok = handle.as_u64();
        let ok = core::ptr::with_exposed_provenance_mut::<IOthingy>(ok as usize);
        alloc::alloc::dealloc(ok.cast::<u8>(), Layout::new::<IOthingy>());
    }
    unsafe fn io_write(
            &self,
            handle: uacpi::Handle,
            offset: usize,
            byte_width: u8,
            val: u64,
        ) -> Result<(), uacpi::Status> {
            let ok = handle.as_u64();
            let ok = core::ptr::with_exposed_provenance_mut::<IOthingy>(ok as usize);
            self.raw_io_write((*ok).base, byte_width, val)
    }
    fn log(&self, log_level: uacpi::LogLevel, string: &str) {
        println!("uacpi [{}]: {}", {
            match log_level {
                LogLevel::DEBUG => "Debug",
                LogLevel::ERROR => "Error",
                LogLevel::INFO => "Info",
                LogLevel::TRACE => "Trace",
                LogLevel::WARN => "Warn",
                _ => panic!("not possible")
            }
        }, string);
    }
    unsafe fn map(&self, phys: uacpi::PhysAddr, len: usize) -> *mut core::ffi::c_void {
        (phys.as_u64() + HHDM.get_response().unwrap().offset()) as *mut _
    }
    unsafe fn pci_read(
            &self,
            address: uacpi::PCIAddress,
            offset: usize,
            byte_width: u8,
        ) -> Result<u64, uacpi::Status> {
        Err(uacpi::Status::Unimplemented)
    }
    unsafe fn pci_write(
            &self,
            address: uacpi::PCIAddress,
            offset: usize,
            byte_width: u8,
            val: u64,
        ) -> Result<(), uacpi::Status> {
            Err(uacpi::Status::Unimplemented)
    }
    unsafe fn raw_io_read(&self, addr: uacpi::IOAddr, byte_width: u8) -> Result<u64, uacpi::Status> {
        if byte_width.is_power_of_two() == false {return Err(uacpi::Status::InvalidArgument);};
        match byte_width {
            1 => {
                let value: u8;
                core::arch::asm!("in al, dx", out("al") value, in("dx") addr.as_u64());
                Ok(value as u64)
            },
            2 => {
                let value: u16;
                core::arch::asm!("in ax, dx", out("ax") value, in("dx") addr.as_u64());
                Ok(value as u64)
            }
            4 => {
                let value: u32;
                core::arch::asm!("in eax, dx", out("eax") value, in("dx") addr.as_u64());
                Ok(value as u64)
            }, 
            _ => {
                return Err(uacpi::Status::InvalidArgument);
            }
        }
    }
    unsafe fn raw_io_write(&self, addr: uacpi::IOAddr, byte_width: u8, val: u64) -> Result<(), uacpi::Status> {
        if !byte_width.is_power_of_two() {return Err(uacpi::Status::InvalidArgument);};
        match byte_width {
            1 => {
                core::arch::asm!("out dx, al", in("al") val as u8, in("dx") addr.as_u64() as u16, options(nomem, nostack, preserves_flags));
                return Ok(());
            },
            2 => {
                core::arch::asm!("out dx, al", in("ax") val as u16, in("dx") addr.as_u64() as u16, options(nomem, nostack, preserves_flags));
                return Ok(());
            },
            4 => {
                core::arch::asm!("out dx, eax", in("dx") addr.as_u64() as u16, in("eax") val as u32, options(nomem, nostack, preserves_flags));
                return Ok(());
            },
            8 => {
                return Err(uacpi::Status::InvalidArgument);
            }
            _ => {
                return Err(uacpi::Status::InvalidArgument);
            }

        }
        
    }
    unsafe fn raw_memory_read(&self, phys: uacpi::PhysAddr, byte_width: u8) -> Result<u64, uacpi::Status> {
        let virt = phys.as_u64() as usize + HHDM.get_response().unwrap().offset() as usize;
        match byte_width {
            1 => {
                let letsgo = unsafe {&mut *(virt as *mut u8)};
                Ok(*letsgo as u64)
            },
            2 => {
                let letsgo = unsafe {&mut *(virt as *mut u16)};
                Ok(*letsgo as u64)
            },
            4 => {
                let letsgo = unsafe {&mut *(virt as *mut u32)};
                Ok(*letsgo as u64)
            },
            8 => {
                let letsgo = unsafe {&mut *(virt as *mut u64)};
                Ok(*letsgo as u64)
            },
            _ => {
                Err(uacpi::Status::InvalidArgument)
            }
        }
    }
    unsafe fn raw_memory_write(
            &self,
            phys: uacpi::PhysAddr,
            byte_width: u8,
            val: u64,
        ) -> Result<(), uacpi::Status> {
        let virt = phys.as_u64() as usize + HHDM.get_response().unwrap().offset() as usize;
        match byte_width {
            1 => {
                let letsgo = unsafe {&mut *(virt as *mut u8)};
                *letsgo = val as u8;
                Ok(())
            },
            2 => {
                let letsgo = unsafe {&mut *(virt as *mut u16)};
                *letsgo = val as u16;
                Ok(())
            },
            4 => {
                let letsgo = unsafe {&mut *(virt as *mut u32)};
                *letsgo = val as u32;
                Ok(())
            },
            8 => {
                let letsgo = unsafe {&mut *(virt as *mut u64)};
                *letsgo = val as u64;
                Ok(())
            },
            _ => {
                Err(uacpi::Status::InvalidArgument)
            }
        }
    }
    fn release_mutex(&self, mutex: uacpi::Handle) {
        
    }
    fn release_spinlock(&self, lock: uacpi::Handle, cpu_flags: uacpi::CpuFlags) {
        
    }
    fn reset_event(&self, event: uacpi::Handle) {
        
    }
    fn schedule_work(&self, work_type: uacpi::WorkType, handler: Box<dyn Fn()>) -> Result<(), uacpi::Status> {
        Err(uacpi::Status::Unimplemented)
    }
    fn signal_event(&self, event: uacpi::Handle) {
        
    }
    fn sleep(&self, msec: u8) {
        
    }
    fn stall(&self, usec: u8) {
        
    }
    fn uninstall_interrupt_handler(&self, handle: uacpi::Handle) -> Result<(), uacpi::Status> {
        Ok(())
    }
    unsafe fn unmap(&self, addr: *mut core::ffi::c_void, len: usize) {
        
    }
    fn wait_for_event(&self, event: uacpi::Handle, timeout: u16) -> bool {
        true
    }
    fn wait_for_work_completion(&self) -> Result<(), uacpi::Status> {
        Ok(())
    }
    
}
pub fn init_acpi() {
    use alloc::sync::Arc;
    uacpi::kernel_api::set_kernel_api(Arc::new(KTUACPIAPI));
    let st = uacpi::init(PhysAddr::new(rsdp.get_response().unwrap().address() as u64 - HHDM.get_response().unwrap().offset() as u64), LogLevel::TRACE, false);
    st.unwrap();
    let st = uacpi::namespace_load();
    st.unwrap();
    let st = uacpi::namespace_initialize();
    println!("Succeded !");
    
}