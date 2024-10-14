use core::ffi::{c_char, CStr};

use crate::{elf::{Elf64_Shdr, Elf64_Sym}, print, println};

use super::Elf64_Ehdr;
use alloc::{collections::btree_map::{self, BTreeMap}, string::{String, ToString}};
use hashbrown::HashMap;
use owo_colors::OwoColorize;
use spin::once::Once;
use rustc_demangle::demangle;

#[used]
#[link_section = ".requests"]
static kernelfile: limine::request::KernelFileRequest = limine::request::KernelFileRequest::new();
pub static symbol_table: Once<btree_map::BTreeMap<usize, String>> = Once::new();
pub fn load() {
    if let Some(k) = kernelfile.get_response() {
        let hdr = k.file().addr().cast::<Elf64_Ehdr>();
        unsafe {
            println!("e_ident [0, 1, 2, 3]: {}{}{}{}", 
                (*hdr).e_ident[0] as char,
                (*hdr).e_ident[1] as char,
                (*hdr).e_ident[2] as char,
                (*hdr).e_ident[3] as char
            )
        }
        let size_of_shent = unsafe {(*hdr).e_shentsize};
        let num_of_shent = unsafe {(*hdr).e_shnum};
        let offset_to_shtable = unsafe {(*hdr).e_shoff};
        println!("Section Information: Size of Section Table Entry {}\n   Number of Entries {}\n  Offset to Section Table {:#x}+{:#x}",
        size_of_shent,
        num_of_shent,
        k.file().addr().addr(),
        offset_to_shtable);
        let mut idx = 0;
        let mut found: Option<*mut Elf64_Shdr> = None;
        let mut stringtable: Option<*mut Elf64_Shdr> = None;
        while idx != num_of_shent {
            let sec = unsafe {k.file().addr().add(offset_to_shtable + (idx as usize * size_of_shent as usize)).cast::<Elf64_Shdr>()};
            unsafe {
                
                if found.is_some() && stringtable.is_some() {
                    break;
                }
                if (*sec).sh_type == 2 {
                    found = Some(sec);
                    
                    
                } else if (*sec).sh_type == 3 && idx != (*hdr).e_shstrndx{
                    println!("found i think");
                    stringtable = Some(sec);
                } 
                    idx += 1;
                    continue;
                
            }
            
        }
        let mut new = BTreeMap::new();
        if let Some(symtabhdr) = found{
            let stringtable = stringtable.unwrap();
            let amoofsyms = unsafe {(*symtabhdr).sh_size / (*symtabhdr).sh_entsize};
            println!("Amount of Symbols: {}", amoofsyms);
            let mut idx = 1;
            while idx != amoofsyms{
                let sym = unsafe {k.file().addr().add((*symtabhdr).sh_offset + (idx * (*symtabhdr).sh_entsize as usize)).cast::<Elf64_Sym>()};
                
                let name = unsafe {
                    k.file().addr().add((*stringtable).sh_offset + ((*sym).st_name as usize )).cast::<u8>()
                };
                
                
                new.insert(unsafe {(*sym).st_value}, unsafe {demangle(CStr::from_ptr(name as *const c_char).to_str().unwrap()).to_string()});
                idx += 1;
            }
        }
        symbol_table.call_once(||new);
        println!("{}", "Table Loaded".yellow());
    }
}