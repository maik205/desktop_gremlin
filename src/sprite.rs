use std::{
    collections::{ HashMap, LinkedList },
    env,
    fs::{ self, read_dir },
    io,
    path::{ Path, PathBuf },
    str::FromStr,
    thread,
    time::Duration,
};

use anyhow::Result;
use bad_signals::signals::{ common::Signalable, signals::Signal };
use image::{ DynamicImage, EncodableLayout };
// absolutely goated.
use sdl3::{
    // might move to winit & wgpu but,... ehhhhhhhhh too lazy.... i love sdl

    Sdl,
    event::{ Event },
    pixels::{ Color, PixelFormat },
    rect::{ Point, Rect },
    render::{ Canvas, FRect, Texture, TextureCreator },
    video::{ Window, WindowBuilder, WindowContext, WindowFlags },
};
pub const GLOBAL_PIXEL_FORMAT: PixelFormat = PixelFormat::RGB24;

use crate::{
    ui::{ Component, Div, Render, RenderStyle, UI, compose, div },
    utils::{ calculate_pix_from_parent, get_png_list },
};

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
        let bytes = match GLOBAL_PIXEL_FORMAT {
            PixelFormat::RGBA32 => {
                self.image
                    .as_rgba8()
                    .map_or(Err(SpriteError::PixelLoadError), |img_buffer| {
                        Ok(img_buffer.as_bytes())
                    })
            }
            PixelFormat::RGB24 => {
                self.image
                    .as_rgb8() // (a: &ImageBuffer<RB....>) => { return Ok(a.as_bytes());}
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
                .update(
                    None,
                    bytes,
                    GLOBAL_PIXEL_FORMAT.bytes_per_pixel() * (self.image.width() as usize)
                )
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

impl LaunchArguments {
    pub fn parse_from_args(args: env::Args) {
        let mut launch_args = LaunchArguments::default();
        let args = args.collect::<Vec<String>>();
        for mut i in 0..args.len() {
            if args[i].starts_with('-') {
                match args[i].as_str() {
                    "-w" => {
                        launch_args.w = FromStr::from_str(args[i + 1].as_str()).unwrap_or(500);
                        i += 1;
                    }
                    "-h" => {
                        launch_args.h = FromStr::from_str(args[i + 1].as_str()).unwrap_or(500);
                        i += 1;
                    }
                    "-t" => {
                        launch_args.title = args[i + 1].clone();
                        i += 1;
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Default for LaunchArguments {
    fn default() -> Self {
        Self {
            w: 500,
            h: 500,
            title: String::from("Desktop Gremlin!"),
            window_flags: vec![
                WindowFlags::TRANSPARENT,
                WindowFlags::ALWAYS_ON_TOP,
                WindowFlags::NOT_FOCUSABLE
                // WindowFlags::BORDERLESS
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

        Ok(DesktopGremlin {
            sdl,
            current_gremlin: None,
            texture_creator,
            canvas,
        })
    }

    // spins up teh event lop
    pub fn go(mut self) {
        let (window_w, window_h) = self.canvas.window().size();

        let mut ui = UI::default();

        let mut child_div = Div::new();

        child_div.styles = vec![RenderStyle::BackgroundColor(Color::RED)].into();

        let component = Component::new(child_div);

        component.set_preferred_size((SizeUnit::Percentage(50), SizeUnit::Percentage(50)));
        // component tree woks!1!
        ui.root = ui.root
            .set_preferred_size((SizeUnit::Percentage(100), SizeUnit::Percentage(100)))
            .add_children(
                vec![
                    div().add_child(
                        compose(Div::default().style(RenderStyle::BackgroundColor(Color::RED)))
                            .set_preferred_size((
                                SizeUnit::Percentage(50),
                                SizeUnit::Percentage(50),
                            ))
                            .add_child(
                                compose(
                                    Div::default().style(RenderStyle::BackgroundColor(Color::BLUE))
                                ).set_preferred_size(SizeUnit::percentage(50, 50))
                            )
                    )
                ]
            );
        let mut texture = self.texture_creator
            .create_texture_streaming(GLOBAL_PIXEL_FORMAT, 500, 500)
            .unwrap();
        let _ = ui.render(&mut texture, None);
        let _ = self.canvas.copy(&texture, None, None);
        // UI rendering logic i guess
        let mut button = Button::default();
        button.on_click.subscribe(|_| {
            println!("Button clicked");
        });
        // let render_rect_size = calculate_pix_from_parent(
        //     (window_w, window_h),
        //     (button.width, button.height)
        // );

        // println!("{:?}", render_rect_size);
        // let render_rect = {
        //     Rect::new(
        //         0,
        //         window_h.saturating_sub(render_rect_size.1) as i32,
        //         render_rect_size.0,
        //         render_rect_size.1
        //     )
        // };
        // button
        //     .render_canvas(
        //         &mut self.canvas,
        //         Some(into_frect(render_rect))
        //         // Some(vec![RenderStyle::BackgroundColor(Color::BLACK)])
        //     )
        //     .unwrap();
        self.canvas.present();
        let mut should_exit = false;

        loop {
            if should_exit {
                break;
            }
            for event in self.sdl.event_pump().unwrap().poll_iter() {
                match event {
                    Event::Quit { .. } => {
                        should_exit = true;
                    }
                    Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                        match mouse_btn {
                            sdl3::mouse::MouseButton::Left => {
                                // if render_rect.contains_point(Point::new(x as i32, y as i32)) {
                                //     button.on_click.set(());
                                // }
                            }
                            _ => (),
                        }
                    }
                    _ => {}
                }
            }
            // thread::sleep(Duration::from_millis(500));
        }
    }

    fn load_gremlin(&mut self, gremlin_txt_path: String) -> Result<Gremlin, GremlinLoadError> {
        let path = Path::new(gremlin_txt_path.as_str());
        let gremlin_txt = fs::read_to_string(path)?;
        let mut gremlin = Gremlin::default();
        for line in gremlin_txt.lines() {
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

// impl Into<Rect> for FRect {
pub fn into_rect(f_rect: FRect) -> Rect {
    Rect::new(f_rect.x as i32, f_rect.y as i32, f_rect.w as u32, f_rect.h as u32)
}
pub fn into_opt_rect(f_rect: Option<FRect>) -> Option<Rect> {
    if let Some(f_rect) = f_rect {
        return Some(Rect::new(f_rect.x as i32, f_rect.y as i32, f_rect.w as u32, f_rect.h as u32));
    }
    None
}
pub fn into_frect(rect: Rect) -> FRect {
    FRect { x: rect.x as f32, y: rect.y as f32, w: rect.w as f32, h: rect.h as f32 }
}
// }

pub struct Button {
    color: Color,
    width: SizeUnit,
    height: SizeUnit,
    on_click: Signal<()>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: SizeUnit::Percentage(100),
            height: SizeUnit::Pixel(100),
            on_click: Signal::new(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SizeUnit {
    Pixel(u32),
    Percentage(u32),
}

impl SizeUnit {
    pub fn pix(w: u32, h: u32) -> (SizeUnit, SizeUnit) {
        (SizeUnit::Pixel(w), SizeUnit::Pixel(h))
    }
    pub fn percentage(w: u32, h: u32) -> (SizeUnit, SizeUnit) {
        (SizeUnit::Percentage(w), SizeUnit::Percentage(h))
    }
}

impl Render for Button {
    fn render(
        &self,
        texture: &mut Texture,
        rect: Option<FRect>
        // styles: Option<Vec<RenderStyle>>
    ) -> Result<()> {
        let _ = texture.with_lock(into_opt_rect(rect), |buf, _| {
            for i in 0..buf.len() {
                match i % 4 {
                    0 => {
                        buf[i] = self.color.r;
                    }
                    1 => {
                        buf[i] = self.color.g;
                    }
                    2 => {
                        buf[i] = self.color.b;
                    }
                    3 => {
                        buf[i] = self.color.a;
                    }
                    _ => {}
                }
            }
        });
        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>
        // styles: Option<Vec<RenderStyle>>
    ) -> Result<()> {
        let color = canvas.draw_color();
        canvas.set_draw_color(self.color);
        canvas.fill_rect(rect).unwrap();
        canvas.set_draw_color(color);

        Ok(())
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
