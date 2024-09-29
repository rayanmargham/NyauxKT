use crate::println;

use super::align_up;
use super::HHDM;
use super::MEMMAP;
use core::ops::Deref;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use spin::mutex::Mutex;
#[derive(Debug, PartialEq)]
pub struct KTNode {
    next: Option<&'static mut KTNode>,
}
pub struct kmallocmanager {
    array: [cache; 7],
}
impl kmallocmanager {
    fn init() -> Self {
        println!("creating that shit");
        let cache1 = cache::init(16);
        let cache2 = cache::init(32);
        let cache3 = cache::init(64);
        let cache4 = cache::init(128);
        let cache5 = cache::init(256);
        let cache6 = cache::init(512);
        let cache7 = cache::init(1024);

        Self {
            array: [cache1, cache2, cache3, cache4, cache5, cache6, cache7],
        }
    }
    pub fn alloc(&mut self, size: usize) -> Option<usize> {
        let a = size.next_power_of_two();
        for i in self.array.iter_mut() {
            if i.size >= a {
                return i.slab_allocsearch();
            }
        }
        None
    }
    pub fn free(&mut self, addr: usize) {
        if addr == 0 {
            return;
        }
        let header = unsafe { &mut *((addr & !0xFFF) as *mut slab_header) };
        let mut rightcache = None;
        'outer: for i in self.array.iter_mut() {
            if i.size == header.size {
                rightcache = Some(i);
                break 'outer;
            }
        }
        if rightcache == None {
            return;
        }
        let him = unsafe {
            &mut *({
                let j = addr as *mut u8;
                j.write_bytes(0, 4096);
                j as *mut KTNode
            })
        };
        let ok = rightcache.unwrap();
        him.next = header.freelist.take();
        header.freelist = Some(him);
        let mut prev = None;
        let mut shit = ok.slabs.take();
        while shit != None {
            if shit.as_ref().unwrap().deref() == header {
                return;
            } else {
                let temp = shit.as_mut().unwrap().next_slab.take();
                prev = shit;
                shit = temp;
            }
        }
        if let Some(jk) = prev {
            jk.next_slab = Some(header);
        }
    }
}
static HEAD: Mutex<Option<&mut KTNode>> = Mutex::new(None);
pub static cool: Mutex<Option<kmallocmanager>> = Mutex::new(None);
pub static FREEPAGES: AtomicUsize = AtomicUsize::new(0);
pub fn pmm_init() {
    let mut head = HEAD.lock();
    let o = unsafe { MEMMAP.get_response().unwrap().entries() };
    let mut last = None;
    let jk = HHDM.get_response().unwrap().offset();
    for entry in o
        .iter()
        .filter(|x| x.entry_type == limine::memory_map::EntryType::USABLE)
    {
        let amount = align_up(entry.length as usize, 4096);
        for i in (0..amount).step_by(4096) {
            let node: &mut KTNode =
                unsafe { &mut *((entry.base as usize + (i) + jk as usize) as *mut KTNode) };
            node.next = last;
            last = Some(node);
            FREEPAGES.fetch_add(1, Ordering::SeqCst);
        }
    }
    *head = last;
    println!(
        "pmm_init(): Free Pages: {}",
        FREEPAGES.load(Ordering::SeqCst)
    );
    drop(head);
    let mut ok = cool.lock();
    *ok = Some(kmallocmanager::init())
}

pub fn pmm_alloc() -> Option<usize> {
    let mut mutd = HEAD.lock();
    let head = mutd.as_mut()?;
    let next_node = head.next.take();
    let it = unsafe {
        (*head as *mut KTNode)
            .sub(HHDM.get_response().unwrap().offset() as usize / size_of::<KTNode>())
            as usize
    };
    FREEPAGES.fetch_sub(1, Ordering::SeqCst);
    *mutd = next_node;

    Some(it)
}
pub fn pmm_dealloc(addr: usize) -> Option<()> {
    let mut mutd = HEAD.lock();
    let head = mutd.as_mut()?;
    let node = head.next.take();
    let created = unsafe {
        &mut *((addr as *mut KTNode)
            .add(HHDM.get_response().unwrap().offset() as usize / size_of::<KTNode>()))
    };
    created.next = node;
    *mutd = Some(created);
    FREEPAGES.fetch_add(1, Ordering::SeqCst);
    Some(())
}
#[derive(Debug, PartialEq)]
struct cache {
    size: usize,
    slabs: Option<&'static mut slab_header>,
}
#[derive(Debug, PartialEq)]
#[repr(C)]
struct slab_header {
    size: usize,
    next_slab: Option<&'static mut slab_header>,
    freelist: Option<&'static mut KTNode>,
}
pub struct kmallocmgr {
    array: [cache; 7],
}
impl slab_header {
    fn init(size: usize) -> &'static mut Self {
        println!("h");
        let data = unsafe { (pmm_alloc().unwrap() as *mut u8) };
        println!("h, {:#x}", data as usize);
        unsafe {
            data.add(HHDM.get_response().unwrap().offset() as usize)
                .write_bytes(0, 4096);
        }
        println!("h");
        let h = unsafe {
            &mut (*(data.add(HHDM.get_response().unwrap().offset() as usize) as *mut slab_header))
        };
        h.size = size;

        let obj_amount = (4096 - size_of::<slab_header>()) / size;
        let mut start = unsafe {
            &mut *((data.add(HHDM.get_response().unwrap().offset() as usize) as usize
                + size_of::<slab_header>()) as *mut KTNode)
        };
        let heraddr = (start as *mut KTNode) as usize;

        let mut prev = &mut start;

        for i in 1..obj_amount {
            let new = unsafe { &mut *((heraddr + i * size) as *mut KTNode) };
            new.next = None;
            prev.next = Some(new);
            prev = prev.next.as_mut().unwrap();
        }
        prev.next = None;
        h.freelist = Some(start);

        return h;
    }
}
impl cache {
    fn init(size: usize) -> Self {
        let new = slab_header::init(size);
        println!("Created Cache of size: {size}");
        Self {
            size: size,
            slabs: Some(new),
        }
    }
    fn slab_allocsearch(&mut self) -> Option<usize> {
        let mut h = &mut self.slabs;
        'outer: while h.is_none() == false {
            if let Some(fre) = h.as_mut().unwrap().freelist.take() {
                h.as_mut().unwrap().freelist = fre.next.take();
                return Some(fre as *mut KTNode as usize);
            } else {
                if h.as_ref().unwrap().next_slab.is_none() {
                    break 'outer;
                }
                h = &mut h.as_mut().unwrap().next_slab;
            }
        }
        let new = slab_header::init(self.size);

        let o = h.as_mut().unwrap().next_slab.take();
        if let Some(g) = o {
            new.next_slab = Some(g);
            h.as_mut().unwrap().next_slab = Some(new);
        } else {
            h.as_mut().unwrap().next_slab = Some(new);
        }
        let gethim = h.as_mut().unwrap().next_slab.take().unwrap();
        println!("{:#?}", gethim.freelist);
        gethim.freelist = gethim.freelist.as_mut().unwrap().next.take();
        let gg = gethim.freelist.as_mut().take().unwrap();
        return Some(*gg as *mut KTNode as usize);
    }
}
