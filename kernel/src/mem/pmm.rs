use crate::println;

use super::align_up;
use super::HHDM;
use super::MEMMAP;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use spin::mutex::Mutex;
#[derive(Debug)]
pub struct KTNode {
    next: Option<&'static mut KTNode>,
}

static HEAD: Mutex<Option<&mut KTNode>> = Mutex::new(None);
pub static FREEPAGES: AtomicUsize = AtomicUsize::new(0);
pub fn pmm_init() {
    let mut head = HEAD.lock();
    let o = MEMMAP.get_response().unwrap().entries();
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
}

pub fn pmm_alloc() -> Option<usize> {
    let mut mutd = HEAD.lock();
    let head = mutd.as_mut()?;
    let next_node = head.next.take();
    let it = *head as *mut KTNode as usize;
    FREEPAGES.fetch_sub(1, Ordering::SeqCst);
    *mutd = next_node;
    Some(it)
}
pub fn pmm_dealloc(addr: usize) -> Option<()> {
    let mut mutd = HEAD.lock();
    let head = mutd.as_mut()?;
    let node = head.next.take();
    let created = unsafe { &mut *(addr as *mut KTNode) };
    created.next = node;
    *mutd = Some(created);
    FREEPAGES.fetch_add(1, Ordering::SeqCst);
    Some(())
}
