use std::{
    collections::{ HashMap, LinkedList },
    fs::{ self, read_dir },
    io,
    path::{ Path, PathBuf },
};

use anyhow::Result;
use image::{ DynamicImage, EncodableLayout };
use sdl3::{
    Sdl,
    pixels::PixelFormat,
    render::{ Canvas, Texture, TextureCreator },
    video::{ Window, WindowBuilder, WindowContext, WindowFlags },
};

use crate::utils::get_png_list;

const PIXEL_FORMAT: PixelFormat = PixelFormat::ARGB32;

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub column_count: u16,
    pub frame_count: u16,
    pub image: DynamicImage,
    pub filter: LinkedList<ImageFilter>,
}

#[derive(Clone, Copy, Debug)]
pub enum ImageFilter {}

impl SpriteSheet {
    pub fn get_line_count(&self) -> u16 {
        self.frame_count.div_ceil(self.column_count)
    }

    pub fn load_to_texture(&self, texture: &mut Texture) -> Result<(), SpriteError> {
        let bytes = match PIXEL_FORMAT {
            PixelFormat::RGBA32 => {
                self.image
                    .as_rgba8()
                    .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
                        Ok(img_buffer.as_bytes())
                    })
            }
            PixelFormat::RGB24 => {
                self.image
                    .as_rgb8()
                    .map_or(Err(SpriteError::PixelLoadError), |a| { Ok(a.as_bytes()) })
            }
            _ => {
                self.image
                    .as_rgba8()
                    .map_or(Err(SpriteError::PixelLoadError), |a| { Ok(a.as_bytes()) })
            }
        };

        if let Ok(bytes) = bytes {
            return texture
                .update(None, bytes, PIXEL_FORMAT.bytes_per_pixel() * (self.image.width() as usize))
                .map_err(|_| SpriteError::TextureWriteError);
        } else {
            return Err(SpriteError::PixelLoadError);
        }
    }
}

pub enum SpriteError {
    PixelLoadError,
    TextureWriteError,
}

#[derive(Clone, Debug, Hash, Default)]
struct AnimationProperties {
    pub sprite_name: String,
    pub sprite_path: Option<PathBuf>,
    pub sprite_count: u32,
}

impl AnimationProperties {
    pub fn new(name: String, sprite_count: u32) -> AnimationProperties {
        Self {
            sprite_name: name,
            sprite_count,
            sprite_path: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Gremlin {
    name: String,
    // map between animation name and directory
    animation_map: HashMap<String, AnimationProperties>,
    metadata: HashMap<String, String>,
    current_animation: Option<Animation>,
}

pub struct DesktopGremlin {
    sdl: Sdl,
    current_gremlin: Option<Gremlin>,
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
}
pub struct LaunchArguments {
    pub w: u32,
    pub h: u32,
    pub title: String,
    pub window_flags: Vec<WindowFlags>,
}
impl Default for LaunchArguments {
    fn default() -> Self {
        Self {
            w: 500,
            h: 500,
            title: Default::default(),
            window_flags: vec![
                WindowFlags::TRANSPARENT,
                WindowFlags::ALWAYS_ON_TOP,
                WindowFlags::NOT_FOCUSABLE,
                WindowFlags::BORDERLESS
            ],
        }
    }
}
impl LaunchArguments {
    fn window_flags(&self) -> u32 {
        let mut acc = self.window_flags[0];
        for flag in &self.window_flags {
            acc |= *flag;
        }
        acc.as_u32()
    }
}
impl DesktopGremlin {
    pub fn new(launch_arguments: Option<LaunchArguments>) -> Result<DesktopGremlin> {
        let sdl = sdl3::init()?;
        let video = sdl.video()?;
        let launch_arguments = launch_arguments.unwrap_or_default();

        let window = WindowBuilder::new(
            &video,
            &launch_arguments.title,
            launch_arguments.w,
            launch_arguments.h
        )
            .set_window_flags(launch_arguments.window_flags())
            .build()?;

        let canvas = window.into_canvas();

        let texture_creator = canvas.texture_creator();

        Ok(DesktopGremlin { sdl, current_gremlin: None, texture_creator, canvas })
    }

    fn load_gremlin(&mut self, gremlin_txt_path: String) -> Result<Gremlin, GremlinLoadError> {
        let path = Path::new(gremlin_txt_path.as_str());
        let gremlin_txt = fs::read_to_string(path)?;
        let mut gremlin = Gremlin::default();
        for line in gremlin_txt.lines() {
            if line.starts_with("//") {
                continue;
            }
            let split = line.split("=").collect::<Vec<&str>>();
            if split.len() == 2 {
                if split[0].starts_with('.') {
                    match split[0] {
                        ".name" => {
                            gremlin.name = String::from(split[1]);
                        }
                        _ => {
                            gremlin.metadata.insert(split[0].to_string(), split[1].to_string());
                        }
                    }
                    continue;
                }

                if let Ok(count) = split[1].parse::<u32>() {
                    let animation_properties = AnimationProperties::new(
                        split[0].to_string(),
                        count
                    );
                    gremlin.animation_map.insert(split[0].to_string(), animation_properties);
                }
            }
        }
        if let Some(parent) = path.parent() && let Some(parent_path_str) = parent.to_str() {
            let mut png_list = HashMap::new();
            // will error out if i can't get into da directoreas
            get_png_list(parent_path_str, 5, &mut png_list)?;

            // lets consume the map so we don't allocate more memory!
            for (name, path) in png_list.into_iter() {
                if let Some(value) = gremlin.animation_map.get_mut(&name) {
                    let _ = value.sprite_path.insert(path);
                }
            }
            Ok(gremlin)
        } else {
            Err(GremlinLoadError::FsError(None))
        }
    }
}

pub enum GremlinLoadError {
    FsError(Option<io::Error>),
}
impl From<std::io::Error> for GremlinLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::FsError(Some(value))
    }
}

#[derive(Debug, Clone)]
pub struct Animation {
    sprite_sheet: SpriteSheet,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum AnimationKind {
    Walk(DirectionX, DirectionY),
    Intro,
    Idle,
    Exit,
    Hover,
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
