
#![no_std]
#![feature(naked_functions)]
#![feature(exposed_provenance)]
// THIS IS FINE CAUSE I STAND BY NON CAMAL CASE
// ALSO CLEARS UP WARNINGS SO I CAN SEE ACTUAL IMPORTANT WARNINGS
#![allow(
    non_upper_case_globals,
    unused_variables,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    unused_macro_rules,
    unused_imports,
    unused_macros,
    unused_attributes
    
    
)]
#![feature(rustc_private)]
#![feature(strict_provenance)]
pub mod gdt;
pub mod idt;
pub mod mem;
pub mod term;
pub mod acpi;
pub mod timers;
pub fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            core::arch::asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            core::arch::asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            core::arch::asm!("idle 0");
        }
    }
}
#[derive(Debug)]
pub struct Element<T: Debug + PartialEq> {
    pub elem: T,
    pub next: ELink<T>
}
extern crate alloc;
use core::fmt::Debug;

use alloc::boxed::Box;
use mem::pmm::{self, cache};
type ELink<T> = Option<Box<Element<T>>>;
#[derive(Debug)]
pub struct List<T: Debug + PartialEq> {
    head: ELink<T>
}
#[derive(Debug)]
pub struct VList<T: Debug + PartialEq> {
    head: ELink<T>,
    cache: pmm::cache
}

impl<T: Debug + PartialEq> VList<T> {
    pub fn new<R>(t: cache) -> Self {
        Self {
            head: None,
            cache: t
        }
    }
    pub fn push(&mut self, r: T) {
        let data = self.cache.slab_allocsearch().unwrap().cast::<Element<T>>();
        unsafe {data.write(Element {
            elem: r,
            next: core::mem::replace(&mut self.head, None),
        })};
        let o = unsafe {Box::from_raw(data)};
        self.head = Some(o);
        
    }
    pub fn pop(&mut self) -> Option<T> {
        match core::mem::replace(&mut self.head, None) {
            None => None,
            Some(node) => {
                self.head = node.next;
                Some(node.elem)
            }
        }
    }
    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node|
        &node.elem)
    }
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node|
        {
            &mut node.elem
        })
    }
    pub fn peek_raw(&mut self) -> Option<&mut Box<Element<T>>> {
        self.head.as_mut().map(|node| node)
    }
    pub fn into_iter(self) -> ListIterV<T> {
        ListIterV(self)
    }
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            next: self.head.as_deref()
        }
    }
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut {
            next: self.head.as_deref_mut()
        }
    }

}
impl<T: Debug + PartialEq> List<T> {
    pub fn new<R>() -> Self {
        Self {
            head: None
        }
    }
    pub fn push(&mut self, r: T) {
        let new_node = Box::new(Element {
            elem: r,
            next: core::mem::replace(&mut self.head, None),
        });

        self.head = Some(new_node);
    }
    pub fn pop(&mut self) -> Option<T> {
        match core::mem::replace(&mut self.head, None) {
            None => None,
            Some(node) => {
                self.head = node.next;
                Some(node.elem)
            }
        }
    }
    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node|
        &node.elem)
    }
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node|
        {
            &mut node.elem
        })
    }
    pub fn peek_raw(&mut self) -> Option<&mut Box<Element<T>>> {
        self.head.as_mut().map(|node| node)
    }
    pub fn into_iter(self) -> ListIter<T> {
        ListIter(self)
    }
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            next: self.head.as_deref()
        }
    }
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut {
            next: self.head.as_deref_mut()
        }
    }
}
impl<T: Debug + PartialEq> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur_link = core::mem::replace(&mut self.head, None);
        while let Some(mut boxed_node) = cur_link {
            cur_link = core::mem::replace(&mut boxed_node.next, None);
        }
    }
}
pub struct ListIter<T: Debug + PartialEq>(List<T>);

pub struct ListIterV<T: Debug + PartialEq>(VList<T>);
impl<T: Debug + PartialEq> Iterator for ListIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}
impl<T: Debug + PartialEq> Iterator for ListIterV<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}
impl <'a, T: Debug + PartialEq> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref_mut();
            &mut node.elem
        })
    }
}
impl <'a, T: Debug + PartialEq> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref();
            &node.elem
        })
    }
}
pub struct Iter<'a, T: Debug + PartialEq> {
    next: Option<&'a Element<T>>
}
pub struct IterMut<'a, T: Debug + PartialEq> {
    next: Option<&'a mut Element<T>>
}
