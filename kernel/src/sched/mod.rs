use core::sync::atomic::AtomicPtr;

use alloc::{
    boxed::Box,
    collections::vec_deque::VecDeque,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

use crate::{
    idt::Registers,
    mem::{
        pmm::pmm_alloc,
        vmm::{self, PageMap, VMMFlags, KERMAP},
        HHDM,
    },
    println, rdmsr, wrmsr,
};

enum KTHandle {}

#[repr(C, packed)]
struct fpu_state {
    fcw: u16,
    rev0: u32,
    rev: u16,
    rev2: u64,
    rev3: u64,
    mxcsr: u32,
}
pub struct cpu_ctx {
    frame: Registers,
    // fpu state would be here but whatevs
}
impl cpu_ctx {
    fn new(entry: usize, usermode: bool, rsp: *mut u8) -> Self {
        if usermode {
            let o = Registers {
                int: 0,
                r10: 0,
                r11: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
                r8: 0,
                r9: 0,
                rax: 0,
                rbp: 0,
                rbx: 0,
                rcx: 0,
                rdi: 0,
                rdx: 0,
                rip: entry as usize,
                rsi: 0,
                error_code: 0,
                cs: 0x40 | (3),
                ss: 0x38 | (3),
                rflags: 0x202,
                rsp: rsp.addr(),
            };
            Self { frame: o }
        } else {
            let o = Registers {
                int: 0,
                r10: 0,
                r11: 0,
                r12: 0,
                r13: 0,
                r14: 0,
                r15: 0,
                r8: 0,
                r9: 0,
                rax: 0,
                rbp: 0,
                rbx: 0,
                rcx: 0,
                rdi: 0,
                rdx: 0,
                rip: entry as usize,
                rsi: 0,
                error_code: 0,
                cs: 0x28,
                ss: 0x30,
                rflags: 0x202,
                rsp: rsp.addr(),
            };
            Self { frame: o }
        }
    }
}
struct process<'a> {
    pagemap: &'a mut PageMap,
}
#[repr(C, align(8))]
pub struct perthreadcpu<'a> {
    kernal_stack_ptr: AtomicPtr<u8>,
    user_stack_ptr: Option<*mut u8>,
    run_queue: Option<*mut Thread<'a>>,  // head of run queue
    cur_thread: Option<*mut Thread<'a>>, // cur_thread atm
}
#[no_mangle]
extern "C" fn lol() -> ! {
    let mut o = 5;
    loop {
        println!("hi {}", o);
        o += 1;
    }
}
// this is called per cpu :)
pub unsafe fn sched_init() {
    let new_kstack = KERMAP
        .lock()
        .as_mut()
        .unwrap()
        .vmm_region_alloc(65536, VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED)
        .unwrap()
        .add(65536);
    let mut pp = Box::new(perthreadcpu {
        kernal_stack_ptr: AtomicPtr::new(new_kstack),
        user_stack_ptr: None,
        run_queue: None,
        cur_thread: None,
    });
    wrmsr(0xC0000101, Box::into_raw(pp).addr() as u64);
}
pub fn create_kentry() {
    // on the buttstrap cpu
    let him = rdmsr(0xC0000101);
    let bro = core::ptr::with_exposed_provenance_mut(him as usize) as *mut perthreadcpu;
    let new_ctx = cpu_ctx::new(lol as usize, false, unsafe {
        (*bro)
            .kernal_stack_ptr
            .load(core::sync::atomic::Ordering::SeqCst)
    });
    let new_man = Thread::new("MY MAN", 0, 0, 0, new_ctx);
    unsafe {
        (*bro).run_queue = Some(Box::into_raw(Box::new(new_man)));
    }
}
struct Thread<'a> {
    name: String,
    tid: usize,
    gs_base: usize,
    fs: usize,
    content: cpu_ctx,
    next: Option<*mut Thread<'a>>,
    process: Option<Arc<Mutex<process<'a>>>>,
}
impl<'a> Thread<'a> {
    fn new(name: &str, tid: usize, gs_base: usize, fs: usize, context: cpu_ctx) -> Self {
        Self {
            name: name.to_string(),
            tid,
            gs_base,
            fs,
            content: context,
            next: None,
            process: None,
        }
    }
}
#[no_mangle]
pub fn schedule_task(regs: Registers) -> Option<Registers> {
    if regs.cs == 0x40 | (3) {
        panic!("usermode not ready lol");
    } else {
        let him = rdmsr(0xC0000101);
        let bro = core::ptr::with_exposed_provenance_mut(him as usize) as *mut perthreadcpu;
        unsafe {
            if (*bro).cur_thread.is_some() {
                // save our ctx :)
                (*(*bro).cur_thread.unwrap()).content.frame = regs;
                if (*(*bro).cur_thread.unwrap()).next.is_some() {
                    ((*bro).cur_thread) = Some((*(*bro).cur_thread.unwrap()).next.unwrap());
                } else {
                    if (*bro).run_queue.is_some() {
                        (*bro).cur_thread = Some((*bro).run_queue.unwrap())
                    } else {
                        return None;
                    }
                }
            } else if (*bro).run_queue.is_some() {
                (*bro).cur_thread = Some((*bro).run_queue.unwrap())
            } else {
                return None;
            }
            // switch our regs
            let our = (*(*bro).cur_thread.unwrap()).content.frame;
            if (*(*bro).cur_thread.unwrap()).process.is_some() {
                (*(*bro).cur_thread.unwrap())
                    .process
                    .as_ref()
                    .unwrap()
                    .lock()
                    .pagemap
                    .switch_to();
            }
            if our.cs == 0x40 | (3) {
                panic!("nope");
            }
            return Some(our);
        }
    }
}
