use core::ops::Deref;
use core::arch::naked_asm;
use spin::Mutex;
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
    panic!(
        "crash vec {:#x} \nwith register rip at {:#x}\nerror code {:#x} \nrflags {:#x} idiot",
        got_registers.int, got_registers.rip, got_registers.error_code, got_registers.rflags
    );
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
    pub rbp: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,

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
macro_rules! exception_function {
    ($code:expr, $handler:ident) => {

        #[naked]
        #[no_mangle]
        extern "C" fn $handler() {

            unsafe {
                naked_asm!(
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push rbp",
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
                    "pop rbp",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
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
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push rbp",
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
                    "pop rbp",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
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
                    "push rax",
                    "push rbx",
                    "push rcx",
                    "push rdx",
                    "push rsi",
                    "push rdi",
                    "push rbp",
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
                    "pop rbp",
                    "pop rdi",
                    "pop rsi",
                    "pop rdx",
                    "pop rcx",
                    "pop rbx",
                    "pop rax",
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
static IDTR: Mutex<IDTR> = Mutex::new(IDTR { offset: 0, size: 0 });
pub struct InterruptManager {}
impl InterruptManager {
    pub fn start_idt() {
        idt_set_gate(0x00, div_error as usize);
        idt_set_gate(0x06, invalid_opcode as usize);
        idt_set_gate(0x08, double_fault as usize);
        idt_set_gate(0x0D, general_protection_fault as usize);
        idt_set_gate(0x0E, page_fault as usize);

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

