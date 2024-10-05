#![no_std]
#![feature(naked_functions)]
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
    unused_macros
)]
pub mod gdt;
pub mod idt;
pub mod mem;
pub mod term;
pub mod acpi;
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
