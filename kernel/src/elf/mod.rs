type Elf64_Addr = usize;
type Elf64_Off = usize;
type Elf64_Half = u16;
type Elf64_Word = u32;
type Elf64_Sword = i32;
type Elf64_Xword = usize;
type Elf64_Sxword = i64;
pub mod symbol;
/* File Header */
#[repr(C)]
pub struct Elf64_Ehdr {
    e_ident: [u8; 16], /* ELF identification */
    e_type: Elf64_Half, /* Object file type */
    e_machine: Elf64_Half,  /* Machine type */
    e_version: Elf64_Word, /* Object file version */
    e_entry: Elf64_Addr, /* Entry point address */
    e_phoff: Elf64_Off,  /* Program header offset */
    e_shoff: Elf64_Off, /* Section header offset */
    e_flags: Elf64_Word, /* Processor-specific flags */
    e_ehsize: Elf64_Half,  /* ELF header size */
    e_phentsize: Elf64_Half, /* Size of program header entry */
    e_phnum: Elf64_Half, /* Number of program header entries */
    e_shentsize: Elf64_Half, /* Size of section header entry */
    e_shnum: Elf64_Half, /* Number of section header entries */
    e_shstrndx: Elf64_Half  /* Section name string table index */
}
/* Section Header */
#[repr(C)]
pub struct Elf64_Shdr {
    sh_name: Elf64_Word, /* Section Name */
    sh_type: Elf64_Word, /* Section Type */
    sh_flags: Elf64_Xword, /* Section Attributes */
    sh_addr: Elf64_Addr, /* Virtual Address in Memory */
    sh_offset: Elf64_Off, /* Offset in file */
    sh_size: Elf64_Xword, /* Size of section */
    sh_link: Elf64_Word, /* Link info to other sections */
    sh_info: Elf64_Word, /* Misc Info. */
    sh_addralign: Elf64_Xword, /* Address Alignment Boundary */
    sh_entsize: Elf64_Xword /* Size of Entries, if section represents table */
}
/* Sym Table Entry */
#[repr(C)]
#[derive(Debug)]
pub struct Elf64_Sym {
    st_name: Elf64_Word /* contains the offset, in bytes, to the symbol name, relative to the
    start of the symbol string table. If this field contains zero, the symbol has
    no name.  */,
    st_info: u8, /* Type and Binding attributes */
    st_other: u8, /* Reversed */
    st_shnidx: Elf64_Half, /* contains the section index of the section in which the symbol is
    “defined.” For undefined symbols, this field contains SHN_UNDEF; for
    absolute symbols, it contains SHN_ABS; and for common symbols, it
    contains SHN_COMMON.  */
    st_value: Elf64_Addr, /* contains the value of the symbol. This may be an absolute value
    or a relocatable address.
    In relocatable files, this field contains the alignment constraint for
    common symbols, and a section-relative offset for defined relocatable
    symbols.
    In executable and shared object files, this field contains a virtual address
    for defined relocatable symbols. */
    st_size: Elf64_Xword, /* contains the size associated with the symbol. If a symbol does not
    have an associated size, or the size is unknown, this field contains zero.  */
}