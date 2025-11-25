use std::{ collections::HashMap, fs::{ DirEntry, read_dir }, io, path::PathBuf };

use sdl3::rect::Rect;

use crate::sprite::SizeUnit;

pub trait Inflate {
    // Inflate around a center
    fn inflate(&self, x: u32, y: u32) -> Rect;
}

pub type Point = (i32, i32);

impl Inflate for Point {
    fn inflate(&self, x: u32, y: u32) -> Rect {
        Rect::new(
            (x as i32).saturating_sub((x as i32).saturating_div(2)),
            (y as i32).saturating_sub((y as i32).saturating_div(2)),
            x,
            y
        )
    }
}
pub fn get_png_list(
    dir: &str,
    max_depth: u16,
    png_list: &mut HashMap<String, PathBuf>
) -> Result<(), io::Error> {
    for entry_res in read_dir(dir)? {
        if let Ok(entry) = entry_res {
            if max_depth > 0 {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() && let Some(path_str) = entry.path().to_str() {
                        // should explode unknowingly
                        let _ = get_png_list(&path_str, max_depth - 1, png_list);
                    } else if
                        ft.is_file() &&
                        let Some(file_name) = entry.file_name().to_str() &&
                        file_name.ends_with(".png")
                    {
                        png_list.insert(file_name.to_lowercase(), entry.path());
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn calculate_pix_from_parent(
    parent_pix: (u32, u32),
    value: (SizeUnit, SizeUnit)
) -> (u32, u32) {
    let calc: fn(u32, SizeUnit) -> u32 = |parent, unit| {
        match unit {
            SizeUnit::Pixel(value) => value,
            SizeUnit::Percentage(percentage) => (percentage * parent) / 100,
        }
    };
    (calc(parent_pix.0, value.0), calc(parent_pix.1, value.1))
}
