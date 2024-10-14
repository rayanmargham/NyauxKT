use alloc::string::{String, ToString};
use core::arch::naked_asm;
use core::ops::Deref;
use spin::Mutex;

use crate::elf::symbol::{self, symbol_table};
use crate::println;
use crate::sched::schedule_task;
use crate::timers::lapic::{self, get_lapic_addr};
#[repr(C)]
#[derive(Clone, Copy)]
struct GateDescriptor {
    offset: u16,
    seg: u16,
    ist_and_reversed: u8,
    flags: u8,
    offset_mid: u16,
    offset_hi: u32,
    reversed: u32,
}
impl GateDescriptor {
    const fn new() -> Self {
        return Self {
            offset: 0,
            seg: 0,
            ist_and_reversed: 0,
            flags: 0,
            offset_mid: 0,
            offset_hi: 0,
            reversed: 0,
        };
    }
}
extern "C" fn exception_handler(registers: u64) {
    let got_registers = unsafe { &*(registers as *mut Registers) };
    println!("--Register Dump---\nCR2={:#x}    RFLAGS={:#x}\n--End Of Register Dump\n\n---Stack Trace---", read_cr2(), got_registers.rflags);
    let mut base_pointer: *mut usize =
        core::ptr::with_exposed_provenance_mut::<usize>(got_registers.rip);
    let n = "No Function :(".to_string();
    let g = get_formatted_string_from_rip(base_pointer.addr()).unwrap_or((base_pointer.addr(), &n));
    println!("call site: {:#x} -- function: {}", g.0, g.1);
    base_pointer = base_pointer.with_addr(got_registers.rbp);
    while !base_pointer.is_null() {
        let addr = unsafe { (*(base_pointer.offset(1))) };
        let n = "No Function :(".to_string();
        let g = get_formatted_string_from_rip(addr).unwrap_or((addr, &n));
        println!("call site: {:#x} -- function: {}", g.0, g.1);
        base_pointer = unsafe { (*base_pointer) as *mut usize };
    }
    println!("---Stack Trace---");
    panic!("oops");
}
extern "C" fn sched(registers: u64) -> usize {
    let got_registers =
        unsafe { &*(core::ptr::with_exposed_provenance_mut::<Registers>(registers as usize)) };

    lapic::send_lapic_eoi(get_lapic_addr());
    if let Some(mut r) = schedule_task(*got_registers) {
        return (&mut r as *mut Registers).addr();
    }
    return core::ptr::with_exposed_provenance_mut::<Registers>(registers as usize).addr();
}
pub fn read_cr2() -> usize {
    let val: usize;
    unsafe {
        core::arch::asm!(
            "mov {}, cr2",
            out(reg) val
        )
    };
    val
}
pub fn get_formatted_string_from_rip<'a>(rip: usize) -> Option<(usize, &'a String)> {
    if let Some(sym) = symbol_table.get() {
        let mut r = sym.range(..rip);
        if let Some((idx, y)) = r.next_back() {
            return Some((*idx, y));
        }
    }
    None
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Registers {
    // Pushed by wrapper
    pub int: usize,

    // Pushed by push_gprs in crate::arch::x86_64
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,
    pub rbp: usize,

    // Pushed by interrupt
    pub error_code: usize,
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
}
static IDT: Mutex<[GateDescriptor; 256]> = Mutex::new([GateDescriptor::new(); 256]);
pub fn idt_set_gate(num: u8, function_ptr: usize) {
    let base = function_ptr;

    IDT.lock()[num as usize] = GateDescriptor {
        offset: (base & 0xFFFF) as u16,
        offset_mid: ((base >> 16) & 0xFFFF) as u16,
        offset_hi: ((base >> 32) & 0xFFFFFFFF) as u32,
        seg: 0x28,
        ist_and_reversed: 0,
        reversed: 0,
        flags: 0xEE,
    };
}
#[repr(C, packed)]
#[derive(Debug)]
struct IDTR {
    size: u16,
    offset: u64,
}
#[macro_export]
macro_rules! push_gprs {
    () => {
        "push rbp"
    };
}
macro_rules! exception_function {
    ($code:expr, $handler:ident) => {

        #[naked]
        #[no_mangle]
        extern "C" fn $handler() {

            unsafe {
                naked_asm!(

                    "push rbp",
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",
                    "push {0}",
                    "mov rdi, rsp",
                    "call {1}",
                    "add rsp, 8",

                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
                    "pop rbp",
                    "add rsp, 8",
                    "iretq",
                    const $code,
                    sym exception_handler
                );
            };




        }
    };
}
macro_rules! exception_function_no_error {
    ($code:expr, $handler:ident, $meow:ident) => {

        #[naked]
        #[no_mangle]
        extern "C" fn $handler() {

            unsafe {
                naked_asm!(

                    "push 0x0",
                    "push rbp",
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",
                    "push {0}\n",
                    "mov rdi, rsp",
                    "call {1}",
                    "add rsp, 8",

                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
                    "pop rbp",
                    "add rsp, 8",
                    "iretq",
                    const $code,
                    sym $meow
                );
            };




        }
    };
}
macro_rules! exception_function_no_error_sched {
    ($code:expr, $handler:ident, $meow:ident) => {

        #[naked]
        #[no_mangle]
        extern "C" fn $handler() {

            unsafe {
                naked_asm!(
                    "push 0x0",
                    "cmp qword ptr [rsp + 16], 0x43",
                    "jne 2f",
                    "swapgs",
                    "2:",
                    "push rbp",
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push r8",
                    "push r9",
                    "push r10",
                    "push r11",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",
                    "push {0}",
                    "mov rdi, rsp",
                    "call {1}",
                    "mov rsp, rax",
                    "add rsp, 8",

                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop r11",
                    "pop r10",
                    "pop r9",
                    "pop r8",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
                    "pop rbp",
                    "add rsp, 8",
                    "cmp qword ptr [rsp + 16], 0x43",
                    "jne 3f",
                    "swapgs",
                    "3:",
                    "iretq",


                    const $code,
                    sym $meow
                );
            };




        }
    };
}
exception_function_no_error!(0x00, div_error, exception_handler);
exception_function_no_error!(0x06, invalid_opcode, exception_handler);
exception_function!(0x08, double_fault);
exception_function!(0x0D, general_protection_fault);
exception_function!(0x0E, page_fault);
exception_function_no_error_sched!(34, schede, sched);
static IDTR: Mutex<IDTR> = Mutex::new(IDTR { offset: 0, size: 0 });
pub struct InterruptManager {}
impl InterruptManager {
    pub fn start_idt() {
        let x = push_gprs!();

        idt_set_gate(0x00, div_error as usize);
        idt_set_gate(0x06, invalid_opcode as usize);
        idt_set_gate(0x08, double_fault as usize);
        idt_set_gate(0x0D, general_protection_fault as usize);
        idt_set_gate(0x0E, page_fault as usize);
        idt_set_gate(34, schede as usize);

        // idt_set_gate(47, haha as usize);
        IDTR.lock().offset = IDT.lock().as_ptr() as u64;

        IDTR.lock().size = ((core::mem::size_of::<GateDescriptor>() * 256) - 1) as u16;

        unsafe {
            let s = IDTR.lock();
            let h = s.deref();
            core::arch::asm!(
                "lidt [{}]",
                in(reg) core::ptr::addr_of!(*s)
            );
        }
    }
}
