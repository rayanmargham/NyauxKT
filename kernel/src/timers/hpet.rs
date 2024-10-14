enum HPETRegisterLocations {
    GeneralCapandIdRegister = 0x0,
    GeneralConfigRegister = 0x10,
    GeneralInterruptStatusRegister = 0x20,
    MainCounterValueRegister = 0x0F0, /* Todo. Config the other timers in the hpet */
}
use crate::{
    mem::{align_down, HHDM},
    println,
};
use core::ptr::NonNull;
use spin::once::Once;
use uacpi::{reboot, HPET_SIGNATURE};
use volatile::VolatilePtr;

pub struct HPET {
    hpet: *mut usize, /* SAFETY: Must be wrapped in a VolatilePtr<NonNull<*mut T>> before writing */
    main_counter_tickns: u32,
}
unsafe impl Send for HPET {}
unsafe impl Sync for HPET {}
pub static thehpet: Once<HPET> = Once::new();
impl HPET {
    pub fn write_to_hpet(&self, reg: usize, val: usize, bit32: bool) {
        if bit32 == true {
            let h = unsafe {
                VolatilePtr::new(
                    NonNull::new(self.hpet.cast::<u8>().add(reg).cast::<u32>()).unwrap(),
                )
            };
            h.write(val as u32);
        } else {
            let h = unsafe {
                VolatilePtr::new(
                    NonNull::new(self.hpet.cast::<u8>().add(reg).cast::<usize>()).unwrap(),
                )
            };
            h.write(val);
        }
    }
    pub fn read_from_hpet(&self, reg: usize, bit32: bool) -> usize {
        if bit32 == true {
            let h = unsafe {
                VolatilePtr::new(
                    NonNull::new(self.hpet.cast::<u8>().add(reg).cast::<u32>()).unwrap(),
                )
            };
            return h.read() as usize;
        } else {
            let h = unsafe {
                VolatilePtr::new(
                    NonNull::new(self.hpet.cast::<u8>().add(reg).cast::<usize>()).unwrap(),
                )
            };
            return h.read();
        }
    }
    pub fn get_ticks(&self) -> usize {
        self.read_from_hpet(0xF0, false)
    }
    pub fn ksleep(&self, ms: usize) {
        let pol_start = self.get_ticks();
        let mut pol_cur = self.get_ticks();
        while ((pol_cur - pol_start) * self.main_counter_tickns as usize) < ms * 1000000 {
            pol_cur = self.get_ticks();
        }
    }
    pub fn new() {
        'block: {
            match uacpi::table_find_by_signature(HPET_SIGNATURE) {
                Ok(hpet) => {
                    let h = hpet.get_virt_addr().cast::<uacpi::Hpet>();
                    let bro = unsafe { core::ptr::read_unaligned(h) };
                    let gas = bro.address;
                    let h = gas.address;
                    let bigpro: *mut u32 = core::ptr::with_exposed_provenance_mut(
                        h as usize + HHDM.get_response().unwrap().offset() as usize,
                    );
                    let bigno = unsafe { VolatilePtr::new(NonNull::new(bigpro).unwrap()) };
                    if (bigno.read() & (1 << 13)) != 0 {
                        let bigno = unsafe {
                            VolatilePtr::new(NonNull::new(bigpro.cast::<usize>()).unwrap())
                        };
                        let ctr = (bigno.read() >> 32) as u32;
                        println!("ticks: nanoseconds {}", ctr / 1000000);
                        let bigno = unsafe {
                            VolatilePtr::new(
                                NonNull::new(bigpro.cast::<u8>().add(0x10).cast::<usize>())
                                    .unwrap(),
                            )
                        };
                        bigno.write(1);
                        thehpet.call_once(|| Self {
                            hpet: bigpro.cast::<usize>(),
                            main_counter_tickns: ctr / 1000000,
                        });
                    } else {
                        panic!("We don't support 32 bit HPET's  sorry :(");
                    }
                    // return Some(Self {
                    //     hpet: h
                    // });
                }
                Err(f) => {
                    println!("Warning: {:?}", f);
                }
            }
        }
    }
}
