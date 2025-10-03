// use std::ffi::{c_char, c_void};
// use std::ptr;
// use transfer_objects::image::Image;
// use x11::xlib;
// use anyhow::anyhow;
use crate::mutable_image::MutableImage;
use anyhow::{Context, Result};
use subprocess::{Exec, Redirection};

pub fn set_wallpaper(image: &MutableImage, output_name: &str) -> Result<()> {
    // Create a temporary directory to store the image
    let tmp_dir = tempfile::Builder::new()
        .prefix("lifestats_wallpaper_")
        .tempdir()
        .context("Failed to create temporary directory")?;

    let tmp_path = tmp_dir.path().join("current_wallpaper.png");
    image
        .save(&tmp_path)
        .context("Failed to save wallpaper image")?;

    // Set the wallpaper using xwallpaper
    Exec::cmd("xwallpaper")
        .arg("--output")
        .arg(output_name)
        .arg("--zoom")
        .arg(tmp_path.to_str().unwrap())
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .join()
        .context("Failed to set wallpaper using xwallpaper")?;

    Ok(())
}

// pub fn debug_ximage_info(display: *mut xlib::Display, root_window: xlib::Window) {
//     unsafe {
//         // Grab a 1×1 region from the root window
//         let test_ximage = xlib::XGetImage(
//             display,
//             root_window,
//             0,
//             0,
//             1,                 // width = 1
//             1,                 // height = 1
//             !0,                // all planes (i.e., 0xFFFFFFFF)
//             xlib::ZPixmap
//         );
//
//         if test_ximage.is_null() {
//             eprintln!("XGetImage failed: couldn't get a 1×1 image of the root window.");
//         } else {
//             // Inspect XImage fields to see how the server stores pixels
//             let bits_per_pixel = (*test_ximage).bits_per_pixel;
//             let bytes_per_line = (*test_ximage).bytes_per_line;
//             let byte_order = (*test_ximage).byte_order; // 0 = LSBFirst, 1 = MSBFirst
//
//             eprintln!("debug_ximage_info(): bits_per_pixel={}, bytes_per_line={}, byte_order={}",
//                       bits_per_pixel, bytes_per_line, byte_order
//             );
//
//             // Clean up
//             xlib::XDestroyImage(test_ximage);
//         }
//     }
// }

// pub fn set_wallpaper(image: &Image, output_name: &str) -> Result<()> {
//     // Open the X11 display
//     let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
//     if display.is_null() {
//         return Err(anyhow!("Could not open X Display."));
//     }
//
//     // Get the default screen and its root window
//     let screen = unsafe { xlib::XDefaultScreen(display) };
//     let root_window = unsafe { xlib::XRootWindow(display, screen) };
//
//     // Query the display's depth and visual
//     let depth = unsafe { xlib::XDefaultDepth(display, screen) };
//     let visual = unsafe { xlib::XDefaultVisual(display, screen) };
//
//     // Because bits_per_pixel=32, we need 4 bytes/pixel, not 3.
//     let width = image.width();
//     let height = image.height();
//     let rgb_buffer = image.get_rgb_buffer(); // 3*bpp data: [r0, g0, b0, r1, g1, b1, ...]
//
//     // 4) Allocate 4-byte-per-pixel buffer with libc::malloc
//     let num_pixels = width * height;
//     let buf_size = num_pixels * 4; // 4 BPP
//     let data_ptr = unsafe { libc::malloc(buf_size) } as *mut u8;
//     if data_ptr.is_null() {
//         unsafe { xlib::XCloseDisplay(display); }
//         return Err(anyhow!("Failed to allocate memory with libc::malloc."));
//     }
//
//     // 5) Copy data from RGB => BGRA (or RGBA) into that new buffer
//     //    On a little-endian 32-bpp X server, memory is typically B, G, R, A.
//     unsafe {
//         let mut out_ptr = data_ptr;
//         for chunk in rgb_buffer.chunks_exact(3) {
//             let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
//             // Write BGRA:
//             ptr::write(out_ptr,     b); out_ptr = out_ptr.add(1);
//             ptr::write(out_ptr,     g); out_ptr = out_ptr.add(1);
//             ptr::write(out_ptr,     r); out_ptr = out_ptr.add(1);
//             ptr::write(out_ptr, 0x00); out_ptr = out_ptr.add(1); // alpha=0
//         }
//     }
//
//     // 6) Create XImage. bits_per_pixel=32, bitmap_pad=32
//     let bytes_per_line = (width * 4) as i32;
//     let ximage = unsafe {
//         xlib::XCreateImage(
//             display,
//             visual,
//             depth as u32,
//             xlib::ZPixmap,
//             0,
//             data_ptr as *mut c_char,
//             width as u32,
//             height as u32,
//             32,
//             bytes_per_line,
//         )
//     };
//
//     if ximage.is_null() {
//         // XCreateImage failed. We must free the buffer ourselves.
//         unsafe {
//             libc::free(data_ptr as *mut c_void);
//             xlib::XCloseDisplay(display);
//         }
//         return Err(anyhow!("XCreateImage failed."));
//     }
//
//     // 7) Pixmap + GC
//     let pixmap = unsafe {
//         xlib::XCreatePixmap(display, root_window, width as u32, height as u32, depth as u32)
//     };
//     let gc = unsafe { xlib::XCreateGC(display, pixmap, 0, ptr::null_mut()) };
//     if gc.is_null() {
//         // Clean up
//         unsafe {
//             xlib::XDestroyImage(ximage);
//             xlib::XCloseDisplay(display);
//         }
//         return Err(anyhow!("XCreateGC failed."));
//     }
//
//     // 8) XPutImage
//     unsafe {
//         xlib::XPutImage(
//             display,
//             pixmap,
//             gc,
//             ximage,
//             0,  // src_x
//             0,  // src_y
//             0,  // dst_x
//             0,  // dst_y
//             width as u32,
//             height as u32,
//         );
//     }
//
//     // 9) Destroy XImage
//     unsafe {
//         // This calls default ximage->f.destroy_image, which calls free(data_ptr).
//         // So we do NOT free(data_ptr) ourselves again.
//         xlib::XDestroyImage(ximage);
//     }
//
//     // 10) Set root background
//     unsafe {
//         xlib::XSetWindowBackgroundPixmap(display, root_window, pixmap);
//         xlib::XClearWindow(display, root_window);
//     }
//
//     // 11) Cleanup
//     unsafe {
//         xlib::XFreeGC(display, gc);
//         // Don’t free the pixmap
//         xlib::XCloseDisplay(display);
//     }
//
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use crate::mutable_image::MutableImage;
    use crate::set_wallpaper::set_wallpaper;
    use image::Rgb;

    fn create_test_image(width: usize, height: usize, color: Rgb<u8>) -> MutableImage {
        let mut buffer = Vec::with_capacity(width * height * 3);
        for _ in 0..(width * height) {
            buffer.extend_from_slice(&[color.0[0], color.0[1], color.0[2]]);
        }

        MutableImage::new_from_buffer(width, height, buffer.into_boxed_slice()).unwrap()
    }

    #[ignore]
    #[test]
    pub fn can_set_wallpaper() {
        let background = create_test_image(3840, 1080, Rgb([255, 0, 0])); // Bright red
        set_wallpaper(&background, "HDMI-1").unwrap();
    }
}
