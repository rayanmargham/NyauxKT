use core::{fmt, ptr::write_bytes};

use bitflags::{bitflags, Flags};
use limine::{memory_map::EntryType, request::KernelAddressRequest};
use owo_colors::OwoColorize;
extern crate alloc;
use alloc::boxed::Box;

use spin::Mutex;
use volatile::VolatilePtr;

use crate::{
    hcf,
    mem::{
        align_down, align_up,
        pmm::{cache, pmm_dealloc},
        MEMMAP,
    },
    println, List, VList,
};
bitflags! {
    #[derive(Debug, PartialEq)]
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

use super::{pmm::pmm_alloc, HHDM};
#[derive(PartialEq)]
pub struct VMMRegion {
    base: usize,
    length: usize,
    flags: VMMFlags,
    iskernel: bool,
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
    head: VList<VMMRegion>,
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
#[test]
fn t() {
    let ok: List<VMMRegion> = List::new::<VMMRegion>();
    for i in ok.iter() {}
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
#[used]
#[link_section = ".requests"]
pub static ADDR: KernelAddressRequest = KernelAddressRequest::new();
unsafe impl Send for PageMap {}
unsafe impl Sync for PageMap {}
extern "C" {
    static THE_REAL: u8;
}
pub static KERMAP: Mutex<Option<PageMap>> = Mutex::new(None);
unsafe fn virt<T>(addr: *mut u8) -> *mut T {
    addr.add(HHDM.get_response().unwrap().offset() as usize)
        .cast::<T>()
}
impl PageMap {
    unsafe fn find_pte_and_allocate(mut pt: *mut usize, va: usize) -> *mut usize {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab = virt::<usize>(pt.cast::<u8>());

            if i == 3 {
                return ptab.add(idx);
            }
            let entry = ptab.add(idx);
            if entry.read_volatile() & VMMFlags::KTPRESENT.bits() == 0 {
                let new_pt = core::ptr::with_exposed_provenance_mut::<u8>(pmm_alloc().unwrap());
                let (reference, other) = {
                    let o = new_pt;
                    o.add(HHDM.get_response().unwrap().offset() as usize)
                        .write_bytes(0, 4096);
                    let j = o.cast::<usize>();
                    (j, o)
                };

                ptab.add(idx).write(
                    reference
                        .map_addr(|a| {
                            a | VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits()
                        })
                        .addr(),
                );

                pt = reference;
            } else {
                pt = entry.with_addr(entry.read_volatile() & 0x000f_ffff_ffff_f000);
            }
        }
        unreachable!();
        todo!()
    }
    unsafe fn find_pte_and_allocate2mb(mut pt: *mut usize, va: usize) -> *mut usize {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab = virt::<usize>(pt.cast::<u8>());
            if i == 2 {
                return ptab.add(idx);
            }
            let entry = ptab.add(idx);
            if entry.read_volatile() & VMMFlags::KTPRESENT.bits() == 0 {
                let new_pt = core::ptr::with_exposed_provenance_mut::<u8>(pmm_alloc().unwrap());
                let (reference, other) = {
                    let o = new_pt;
                    o.add(HHDM.get_response().unwrap().offset() as usize)
                        .write_bytes(0, 4096);
                    let j = o.cast::<usize>();
                    (j, o)
                };

                ptab.add(idx).write(
                    reference
                        .map_addr(|a| {
                            a | VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits()
                        })
                        .addr(),
                );

                pt = reference;
            } else if entry.read_volatile() & VMMFlags::KT2MB.bits() == 1 && i == 2 {
                let data = core::ptr::with_exposed_provenance_mut::<u8>(pmm_alloc().unwrap());
                data.add(HHDM.get_response().unwrap().offset() as usize)
                    .write_bytes(0, 4096);
                let g: *mut usize = data
                    .add(HHDM.get_response().unwrap().offset() as usize)
                    .cast::<usize>();
                let old_phys = entry
                    .map_addr(|a| a & 0x000f_ffff_ffff_f000)
                    .read_volatile();
                let old_flags = entry
                    .map_addr(|a| a & !0x000f_ffff_ffff_f000)
                    .read_volatile();
                for i in 0..512 {
                    g.add(idx).write_volatile(
                        old_phys + i * 4096 | (old_flags & !VMMFlags::KT2MB.bits()),
                    );
                }
                pt = data.cast::<usize>();
            } else {
                pt = entry.with_addr(entry.read_volatile() & 0x000f_ffff_ffff_f000);
            }
        }
        unreachable!();
        todo!()
    }
    unsafe fn find_pte(mut pt: *mut usize, va: usize) -> Option<*mut usize> {
        let mut shift = 48;
        for i in 0..4 {
            shift -= 9;
            let idx = (va >> shift) & 0x1ff;
            let ptab = virt::<usize>(pt.cast::<u8>());
            if i == 3 {
                if ptab.add(idx).read_volatile() == 0 {
                    return None;
                }
                return Some(ptab.add(idx));
            }
            let entry = ptab.add(idx);
            if entry.read_volatile() & VMMFlags::KTPRESENT.bits() == 0 {
                return None;
            } else if entry.read_volatile() & VMMFlags::KT2MB.bits() == 1 && i == 2 {
                let data = core::ptr::with_exposed_provenance_mut::<u8>(pmm_alloc().unwrap());
                data.add(HHDM.get_response().unwrap().offset() as usize)
                    .write_bytes(0, 4096);
                let g: *mut usize = data
                    .add(HHDM.get_response().unwrap().offset() as usize)
                    .cast::<usize>();
                let old_phys = entry
                    .map_addr(|a| a & 0x000f_ffff_ffff_f000)
                    .read_volatile();
                let old_flags = entry
                    .map_addr(|a| a & !0x000f_ffff_ffff_f000)
                    .read_volatile();
                for i in 0..512 {
                    g.add(idx).write_volatile(
                        old_phys + i * 4096 | (old_flags & !VMMFlags::KT2MB.bits()),
                    );
                }
                pt = data.cast::<usize>();
            } else {
                pt = entry.with_addr(entry.read_volatile() & 0x000f_ffff_ffff_f000);
            }
        }
        // unreachable!()
        todo!()
    }

    pub fn map(&self, pt: usize, va: usize, flags: usize) {
        let him = unsafe { Self::find_pte_and_allocate(self.rootpagetable, va) };

        unsafe { him.write_volatile(pt | flags) };
    }
    pub fn map2mb(&self, pt: usize, va: usize, flags: usize) {
        let him =
            unsafe { Self::find_pte_and_allocate2mb(self.rootpagetable, va & !0x1fffff_usize) };

        unsafe { him.write_volatile(pt & !0x1fffff_usize | flags | VMMFlags::KT2MB.bits()) };
    }
    pub fn unmap(&self, va: usize) {
        let him = unsafe { Self::find_pte(self.rootpagetable, va) };
        if let Some(h) = him {
            unsafe { h.write_volatile(0) };

            unsafe {
                core::arch::asm!(
                    "invlpg [{0}]",
                    in(reg) va,
                    options(nostack)
                );
            };
        } else {
            println!("not found");
        }
    }
    pub fn virt_to_phys(&self, va: usize) -> Option<*mut usize> {
        let him = unsafe { Self::find_pte(self.rootpagetable, va) };
        if let Some(h) = him {
            return Some(h.map_addr(|a| a & 0x0007FFFFFFFFF000));
        } else {
            None
        }
    }
    pub fn new_inital() {
        let mut q = PageMap {
            head: VList::new::<VMMRegion>(cache::init(size_of::<VMMRegion>().next_power_of_two())),
            rootpagetable: unsafe {
                let data = core::ptr::with_exposed_provenance_mut::<u8>(pmm_alloc().unwrap());
                (data as *mut u8)
                    .add(HHDM.get_response().unwrap().offset() as usize)
                    .write_bytes(0, 4096);
                data.cast::<usize>()
            },
        };
        println!("done");
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
                HHDM.get_response().unwrap().offset() as usize + i,
                VMMFlags::KTPRESENT.bits() | VMMFlags::KTWRITEALLOWED.bits(),
            );
            hhdm_pages += 1;
        }
        println!("hhdm mapped, mapping memory map");
        let mut lock = MEMMAP.lock();
        let entries = lock.get_response_mut().unwrap().entries_mut();
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
                            HHDM.get_response().unwrap().offset() as usize
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
                            HHDM.get_response().unwrap().offset() as usize
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
            iskernel: true,
        };
        let HHDMM = VMMRegion {
            base: HHDM.get_response().unwrap().offset() as usize,
            length: align_up(pages_in_hhdm * 0x1000, 4096),
            flags: VMMFlags::KTPRESENT | VMMFlags::KTWRITEALLOWED,
            iskernel: false,
        };
        self.head.push(ITSHIM);
        self.head.push(HHDMM);
    }
    pub fn vmm_region_alloc(&mut self, size: usize, flags: VMMFlags) -> Option<*mut u8> {
        let mut store: Option<&mut VMMRegion> = None;
        for (idx, i) in self.head.iter_mut().enumerate() {
            if store.is_none() {
                store = Some(i);
                continue;
            }
            let temp = store.unwrap();

            if i.base.wrapping_sub(temp.base + temp.length)
                >= align_up(size as usize, 4096) as usize + 0x1000
            {
                let new_guy = VMMRegion {
                    base: temp.base + temp.length,
                    length: align_up(size, 4096),
                    flags,
                    iskernel: false,
                };

                let amou = align_up(size as usize, 4096) / 4096;
                for i in 0..amou {
                    let data = {
                        let o = pmm_alloc().unwrap() as *mut u8;
                        unsafe {
                            o.add(HHDM.get_response().unwrap().offset() as usize)
                                .write_bytes(0, 4096);
                        }

                        o
                    };
                    self.map(
                        data.expose_provenance(),
                        new_guy.base + (i * 0x1000),
                        new_guy.flags.bits(),
                    );
                }

                let h = core::ptr::with_exposed_provenance_mut::<u8>(new_guy.base);
                unsafe { h.write_bytes(0, new_guy.length) };

                self.head.push(new_guy);

                return Some(h);
            } else {
                store = Some(i);
                continue;
            }
        }
        panic!("no space");
        // let mut store = None;
        // for (idx, i) in self.head.iter_mut().enumerate() {
        //     if idx == 0 {
        //         store = Some(i);

        //         continue;
        //     }
        //     let temp = store.as_mut().unwrap();
        //     if i.base - (temp.base + temp.length) >= align_up(size as usize, 4096) as usize + 0x1000 {
        //         let new_guy = VMMRegion {
        //             base: temp.base + temp.length,
        //             length: align_up(size, 4096),
        //             flags,
        //         };
        //         println!("created region {:#?}", new_guy);
        //         let amou = align_up(size as usize, 4096) / 4096;
        //         for i in 0..amou {
        //             let data = {
        //                 let o = pmm_alloc().unwrap() as *mut u8;
        //                 unsafe {
        //                     o.add(HHDM.get_response().unwrap().offset() as usize)
        //                         .write_bytes(0, 4096);
        //                 }
        //                 o
        //             };
        //             self.map(
        //                 data.addr(),
        //                 new_guy.base + (i * 0x1000),
        //                 new_guy.flags.bits(),
        //             );
        //         }
        //         let h = 0 as *mut u8;
        //         unsafe {h.with_addr(new_guy.base).write_bytes(0, new_guy.length)};
        //         let n = new_guy.base;
        //         self.head.insert(idx, new_guy);
        //         self.region_walk();
        //         return Some(h.with_addr(n) as *mut u8);
        //     } else {
        //         store = Some(i);
        //         continue;
        //     }
        //     // |    |              |    |
        //     // |    |  FREE SPACE  |    |
        //     // |____|. . . . . . . |____|
        // }
        // panic!("out of vmm region space lmao");
    }
    pub fn vmm_region_dealloc(&mut self, addr: *mut u8) {
        if addr == core::ptr::null_mut() {
            return;
        }
        let mut store: Option<&mut VMMRegion> = None;
        for i in self.head.iter_mut() {
            if store.is_none() {
                store = Some(i);
                continue;
            }
            let temp = store.as_mut().unwrap();
            if i.base == addr.expose_provenance() {
                // let ok = self.head.peek_raw().expect("This is not Possible...");
            }
        }
        // let mut idxx = -1;
        // for (idx, i) in self.head.iter().enumerate() {
        //     if i.base == addr {
        //         println!("deallocing region {:#?}", i);
        //         let num_of_pages = i.length / 4096;
        //         for f in 0..num_of_pages {
        //             println!("deaellocing");
        //             let phys = self.virt_to_phys(i.base + (f * 0x1000)).unwrap() as *mut u8;
        //             self.unmap(i.base + (f * 0x1000));
        //             assert_eq!(
        //                 None,
        //                 Self::find_pte(self.rootpagetable as usize, i.base + (f * 0x1000))
        //             );

        //             pmm_dealloc(phys as usize).unwrap();
        //         }
        //         idxx = idx as i32;
        //     }
        // }
        // if idxx == -1 {
        //     println!("WTF");
        //     return;
        // }
        // self.head.remove(idxx as usize);
    }
}
