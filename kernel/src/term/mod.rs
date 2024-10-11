use core::{ffi::c_void, fmt};

use limine::framebuffer::Framebuffer;
use spin::Mutex;
static FONT: &[u8] = include_bytes!("../../font.F16");
const BG: u32 = 0x000000;
const FG: u32 = 0xe3e3de;
pub struct Terminal<'a> {
    fb: Option<&'a Framebuffer<'a>>,
    ctx: Option<*mut flanterm_bindings::flanterm_context>,
}
unsafe impl Send for Terminal<'_> {}
impl<'a> Terminal<'a> {
    pub const fn new() -> Self {
        Terminal {
            fb: None,
            ctx: None,
        }
    }
    pub fn deinit(&mut self) {
        unsafe {
            ((*self.ctx.unwrap()).deinit.unwrap())(self.ctx.unwrap(), None);
        }
    }
    pub fn init(&mut self, f: &Framebuffer) {
        let mut bg = BG;
        let mut fg = FG;
        unsafe {
            let ctx = flanterm_bindings::flanterm_fb_init(
                None,
                None,
                f.addr() as *mut u32,
                f.width().try_into().unwrap(),
                f.height().try_into().unwrap(),
                f.pitch().try_into().unwrap(),
                f.red_mask_size().into(),
                f.red_mask_shift().into(),
                f.green_mask_size().into(),
                f.green_mask_shift().into(),
                f.blue_mask_size().into(),
                f.blue_mask_shift().into(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut bg,
                &mut fg,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                FONT.as_ptr() as *mut c_void,
                8,
                16,
                1,
                1,
                1,
                20,
            );
            self.ctx = Some(ctx);
        }
    }
    pub fn write_string(&mut self, s: &str) {
        unsafe {
            flanterm_bindings::flanterm_write(
                self.ctx.unwrap(),
                s.as_ptr() as *const i8,
                size_of_val(s),
            );
        }
    }
    pub fn clear_screen(&mut self, col: u32) {
        unsafe {
            // cursed
            ((*self.ctx.unwrap()).set_text_bg_rgb.unwrap())(self.ctx.unwrap(), col as u32);
            ((*self.ctx.unwrap()).clear.unwrap())(self.ctx.unwrap(), true);
        }
    }
}
pub static TERMGBL: Mutex<Terminal> = Mutex::new(Terminal::new());
impl<'a> fmt::Write for Terminal<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        for i in s.chars() {
            unsafe {
                core::arch::asm!("out dx, al", in("al") i as u8, in("dx") 0x3F8 as u16, options(nomem, nostack, preserves_flags)); }

        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::term::_print(format_args!($($arg)*)));
}
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    TERMGBL.lock().write_fmt(args).unwrap();
}
pub fn clear_screenterm(col: u32) {
    TERMGBL.lock().clear_screen(col);
}
