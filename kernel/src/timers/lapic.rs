use crate::{mem::HHDM, println, rdmsr, timers::hpet::thehpet};

use super::hpet::HPET;

fn write_lapic_register(lapic_addr: u64, reg: u64, val: u32) {
    unsafe {
        core::ptr::write_volatile((lapic_addr + reg) as *mut u32, val);
    }
}
fn read_lapic_register(lapic_addr: u64, reg: u64) -> u32 {
    unsafe {
        return core::ptr::read_volatile((lapic_addr + reg) as *mut u32) as u32;
    }
}
fn send_lapic_eoi(lapic_addr: u64) {
    unsafe { core::ptr::write_volatile((lapic_addr + 0xb0) as *mut u32, 0) };
}
fn read_lapic_id(lapic_addr: u64) -> u32 {
    unsafe { return core::ptr::read_volatile((lapic_addr + 0x20) as *mut u32) };
}
fn get_lapic_addr() -> u64 {
    let addr = rdmsr(0x1b);

    return (addr & 0xfffff000) + HHDM.get_response().unwrap().offset();
}
pub fn init_lapic() {
    unsafe {
        core::arch::asm!(
            "mov rax, {0}",
            "mov cr8, rax",
            const 0,
            out("rax") _,
        );
    }
    let a = get_lapic_addr();
    // 0x100 enables the interrupt, 33 is the interrupt number for a surprious interrupt
    write_lapic_register(a, 0xf0, 0x100 | 33);
    // divide by 4
    write_lapic_register(a, 0x3e0, 1);
    // one shot timer, unmasked on interrupt 34
    // 1 << 16 masks the timer
    write_lapic_register(a, 0x320, 34 | (1 << 16));
    // calibrate the lapic, set the inital count
    write_lapic_register(a, 0x380, 0xffffffff);
    thehpet.get().unwrap().ksleep(10);
    let mut lapic_ticks_per_10ms = read_lapic_register(a, 0x390);
    lapic_ticks_per_10ms = 0xffffffff - lapic_ticks_per_10ms;
    println!("lapic ticks per 10 ms {}", lapic_ticks_per_10ms);
    write_lapic_register(a, 0x380, lapic_ticks_per_10ms);
    // unmasked, periodic, interrupt 34
    // (1 << 17) sets to periodic
    // read sdm for more info
    write_lapic_register(a, 0x320, 34 | (0 << 16) | (1 << 17));

    unsafe { core::arch::asm!("cli") };
}
