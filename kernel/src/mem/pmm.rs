use crate::println;

use super::align_up;
use super::HHDM;
use super::MEMMAP;
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use spin::mutex::Mutex;
#[derive(Debug, PartialEq)]
pub struct KTNode {
    next: Option<&'static mut KTNode>,
}
pub struct um {
    next: holder_type
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
    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        let a = size.next_power_of_two();
        for i in self.array.iter_mut() {
            if i.size >= a {
                return i.slab_allocsearch();
            }
        }
        None
    }
    pub fn free(&mut self, addr: *mut u8) {
        
        if addr == core::ptr::null_mut() {
            return;
        }
        
        let header = NonNull::new(addr.map_addr(|a| a & !0xFFF).cast::<slab_header>()).unwrap();
        let mut rightcache = None;
        'outer: for i in self.array.iter_mut() {
            if i.size == unsafe {(*header.as_ptr()).obj_size} {
                rightcache = Some(i);
                break 'outer;
            }
        }
        if rightcache.is_none() {
            return;
        }
        rightcache.unwrap().free(addr);
        
    }
    //     let header = unsafe { &mut *((addr & !0xFFF) as *mut slab_header) };
    //     let mut rightcache = None;
    //     'outer: for i in self.array.iter_mut() {
    //         if i.size == header.size {
    //             rightcache = Some(i);
    //             break 'outer;
    //         }
    //     }
    //     if rightcache == None {
    //         return;
    //     }
    //     let him = unsafe {
    //         &mut *({
    //             let j = addr as *mut u8;
    //             j.write_bytes(0, 4096);
    //             j as *mut KTNode
    //         })
    //     };
    //     let ok = rightcache.unwrap();
    //     him.next = header.freelist.take();
    //     header.freelist = Some(him);
    //     let mut prev = None;
    //     let mut shit = ok.slabs.take();
    //     while shit != None {
    //         if shit.as_ref().unwrap().deref() == header {
    //             return;
    //         } else {
    //             let temp = shit.as_mut().unwrap().next_slab.take();
    //             prev = shit;
    //             shit = temp;
    //         }
    //     }
    //     if let Some(jk) = prev {
    //         jk.next_slab = Some(header);
    //     }
    // }
}

use alloc::sync::Arc;
unsafe impl Sync for kmallocmanager {}
unsafe impl Send for kmallocmanager {}
static HEAD: Mutex<Option<&mut KTNode>> = Mutex::new(None);

pub static cool: Mutex<Option<kmallocmanager>> = Mutex::new(None);

pub static FREEPAGES: AtomicUsize = AtomicUsize::new(0);
pub fn pmm_init() {
    let mut head = HEAD.lock();
    let mut hhh = MEMMAP.lock();
    let o = hhh.get_response_mut().unwrap().entries();
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
    
    *cool.lock() = Some(kmallocmanager::init());
    
    
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
#[derive(Debug)]
pub struct cache {
    size: usize,
    slabs: holder_type2,
}
#[derive(Debug)]
struct holder_type(Option<NonNull<um>>);

#[derive(Debug)]
struct holder_type2(Option<NonNull<slab_header>>);
impl holder_type2 {
    fn from_raw(h: NonNull<slab_header>) -> Self{
        Self(Some(h))
    }
    fn new() -> Self {
        Self(None)
    }
}
impl holder_type {
    fn from_raw(h: NonNull<um>) -> Self{
        Self(Some(h))
    }
    fn new() -> Self {
        Self(None)
    }
}


#[derive(Debug)]
#[repr(C)]
struct slab_header {
    obj_size: usize,
    next_slab: holder_type2,
    freelist: holder_type,
}

#[derive(Debug)]
struct LinkedListIter2<'a>(&'a mut holder_type2);
#[derive(Debug)]
struct LinkedListIter<'a>(&'a mut holder_type);
impl<'a> IntoIterator for &'a mut holder_type2 {
    type IntoIter = LinkedListIter2<'a>;
    type Item = &'a mut slab_header;
    fn into_iter(self) -> Self::IntoIter {
        LinkedListIter2(self)
    }
}
impl<'a> Iterator for LinkedListIter2<'a> {
    type Item = &'a mut slab_header;
    fn next(&mut self) -> Option<Self::Item> {
       
        unsafe {
            // no idea how to impl this
            self.0.0.and_then(|s|
                {
                    self.0 = 
                        &mut (*s.as_ptr()).next_slab
                    ;
                    Some(s.as_ptr().as_mut())
                }
            )?
        }
    }
    
}

impl<'a> IntoIterator for &'a mut holder_type {
    type IntoIter = LinkedListIter<'a>;
    type Item = &'a mut um;
    fn into_iter(self) -> Self::IntoIter {
        LinkedListIter(self)
    }
}
impl<'a> Iterator for LinkedListIter<'a> {
    type Item = &'a mut um;
    fn next(&mut self) -> Option<Self::Item> {
       
        unsafe {
            // no idea how to impl this
            self.0.0.and_then(|s|
                {
                    self.0 = 
                        &mut (*s.as_ptr()).next
                    ;
                    Some(s.as_ptr().as_mut())
                }
            )?
        }
    }
    
}

pub struct kmallocmgr {
    array: [cache; 7],
}
impl slab_header {
    fn init(size: usize) -> NonNull<Self> {
        let data = pmm_alloc().unwrap() as *mut u8;
        unsafe {
            data.add(HHDM.get_response().unwrap().offset() as usize)
            .write_bytes(0, 4096);
        }
        let it = NonNull::new(unsafe {
            data.add(HHDM.get_response().unwrap().offset() as usize)
            as *mut slab_header
        }).unwrap();
        unsafe {
            (*it.as_ptr()).obj_size = size;
        }
        let obj_amount = (4096 - size_of::<slab_header>()) / size;
        let start = unsafe {it.add(1).cast::<um>()};
        let mut prev = start;
        for i in 1..obj_amount {
            unsafe {
                let new = start.byte_add(i * size);
                (*new.as_ptr()).next = holder_type::new();
                (*prev.as_ptr()).next = holder_type::new();
                prev = new;
            }
            
        }
        unsafe {
            (*it.as_ptr()).freelist = holder_type::from_raw(start);
            (*it.as_ptr()).next_slab = holder_type2::new();
        }
        
        return it;
        
        

        
    }
    fn pop(&mut self) -> Option<NonNull<um>> {
        match core::mem::replace(&mut self.freelist.0, None) {
            Some(t) => {
                self.freelist.0 = unsafe {(*t.as_ptr()).next.0};
                
                Some(t)
            },
            None => {
                None
            }
        }
    }
    fn push(&mut self, f: NonNull<um>) {
        match core::mem::replace(&mut self.freelist.0, None) {
            Some(t) => {
                // self.freelist.0 = unsafe {(*t.as_ptr()).next.0};
                unsafe {
                    (*t.as_ptr()).next = holder_type::from_raw(f);
                    self.freelist = holder_type::from_raw(f);
                }
                // Some(t)
            },
            None => {
                self.freelist = holder_type::from_raw(f);
            }}
    }
    
    //     let data = pmm_alloc().unwrap() as *mut u8;
        
    //     unsafe {
    //         data.add(HHDM.get_response().unwrap().offset() as usize)
    //             .write_bytes(0, 4096);
    //     }
        
    //     let h = unsafe {
    //         &mut (*(data.add(HHDM.get_response().unwrap().offset() as usize) as *mut slab_header))
    //     };
    //     h.size = size;

    //     let obj_amount = (4096 - size_of::<slab_header>()) / size;
    //     let mut start = unsafe {
    //         &mut *((data.add(HHDM.get_response().unwrap().offset() as usize) as *mut slab_header)
    //         .add(1) as *mut KTNode)

    //     };
    //     let heraddr = (start as *mut KTNode) as usize;

    //     let mut prev = &mut start;

    //     for i in 1..obj_amount {
    //         let new = unsafe { &mut *((heraddr + i * size) as *mut KTNode) };
    //         new.next = None;
    //         prev.next = Some(new);
    //         prev = prev.next.as_mut().unwrap();
    //     }
    //     prev.next = None;
    //     h.freelist = Some(start);

    //     return h;
    // }
}
impl cache {
    pub fn init(size: usize) -> Self {
        let new = slab_header::init(size);
        println!("Created Cache of size: {size}");
        let ok = Self {
            size: size,
            slabs: holder_type2::from_raw(new),
        };
        ok
        
    }
    pub fn slab_allocsearch(&mut self) -> Option<*mut u8> {
        
        let h = &mut self.slabs;
        for j in h.into_iter() {
            unsafe {
                
                match j.pop() {
                    Some(y) => {
                       
                    y.as_ptr().cast::<u8>().write_bytes(0, self.size);
                     return Some(y.as_ptr().cast::<u8>());
                    },
                    None => {
                        
                    }
                }
            }
        //     for i in unsafe {(*j.as_ptr()).freelist.0.iter_mut()} {
            
        //     unsafe {
        //             (*j.as_ptr()).freelist = 
                    
        //             i.as_ptr().cast::<u8>().write_bytes(0, self.size);
        //             return Some(i.as_ptr().cast::<u8>());
                    
                
                
            }
        
        
        
        // println!("creating new");
        let new = slab_header::init(self.size);
        //println!("getting more");

        unsafe {
            
            
            
            match h.into_iter().last() {
                Some(jh) => {
                    (jh).next_slab = holder_type2::from_raw(new);
                },
                None => {
                    (*h.0.unwrap().as_ptr()).next_slab = holder_type2::from_raw(new);
                }
            }
            
            // println!("h's yk: {:#x}", h.into_iter().last().unwrap().as_ptr().addr());
        }
        
        self.slab_allocsearch()
    }
    pub fn free(&mut self, addr: *mut u8) {
        if addr == core::ptr::null_mut() {
            return;
        }
        let him = addr.map_addr(|a| a & !0xFFF).cast::<slab_header>();
        
        unsafe {
            addr.write_bytes(0, self.size);
            let bro = addr.cast::<um>();
            (*him).push(NonNull::new(bro).unwrap());
        }
    
        
    }
    pub fn clear_all_slabs(&mut self) {
        let mut cur_link = core::mem::replace(&mut self.slabs, holder_type2(None));
        while let Some(mut slab) = cur_link.0 {
            let g = unsafe {slab.as_mut()};
            let test = g.freelist.0.unwrap();
            unsafe {test.cast::<u8>().write_bytes(0, 4096)};
            cur_link = core::mem::replace(&mut g.next_slab, holder_type2::new());
            

        }
    }
}
impl Drop for cache {
    fn drop(&mut self) {
        self.clear_all_slabs();
    }
}
