use std::{
    collections::{HashMap, VecDeque},
    fs::read_dir,
    io,
    path::PathBuf,
    rc::Rc,
};

use image::{DynamicImage, EncodableLayout};
use sdl3::{
    pixels::PixelFormat,
    rect::{Point, Rect},
    render::Texture,
    sys::mouse::SDL_GetGlobalMouseState,
    video::Window,
};

use crate::{
    events::MouseButton,
    gremlin::{
        AnimationProperties, Animator, DEFAULT_COLUMN_COUNT, GLOBAL_PIXEL_FORMAT, SizeUnit,
        SpriteError,
    },
};

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

pub fn resize_image_to_window(
    image: DynamicImage,
    window: &Window,
    animation_properties: AnimationProperties,
) -> DynamicImage {
    let scale_factor = (1, 1);
    let (sprite_width, sprite_height) = window.size();
    let (target_width, target_height) = (
        (DEFAULT_COLUMN_COUNT * sprite_width * scale_factor.0) / scale_factor.1,
        (animation_properties
            .sprite_count
            .div_ceil(DEFAULT_COLUMN_COUNT)
            * sprite_height
            * scale_factor.0)
            / scale_factor.1,
    );
    image.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Triangle,
    )
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

pub fn _get_writer<T: Fn(&mut (u8, u8, u8, u8))>(a: T) -> impl Fn(&mut [u8], usize) {
    move |buffer: &mut [u8], _: usize| {
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
/// *SAFETY*: Only use this function when the Sdl context is still in scope and available.
pub fn get_cursor_position() -> (f32, f32) {
    unsafe {
        let (mut x, mut y): (f32, f32) = (0.0, 0.0);
        let (x_ptr, y_ptr): (*mut f32, *mut f32) = (&mut x, &mut y);
        SDL_GetGlobalMouseState(x_ptr, y_ptr);
        (x, y)
    }
}

pub fn get_move_direction(cursor_position: Point, gremlin_rect: Rect) -> (DirectionX, DirectionY) {
    if gremlin_rect.contains_point(cursor_position) {
        return (DirectionX::None, DirectionY::None);
    }

    let dir_x = if cursor_position.x > gremlin_rect.right() {
        DirectionX::Right
    } else if cursor_position.x < gremlin_rect.left() {
        DirectionX::Left
    } else {
        DirectionX::None
    };

    let dir_y = if cursor_position.y < gremlin_rect.top() {
        DirectionY::Up
    } else if cursor_position.y > gremlin_rect.bottom() {
        DirectionY::Down
    } else {
        DirectionY::None
    };
    (dir_x, dir_y)
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum DirectionX {
    None,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Hash)]
pub enum DirectionY {
    None,
    Up,
    Down,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct MouseKeysState {
    pub left: bool,
    pub middle: bool,
    pub right: bool,
}

impl MouseKeysState {
    pub fn set_button(&mut self, button: &MouseButton, state: bool) {
        match button {
            MouseButton::Left => self.left = state,
            MouseButton::Right => self.right = state,
            MouseButton::Middle => self.middle = state,
            _ => {}
        }
    }

    pub fn is_active(&self, button: &MouseButton) -> bool {
        match button {
            MouseButton::Left => self.left,
            MouseButton::Right => self.right,
            MouseButton::Middle => self.middle,
            _ => false,
        }
    }
}

pub fn win_to_rect(window: &Window) -> Rect {
    let (x, y) = window.position();
    let (w, h) = window.size();
    Rect::new(x, y, w, h)
}

#[derive(Default)]
pub struct TextureCache {
    pub data: VecDeque<(String, TextureCacheItem)>,
}
// /
type TextureCacheItem = (Animator, Rc<Texture>);

impl TextureCache {
    // rearrange to purge cache later with a LRU policy
    pub fn rearrange(&mut self, access_index: usize) {
        if let Some(item) = self.data.remove(access_index) {
            self.data.push_back(item);
        }
    }

    pub fn print(&self) {
        let mut res = String::new();
        for (name, rc) in &self.data {
            res += format!(
                "| {} strong:{} weak:{}",
                name,
                Rc::strong_count(&rc.1),
                Rc::weak_count(&rc.1)
            )
            .as_str();
        }
        println!("{}", (res))
    }
    pub fn cache(&mut self, name: String, texture: TextureCacheItem) {
        match &self.data.len() {
            CACHE_CAPACITY.. => {
                self.data.pop_front();
            }
            _ => {}
        };
        self.print();

        self.data.push_back((name, texture));
    }

    pub fn lookup(&self, name: String) -> Option<(usize, TextureCacheItem)> {
        self.data
            .iter()
            .enumerate()
            .rev()
            .find(|a| a.1.0 == name)
            .map(|a| (a.0, a.1.1.clone()))
    }
}

const CACHE_CAPACITY: usize = 10;
