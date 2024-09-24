use spin::Mutex;

use limine::{
    memory_map::EntryType,
    request::{HhdmRequest, MemoryMapRequest},
};
use owo_colors::OwoColorize;

use crate::println;

#[derive(Debug)]
pub struct KTNode {
    next: Option<*mut KTNode>,
}

pub struct PhysicalAllocator {
    head: Option<*mut KTNode>,
}
#[repr(C)]
#[derive(PartialEq)]
struct slab_header {
    size: usize,
    next_slab: Option<*mut slab_header>,
    freelist: Option<*mut KTNode>,
}
#[derive(PartialEq)]
struct Cache {
    slabs: Option<*mut slab_header>,
    size: usize,
}
pub struct KmallocManager {
    array: [Cache; 7],
}
pub static mut RAMUSAGE: u64 = 0;
impl KmallocManager {
    fn init() -> Self {
        println!("Creating Objects...");
        let cache1 = Cache::init(16);
        let cache2 = Cache::init(32);
        let cache3 = Cache::init(64);
        let cache4 = Cache::init(128);
        let cache5 = Cache::init(256);
        let cache6 = Cache::init(512);
        let cache7 = Cache::init(1024);
        Self {
            array: [cache1, cache2, cache3, cache4, cache5, cache6, cache7],
        }
    }
    pub fn free(&mut self, addr: u64) {
        if addr == 0 {
            return;
        }
        let h = (addr & !0xFFF) as *mut slab_header;
        let mut rightCache = None;
        'outer: for i in self.array.iter_mut() {
            unsafe {
                if i.size == (*h).size {
                    rightCache = Some(i);
                    break 'outer;
                }
            }
        }
        if rightCache == None {
            return;
        }
        let new = addr as *mut KTNode;

        unsafe { new.write_bytes(0, 1) };
        unsafe {
            let ok = rightCache.unwrap();
            (*new).next = (*h).freelist;
            (*h).freelist = Some(new);
            RAMUSAGE -= ok.size as u64;
            let mut prev = None;
            let mut shit = ok.slabs;
            while shit != None {
                if (*shit.unwrap()) == *h {
                    return;
                } else {
                    prev = shit;
                    shit = (*shit.unwrap()).next_slab;
                }
            }
            (*prev.unwrap()).next_slab = Some(h);
            return;
        }
    }
    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        let a = size.next_power_of_two();
        for i in self.array.iter_mut() {
            if i.size >= a {
                return i.slab_allocsearch();
            }
        }

        None
    }
}
#[used]
#[link_section = ".requests"]
pub static HDDM_OFFSET: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
pub static MEMMAP: Mutex<MemoryMapRequest> = Mutex::new(MemoryMapRequest::new());
/// stolen troll
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}
// THIS IS FINE BECAUSE THE ALLOCATOR IS BEHIND A MUTEX,
// AS LONG AS IT IS BEHIND AM UTEX IT IS OKAY AND SHALL REMAIN UNCHANGED
// I HAVE TESTED THE ALLOCATOR A BUNCH AND LLVM DOESNT DO ANY UB ON THIS IMPLMENTATION SO ITS FINE
// PLEASE IGNORE THIS CODE AS THIS WILL BE THE ONLY PART OF THE KERNEL THAT IS DISGUSTING WITH THE STATIC MUT
unsafe impl Sync for PhysicalAllocator {}
unsafe impl Send for PhysicalAllocator {}
unsafe impl Sync for KmallocManager {}
unsafe impl Send for KmallocManager {}
pub static PMM: Mutex<PhysicalAllocator> = Mutex::new(PhysicalAllocator { head: None });
pub static KmallocManager: Mutex<Option<KmallocManager>> = Mutex::new(None);

impl PhysicalAllocator {
    pub fn new() -> Result<(), &'static str> {
        println!("{}", "--Memory MAP--".bright_blue());
        let unlocked = MEMMAP.lock();
        let entries = unlocked.get_response().as_ref().unwrap().entries();

        let mut new = PhysicalAllocator { head: None };
        let mut last = None;
        for i in entries.iter() {
            match i.entry_type {
                EntryType::USABLE => {
                    let page_amount = align_up(i.length as usize, 4096) / 4096;
                    for e in 0..page_amount {
                        unsafe {
                            let node: *mut KTNode = ((i.base + (e as u64 * 4096))
                                + HDDM_OFFSET.get_response().unwrap().offset())
                                as *mut KTNode;
                            (*node).next = last;
                            last = Some(node);
                        }
                    }

                    println!(
                        "Created Freelist Node of Base {:#x} and Page Count {}",
                        i.base.yellow(),
                        (align_up(i.length as usize, 4096) / 4096).green()
                    );
                }
                _ => {}
            }
        }
        new.head = last;

        *PMM.lock() = new;
        *KmallocManager.lock() = Some(KmallocManager::init());
        return Ok(());
    }
    pub fn alloc(&mut self) -> Result<*mut u8, &str> {
        let w = self.head.unwrap();
        'outer: loop {
            match unsafe { (*w).next } {
                Some(e) => {
                    self.head = Some(e);
                    unsafe { RAMUSAGE += 4096 as u64 };
                    return Ok((unsafe {
                        (w as *mut u8).sub(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                    }));
                }
                None => {
                    break 'outer;
                }
            }
        }
        println!("Reached end");
        return Err("no memory");
    }
    pub fn dealloc(&mut self, addr: *mut u8) -> Result<(), &str> {
        let w = self.head.unwrap();
        let e = align_down(addr as usize, 4096);

        let node: *mut KTNode = e as *mut KTNode;
        unsafe {
            (*node).next = self.head;
            self.head = Some(node);
        }
        unsafe { RAMUSAGE -= 4096 as u64 };
        Ok(())
    }
}

impl slab_header {
    fn init(size: usize) -> *mut Self {
        let mut area: *mut u64 = PMM.lock().alloc().unwrap() as *mut u64;
        area = (area as u64 + HDDM_OFFSET.get_response().unwrap().offset()) as *mut u64;
        unsafe { area.write_bytes(0, 4096 / 8) };
        let header = (area) as *mut slab_header;

        unsafe {
            header.write_bytes(0, 1);
            (*header).size = size;

            let obj_amount = (4096 - size_of::<slab_header>()) / size;
            let start = (header as u64 + size_of::<slab_header>() as u64) as *mut KTNode;
            println!("objection ammount: {obj_amount}");

            (*header).freelist = Some(start);
            start.write_bytes(0, 1);
            (*start).next = None;
            let mut prev = start;

            for i in 1..obj_amount {
                let new = (start as u64 + (i as u64 * size as u64)) as *mut KTNode;
                new.write(KTNode { next: None });
                (*new).next = None;

                (*prev).next = Some(new);

                prev = new;
            }

            (*prev).next = None;
        }
        return header;
    }
}
impl Cache {
    fn init(size: usize) -> Self {
        let new = slab_header::init(size);
        println!("Created Cache of size: {size}");
        Self {
            size: size,
            slabs: Some(new),
        }
    }
    fn slab_allocsearch(&mut self) -> Option<*mut u8> {
        let mut h = self.slabs;
        'outer: while h.is_none() == false {
            unsafe {
                if (*h.unwrap()).freelist.is_some() {
                    let new = (*h.unwrap()).freelist.unwrap();

                    (*h.unwrap()).freelist = (*new).next;
                    RAMUSAGE += self.size as u64;
                    return Some(new as *mut u8);
                } else {
                    if (*h.unwrap()).next_slab.is_none() {
                        break 'outer;
                    }
                    h = (*h.unwrap()).next_slab;
                }
            }
        }
        // make new slab for Cache since theres no more space
        let new = slab_header::init(self.size);
        unsafe {
            (*h.unwrap()).next_slab = Some(new);
            let o = (*new).freelist.unwrap();
            (*new).freelist = (*o).next;
            RAMUSAGE += self.size as u64;
            return Some(o as *mut u8);
        }
    }
}
