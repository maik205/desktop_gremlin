use std::{
    collections::{HashMap, LinkedList, VecDeque},
    env,
    fs::{self},
    io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

use anyhow::Result;
use image::{DynamicImage, EncodableLayout};
// absolutely goated.
use sdl3::{
    // might move to winit & wgpu but,... ehhhhhhhhh too lazy.... i love sdl
    Sdl,
    pixels::PixelFormat,
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    sys::{
        properties::SDL_GetPointerProperty,
        video::{SDL_GetWindowProperties, SDL_PROP_WINDOW_WIN32_HWND_POINTER},
    },
    video::{Window, WindowBuilder, WindowContext, WindowFlags},
};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongW, LWA_COLORKEY, SetLayeredWindowAttributes, SetWindowLongW,
        WS_EX_LAYERED,
    },
};

pub const GLOBAL_PIXEL_FORMAT: PixelFormat = PixelFormat::RGBA32;

use crate::utils::get_png_list;

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

    pub fn into_texture(
        &self,
        texture_creator: &TextureCreator<WindowContext>,
    ) -> Result<Texture, SpriteError> {
        let bytes = match GLOBAL_PIXEL_FORMAT {
            PixelFormat::RGBA32 => self
                .image
                .as_rgba8()
                .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
                    Ok(img_buffer.as_bytes())
                }),
            PixelFormat::RGB24 => {
                self.image
                    .as_rgb8() // (a: &ImageBuffer<RB....>) => { return Ok(a.as_bytes());}
                    .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
                        Ok(img_buffer.as_bytes())
                    })
            }
            _ => self
                .image
                .as_rgba8()
                .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
                    Ok(img_buffer.as_bytes())
                }),
        };

        if let Ok(bytes) = bytes {
            let mut texture = texture_creator
                .create_texture_static(GLOBAL_PIXEL_FORMAT, self.image.width(), self.image.height())
                .map_err(|_| SpriteError::TextureWriteError)?;

            texture
                .update(
                    None,
                    bytes,
                    GLOBAL_PIXEL_FORMAT.bytes_per_pixel() * (self.image.width() as usize),
                )
                .map_err(|_| SpriteError::TextureWriteError)?;

            Ok(texture)
        } else {
            return Err(SpriteError::PixelLoadError);
        }
    }

    pub fn sprite_size(&self) -> (u32, u32) {
        (
            self.image.width().saturating_div(self.column_count as u32),
            self.image
                .height()
                .saturating_div(self.get_line_count() as u32),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SpriteError {
    PixelLoadError,
    TextureWriteError,
}

#[derive(Clone, Debug, Hash, Default)]
pub struct AnimationProperties {
    pub animation_name: String,
    pub sprite_path: Option<PathBuf>,
    pub sprite_count: u32,
}

impl AnimationProperties {
    pub fn new(name: String, sprite_count: u32) -> AnimationProperties {
        Self {
            animation_name: name,
            sprite_count,
            sprite_path: None,
        }
    }
}

impl Animation {
    pub fn get_frame_rect(&self) -> Rect {
        let (sprite_width, sprite_height) = self.sprite_sheet.sprite_size();
        Rect::new(
            (((self.current_frame % self.sprite_sheet.column_count) as u32) * sprite_width) as i32,
            (((self.current_frame / self.sprite_sheet.column_count) as u32) * sprite_height) as i32,
            sprite_width,
            sprite_height,
        )
    }
}

impl TryInto<Animation> for &AnimationProperties {
    type Error = GremlinLoadError;

    fn try_into(self) -> std::result::Result<Animation, Self::Error> {
        if let Some(path) = &self.sprite_path
            && let Ok(image) = image::open(path)
        {
            let sprite_sheet = SpriteSheet {
                column_count: 10,
                frame_count: self.sprite_count as u16,
                image,
                filter: Default::default(),
            };
            return std::result::Result::Ok(Animation {
                sprite_sheet,
                current_frame: 0,
                properties: self.clone(),
            });
        }
        Err(GremlinLoadError::FsError(None))
    }
}

#[derive(Default)]
pub struct Gremlin {
    pub name: String,
    // map between animation name and directory
    pub animation_map: HashMap<String, AnimationProperties>,
    pub metadata: HashMap<String, String>,
    pub animator: Option<Animator>,
}

pub struct DesktopGremlin {
    pub sdl: Sdl,
    pub current_gremlin: Option<Gremlin>,
    pub canvas: Canvas<Window>,
    pub should_exit: Arc<Mutex<bool>>,
    // pub texture_cache: Arc<Mutex<TextureCache<'a>>>,
    pub task_queue: VecDeque<GremlinTask>,
    pub task_channel: (Sender<GremlinTask>, Receiver<GremlinTask>),
    pub should_check_for_action: bool,
}

pub struct LaunchArguments {
    pub w: u32,
    pub h: u32,
    pub title: String,
    pub window_flags: Vec<WindowFlags>,
}

pub const GLOBAL_FRAMERATE: u32 = 48;

impl LaunchArguments {
    pub fn _parse_from_args(args: env::Args) {
        let mut launch_args = LaunchArguments::default();
        let args = args.collect::<Vec<String>>();
        let mut i = 0;
        while i < args.len() {
            if args[i].starts_with('-') {
                match args[i].as_str() {
                    "-w" => {
                        launch_args.w = FromStr::from_str(args[i + 1].as_str()).unwrap_or(200);
                        i += 1;
                    }
                    "-h" => {
                        launch_args.h = FromStr::from_str(args[i + 1].as_str()).unwrap_or(200);
                        i += 1;
                    }
                    "-t" => {
                        launch_args.title = args[i + 1].clone();
                        i += 1;
                    }
                    _ => {}
                }
            }
            i += 1;
        }
    }
}

impl Default for LaunchArguments {
    fn default() -> Self {
        Self {
            w: 150,
            h: 150,
            title: String::from("Desktop Gremlin!"),
            window_flags: vec![
                WindowFlags::TRANSPARENT,
                WindowFlags::ALWAYS_ON_TOP,
                WindowFlags::NOT_FOCUSABLE,
                WindowFlags::BORDERLESS,
            ],
        }
    }
}
impl LaunchArguments {
    fn window_flags(&self) -> u32 {
        if self.window_flags.len() == 0 {
            return 0;
        }
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
            launch_arguments.h,
        )
        .set_window_flags(launch_arguments.window_flags())
        .build()?;

        #[cfg(target_os = "windows")]
        unsafe {
            let sdl_props = SDL_GetWindowProperties(window.raw());
            let hwnd = SDL_GetPointerProperty(
                sdl_props,
                SDL_PROP_WINDOW_WIN32_HWND_POINTER,
                std::ptr::null_mut(),
            );

            let hwnd = HWND(hwnd);

            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | (WS_EX_LAYERED.0 as i32));

            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0x00000000), 255, LWA_COLORKEY);
        }

        let canvas = window.into_canvas();

        Ok(DesktopGremlin {
            sdl,
            current_gremlin: None,
            canvas,
            should_exit: Arc::new(Mutex::new(false)),
            // texture_cache: Default::default(),
            task_queue: Default::default(),
            task_channel: mpsc::channel(),
            should_check_for_action: true,
        })
    }

    pub fn load_gremlin(&mut self, gremlin_txt_path: String) -> Result<Gremlin, GremlinLoadError> {
        let path = Path::new(gremlin_txt_path.as_str());
        let gremlin_txt = fs::read_to_string(path)?;
        let mut gremlin = Gremlin::default();
        for line in gremlin_txt.lines() {
            // skip comments
            if line.starts_with("//") {
                continue;
            }
            let split = line.split('=').collect::<Vec<&str>>();
            if split.len() == 2 {
                if split[0].starts_with('.') {
                    match split[0] {
                        ".name" => {
                            gremlin.name = String::from(split[1]);
                        }
                        _ => {
                            gremlin
                                .metadata
                                .insert(split[0].to_string(), split[1].to_string());
                        }
                    }
                    continue;
                }

                if let Ok(count) = split[1].parse::<u32>() {
                    let animation_properties =
                        AnimationProperties::new(split[0].to_string(), count);
                    gremlin
                        .animation_map
                        .insert(split[0].to_string(), animation_properties);
                }
            }
        }
        if let Some(parent) = path.parent()
            && let Some(parent_path_str) = parent.to_str()
        {
            let mut png_list = HashMap::new();
            // will error out if i can't get into da directories
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GremlinTask {
    Play(String),
    PlayInterrupt(String),
}

#[derive(Debug)]
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
    pub sprite_sheet: SpriteSheet,
    pub current_frame: u16,
    pub properties: AnimationProperties,
}

#[derive(Default, Clone, Hash, Debug)]
pub struct Animator {
    pub current_frame: u32,
    pub texture_size: (u32, u32),
    pub sprite_size: (u32, u32),
    pub animation_properties: AnimationProperties,
    pub column_count: u32,
}

pub const DEFAULT_COLUMN_COUNT: u32 = 10;

impl TryFrom<&AnimationProperties> for Animator {
    type Error = ();

    fn try_from(value: &AnimationProperties) -> std::result::Result<Self, Self::Error> {
        if let Some(ref path) = value.sprite_path
            && let Ok(image_data) = image::open(path).map_err(|_| Err::<Self, ()>(()))
        {
            return Ok(Animator {
                current_frame: Default::default(),
                texture_size: (image_data.width(), image_data.height()),
                animation_properties: value.clone(),
                column_count: DEFAULT_COLUMN_COUNT,
                sprite_size: (
                    image_data.width().div_ceil(DEFAULT_COLUMN_COUNT),
                    image_data
                        .height()
                        .div_ceil(value.sprite_count.div_ceil(DEFAULT_COLUMN_COUNT)),
                ),
            });
        }
        Err(())
    }
}

impl From<&Animation> for Animator {
    fn from(value: &Animation) -> Self {
        Self {
            current_frame: Default::default(),
            texture_size: (
                value.sprite_sheet.image.width(),
                value.sprite_sheet.image.height(),
            ),
            sprite_size: (
                value
                    .sprite_sheet
                    .image
                    .width()
                    .div_ceil(DEFAULT_COLUMN_COUNT),
                value
                    .sprite_sheet
                    .image
                    .height()
                    .div_ceil(value.properties.sprite_count.div_ceil(DEFAULT_COLUMN_COUNT)),
            ),
            animation_properties: value.properties.clone(),
            column_count: DEFAULT_COLUMN_COUNT,
        }
    }
}

impl Animator {
    pub fn get_frame_rect(&self) -> Rect {
        let (sprite_width, sprite_height) = self.sprite_size;
        Rect::new(
            (((self.current_frame % self.column_count) as u32) * sprite_width) as i32,
            (((self.current_frame / self.column_count) as u32) * sprite_height) as i32,
            sprite_width,
            sprite_height,
        )
    }
}
