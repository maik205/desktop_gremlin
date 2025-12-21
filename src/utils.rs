use std::{collections::HashMap, fs::read_dir, io, path::PathBuf};

use image::{DynamicImage, EncodableLayout};
use sdl3::{
    EventPump, pixels::PixelFormat, rect::{Point, Rect}, sys::mouse::SDL_GetGlobalMouseState
};

use crate::sprite::{GLOBAL_PIXEL_FORMAT, SizeUnit, SpriteError};

pub fn inflate(point: Point, x: u32, y: u32) -> Rect {
    Rect::new(
        (point.x as i32).saturating_sub(x.saturating_div(2) as i32),
        (point.y as i32).saturating_sub(y.saturating_div(2) as i32),
        x,
        y,
    )
}
pub fn get_png_list(
    dir: &str,
    max_depth: u16,
    png_list: &mut HashMap<String, PathBuf>,
) -> Result<(), io::Error> {
    for entry_res in read_dir(dir)? {
        if let Ok(entry) = entry_res {
            if max_depth > 0 {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir()
                        && let Some(path_str) = entry.path().to_str()
                    {
                        // should explode unknowingly
                        let _ = get_png_list(&path_str, max_depth - 1, png_list);
                    } else if ft.is_file()
                        && let Some(file_name) = entry.file_name().to_str()
                        && file_name.ends_with(".png")
                    {
                        png_list.insert(
                            file_name
                                .to_uppercase()
                                .strip_suffix(".PNG")
                                .unwrap()
                                .to_string(),
                            entry.path(),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn calculate_pix_from_parent(
    parent_pix: (u32, u32),
    value: (SizeUnit, SizeUnit),
) -> (u32, u32) {
    let calc: fn(u32, SizeUnit) -> u32 = |parent, unit| match unit {
        SizeUnit::Pixel(value) => value,
        SizeUnit::Percentage(percentage) => (percentage * parent) / 100,
    };
    (calc(parent_pix.0, value.0), calc(parent_pix.1, value.1))
}

pub fn img_get_bytes_global(image: &DynamicImage) -> Result<Vec<u8>, SpriteError> {
    match GLOBAL_PIXEL_FORMAT {
        PixelFormat::RGBA32 => {
            Ok(image.as_rgba8().unwrap().as_bytes().to_vec())
            // .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
            //     Ok(img_buffer.as_bytes().to_vec())
            // })
        }
        PixelFormat::RGB24 => {
            image
                .as_rgb8() // (a: &ImageBuffer<RB....>) => { return Ok(a.as_bytes());}
                .map_or(Err(SpriteError::PixelLoadError), |a| {
                    Ok(a.as_bytes().to_vec())
                })
        }
        _ => image
            .as_rgba8()
            .map_or(Err(SpriteError::PixelLoadError), |a| {
                Ok(a.as_bytes().to_vec())
            }),
    }
}

pub fn get_writer<T: Fn(&mut (u8, u8, u8, u8))>(a: T) -> impl Fn(&mut [u8], usize) {
    move |buffer: &mut [u8], b: usize| {
        let mut i = 0;
        while i + 3 < buffer.len() {
            let mut color_components: (u8, u8, u8, u8) =
                (buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]);
            a(&mut color_components);
            (buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]) = color_components;
            i += 3;
        }
    }
}

pub fn get_cursor_position(event_pump: &EventPump) -> (f32, f32) {
    unsafe {
        let (mut x, mut y): (f32, f32) = (0.0, 0.0);
        let (x_ptr, y_ptr): (*mut f32, *mut f32) = (&mut x, &mut y);
        SDL_GetGlobalMouseState(x_ptr, y_ptr);
        (x, y)
    }
}
