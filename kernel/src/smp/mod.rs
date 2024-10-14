use limine::*;
use owo_colors::OwoColorize;
use request::SmpRequest;

use crate::{
    gdt::init_gdt,
    idt, println,
    sched::{create_kentry, sched_init},
    timers::lapic::init_lapic,
};

#[used]
#[link_section = ".requests"]
static liminecpu: SmpRequest = SmpRequest::new();
unsafe extern "C" fn bootstrapper(e: &limine::smp::Cpu) -> ! {
    println!("CPU {} is {}", e.id, "Online!".bright_green());
    init_gdt();
    idt::InterruptManager::start_idt();
    init_lapic();
    sched_init();
    core::arch::asm!("sti");
    loop {
        core::arch::asm!("hlt");
    }
}

pub fn bootstrap() {
    let response = liminecpu.get_response().unwrap();
    println!("---SMP---");
    for i in response.cpus() {
        if i.lapic_id == response.bsp_lapic_id() {
            continue;
        }
        println!("CPU: {}", i.lapic_id);
        i.goto_address.write(bootstrapper);
    }
    println!("---SMP---");
    init_lapic();
    unsafe { sched_init() };
    create_kentry();
    unsafe {
        core::arch::asm!("sti");
        loop {
            core::arch::asm!("hlt");
        }
    }
}
