use core::fmt;

use bitflags::{bitflags, Flags};
use limine::{memory_map::EntryType, request::KernelAddressRequest};
use owo_colors::OwoColorize;
extern crate alloc;
use alloc::boxed::Box;
use spin::Mutex;

use crate::{hcf, println};

use super::pmm::{HDDM_OFFSET, MEMMAP, PMM};
bitflags! {
    #[derive(Debug)]
    pub struct VMMFlags: usize
    {
        const KTEXECUTABLEDISABLE = 1 << 63;
        const KTPRESENT = 1;
        const KTWRITEALLOWED = 1 << 1;
        const KTUSERMODE = 1 << 2;
        const KTWRITETHROUGH = 1 << 3;
        const KTCACHEDISABLE = 1 << 4;
        const KTPATBIT4096 = 1 << 7;
        const KTPATBIT2MB = 1 << 12;
        const KT2MB = 1 << 7;

    }
}
use alloc::vec::Vec;

pub struct VMMRegion {
    base: usize,
    length: usize,
    flags: VMMFlags,
}
impl fmt::Debug for VMMRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VMMRegion {{\n    base: {:#x}\n    length: {}\n    flags: {:?}\n}}",
            self.base, self.length, self.flags
        )
    }
}
#[derive(Debug)]
pub struct PageMap {
    head: Vec<VMMRegion>,
    rootpagetable: *mut usize,
}
#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
}
#[macro_export]
macro_rules! unwrap_or_return0 {
    ( $e:expr ) => {
        match $e {
            Some(x) => x,
            None => return 0,
        }
    };
}
// it is similar and we are using raw pointer however,
// no raw addresses tho like just u64 or sm shit
// only using raw pointers when NEEDED
// if there is something to be improved please notify me abt it
#[used]
#[link_section = ".requests"]
pub static ADDR: KernelAddressRequest = KernelAddressRequest::new();
unsafe impl Send for PageMap {}
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}
unsafe impl Sync for PageMap {}
pub static KERMAP: Mutex<Option<PageMap>> = Mutex::new(None);
extern "C" {
    static THE_REAL: u8;
}
unsafe fn virt<T>(addr: usize) -> *mut T {
    (HDDM_OFFSET.get_response().unwrap().offset() as usize + addr) as *mut T
}

impl PageMap {
    unsafe fn find_pte_and_allocate(mut pt: usize, va: usize) -> *mut usize {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab: *mut usize = virt(pt);

            if i == 3 {
                return ptab.add(idx);
            }
            let entry = ptab.add(idx);

            if entry.read() & VMMFlags::KTPRESENT.bits() == 0 {
                entry.write(
                    {
                        let data = PMM.lock().alloc().unwrap();
                        data.add(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                            .write_bytes(0, 4096);
                        data as usize
                    } | VMMFlags::KTPRESENT.bits()
                        | VMMFlags::KTWRITEALLOWED.bits(),
                );
            }

            pt = entry.read() & 0x000f_ffff_ffff_f000;
        }
        unreachable!()
    }
    unsafe fn find_pte_and_allocate2mb(mut pt: usize, va: usize) -> *mut usize {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab: *mut usize = virt(pt);

            if i == 2 {
                return ptab.add(idx);
            }
            let entry = ptab.add(idx);

            if entry.read() & VMMFlags::KTPRESENT.bits() == 0 {
                entry.write(
                    {
                        let data = PMM.lock().alloc().unwrap();
                        data.add(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                            .write_bytes(0, 4096);
                        data as usize
                    } | VMMFlags::KTPRESENT.bits()
                        | VMMFlags::KTWRITEALLOWED.bits(),
                );
            }

            let p = entry.read();
            pt = entry.read() & 0x000f_ffff_ffff_f000;
        }
        unreachable!()
    }
    unsafe fn find_pte(mut pt: usize, va: usize) -> *mut usize {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab: *mut usize = virt(pt);

            if i == 3 {
                return ptab.add(idx);
            }
            let entry = ptab.add(idx);

            if entry.read() & VMMFlags::KTPRESENT.bits() == 0 {
                return entry;
            }

            pt = entry.read() & 0x000f_ffff_ffff_f000;
        }
        unreachable!()
    }
    pub fn map(&self, pt: usize, va: usize, flags: usize) {
        let him = unsafe { Self::find_pte_and_allocate(self.rootpagetable as usize, va) };

        unsafe { him.write(pt | flags) };
    }
    pub fn map2mb(&self, pt: usize, va: usize, flags: usize) {
        let him = unsafe {
            Self::find_pte_and_allocate2mb(self.rootpagetable as usize, va & !0xfffff_usize)
        };

        unsafe { him.write((pt & !0xfffff_usize) | flags | VMMFlags::KT2MB.bits()) };
    }
    pub fn unmap(&self, va: usize) {
        let him = unsafe { Self::find_pte(self.rootpagetable as usize, va) };

        unsafe { him.write(0) };
        println!("{:#x}", unsafe { him.read() });
        unsafe {
            core::arch::asm!("invlpg [{x}]", x = in(reg) va, options(nostack, preserves_flags))
        };
    }
    pub fn virt_to_phys(&self, va: usize) -> usize {
        let him = unsafe { Self::find_pte(self.rootpagetable as usize, va) };
        return unsafe { him.read() as usize };
    }
    pub fn new_inital() {
        let mut q = PageMap {
            head: Vec::new(),
            rootpagetable: {
                let data = PMM.lock().alloc().unwrap();
                unsafe {
                    data.add(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                        .write_bytes(0, 4096)
                }
                data as *mut usize
            },
        };
        println!("{:#x}", q.rootpagetable as usize);
        let size_pages = unsafe { align_up(&THE_REAL as *const _ as usize, 4096) / 4096 };
        println!("kernel in pages {}", size_pages);
        for i in 0..=size_pages {
            q.map(
                ADDR.get_response().unwrap().physical_base() as usize + (i * 4096),
                ADDR.get_response().unwrap().virtual_base() as usize + (i * 4096),
                VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
            );
        }
        println!("kernel has been mapped");
        let mut hhdm_pages = 0;
        for i in (0..0x100000000 as usize).step_by(2097152) {
            assert_eq!(i % 2097152, 0);

            q.map2mb(
                i as usize,
                HDDM_OFFSET.get_response().unwrap().offset() as usize + i,
                VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
            );
            hhdm_pages += 1;
        }
        println!("hhdm mapped, mapping memory map");
        let mut map = MEMMAP.lock();
        let entries = map.get_response_mut().unwrap().entries_mut();
        for i in entries.iter_mut() {
            match i.entry_type {
                EntryType::ACPI_NVS
                | EntryType::ACPI_RECLAIMABLE
                | EntryType::USABLE
                | EntryType::BOOTLOADER_RECLAIMABLE
                | EntryType::KERNEL_AND_MODULES
                | EntryType::RESERVED => {
                    let disalign = i.base as usize % 4096;

                    i.base = align_down(i.base as usize, 4096) as u64;
                    let page_amount = align_up(i.length as usize - disalign, 2097152) / 2097152;

                    for e in 0..page_amount {
                        q.map2mb(
                            i.base as usize + (e * 2097152) as usize,
                            HDDM_OFFSET.get_response().unwrap().offset() as usize
                                + i.base as usize
                                + (e * 2097152) as usize,
                            VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
                        )
                    }
                    hhdm_pages += page_amount;
                }
                EntryType::FRAMEBUFFER => {
                    let disalign = i.base as usize % 2097152;
                    i.base = align_down(i.base as usize, 2097152) as u64;
                    let page_amount = align_up(i.length as usize - disalign, 2097152) / 2097152;

                    for e in 0..page_amount {
                        q.map2mb(
                            i.base as usize + (e * 2097152) as usize,
                            HDDM_OFFSET.get_response().unwrap().offset() as usize
                                + i.base as usize
                                + (e * 2097152) as usize,
                            // enable wc for sped
                            VMMFlags::KTPRESENT.bits()
                                | VMMFlags::KTWRITEALLOWED.bits()
                                | VMMFlags::KTPATBIT2MB.bits()
                                | VMMFlags::KTWRITETHROUGH.bits(),
                        );
                    }
                    hhdm_pages += page_amount;
                }
                _ => {}
            }
        }
        println!("{:#x}", q.rootpagetable as usize);
        q.switch_to();
        q.region_setup(hhdm_pages);
        let o = q
            .vmm_region_alloc(12050, VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED)
            .unwrap();
        unsafe {
            o.write(1);
            if o.read() == 1 {
                println!("IT FUCKING WORKED");
            }
        }
        q.vmm_region_dealloc(o as usize);
        unsafe { o.write(2) };
        // q.region_walk();
        *KERMAP.lock() = Some(q);

        println!("vmm inited");
    }
    pub fn switch_to(&self) {
        unsafe {
            core::arch::asm!(
                "mov cr3, {}",
                in(reg) self.rootpagetable as usize
            )
        }
    }
    pub fn region_walk(&self) {
        for i in self.head.iter() {
            println!("{:#?}", i);
        }
    }
    pub fn region_setup(&mut self, pages_in_hhdm: usize) {
        // println!("got {pages_in_hhdm}");
        // let kernel_range = Some(Box::new(VMMRegion {
        //     base: ADDR.get_response().unwrap().virtual_base() as usize,
        //     length: unsafe { align_up(&THE_REAL as *const _ as usize, 4096) },
        //     flags: VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
        //     next: None,
        // }));
        // let mut hhdm_range = Some(Box::new(VMMRegion {
        //     base: HDDM_OFFSET.get_response().unwrap().offset() as usize,
        //     length: align_up(pages_in_hhdm * 0x1000, 4096),
        //     flags: VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
        //     next: None,
        // }));
        // println!("Kernel Region: {:#?}", kernel_range);
        // println!("HHDM Region: {:#?}", hhdm_range);
        // hhdm_range.as_mut().unwrap().next = kernel_range;
        // self.head = hhdm_range;
        let ITSHIM = VMMRegion {
            base: ADDR.get_response().unwrap().virtual_base() as usize,
            length: unsafe { align_up(&THE_REAL as *const _ as usize, 4096) },
            flags: VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED,
        };
        let HHDM = VMMRegion {
            base: HDDM_OFFSET.get_response().unwrap().offset() as usize,
            length: align_up(pages_in_hhdm * 0x1000, 4096),
            flags: VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED,
        };

        self.head.push(HHDM);
        self.head.push(ITSHIM);
    }
    pub fn vmm_region_alloc(&mut self, size: usize, flags: VMMFlags) -> Option<*mut u8> {
        let mut store = None;
        for (idx, i) in self.head.iter_mut().enumerate() {
            if idx == 0 {
                store = Some(i);

                continue;
            }
            let temp = store.as_mut().unwrap();
            if i.base - temp.base + temp.length >= align_up(size as usize, 4096) as usize + 0x1000 {
                let new_guy = VMMRegion {
                    base: temp.base + temp.length,
                    length: align_up(size, 4096),
                    flags,
                };

                let amou = align_up(size as usize, 4096) / 4096;
                for i in 0..amou {
                    let data = {
                        let o = PMM.lock().alloc().unwrap();
                        unsafe {
                            o.add(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                                .write_bytes(0, 4096);
                        }
                        o
                    };
                    self.map(
                        data as usize,
                        new_guy.base + (i * 0x1000),
                        new_guy.flags.bits(),
                    );
                }
                let n = new_guy.base;
                self.head.insert(idx, new_guy);
                return Some(n as *mut u8);
            } else {
                store = Some(i);
                continue;
            }
            // |    |              |    |
            // |    |  FREE SPACE  |    |
            // |____|. . . . . . . |____|
        }
        panic!("out of vmm region space lmao");
    }
    pub fn vmm_region_dealloc(&mut self, addr: usize) {
        let mut idxx = -1;
        for (idx, i) in self.head.iter().enumerate() {
            if i.base == addr {
                let num_of_pages = i.length / 4096;
                for f in 0..num_of_pages {
                    let phys = self.virt_to_phys(i.base + (f * 0x1000)) as *mut u8;
                    self.unmap(i.base + (f * 0x1000));
                    PMM.lock()
                        .dealloc(unsafe {
                            phys.add(HDDM_OFFSET.get_response().unwrap().offset() as usize)
                        })
                        .unwrap();
                }
                idxx = idx as i32;
            }
        }
        if idxx == -1 {
            println!("WTF");
            return;
        }
        self.head.remove(idxx as usize);
    }
}
