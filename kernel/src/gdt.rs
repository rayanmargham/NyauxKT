use core::arch::global_asm;
use core::ffi::c_void;
use core::include_str;
use core::mem::size_of_val;
use core::ops::Deref;
use core::ptr::addr_of;
use spin::Mutex;
global_asm!(include_str!("flush.s"));
#[repr(C, packed)]

struct GDTR {
    size: u16,
    offset: u64,
}
impl GDTR {
    const fn new(table: u64, size: u16) -> GDTR {
        GDTR {
            size: size - 1,
            offset: table,
        }
    }
}

extern "C" {
    fn gdt_flush(a: *const c_void);
}
static REALGDT: [u64; 9] = [
    0x0,
    0x00009a000000ffff,
    0x000093000000ffff,
    0x00cf9a000000ffff,
    0x00cf93000000ffff,
    0x00af9b000000ffff,
    0x00af93000000ffff,
    0x00aff3000000ffff,
    0x00affb000000ffff,
];
static GDT: Mutex<GDTR> = Mutex::new(GDTR::new(0, 1));
pub fn init_gdt() {
    GDT.lock().offset = addr_of!(REALGDT) as u64;
    GDT.lock().size = size_of_val(&REALGDT) as u16;

    unsafe {
        let h = GDT.lock();
        let j = h.deref();
        gdt_flush(addr_of!(*j) as *const _);
    }
}
