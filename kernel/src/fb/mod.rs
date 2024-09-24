pub fn putpixel<'a>(
    buffer: &'a limine::framebuffer::Framebuffer<'a>,
    x: usize,
    y: usize,
    col: u32,
) {
    let calculation = buffer.pitch() as usize * y + (buffer.bpp() as usize / 8) * x;

    unsafe {
        *(buffer.addr().add(calculation) as *mut u32) = col;
    }
}
