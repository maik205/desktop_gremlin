use std::{
    collections::{HashMap, HashSet, LinkedList, VecDeque},
    env,
    ffi::c_void,
    fs::{self},
    io,
    marker::PhantomData,
    path::{Path, PathBuf},
    ptr::null_mut,
    rc::Rc,
    str::FromStr,
    sync::{
        Arc, LazyLock, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::Result;
use bad_signals::signals::{common::Signalable, signals::Signal};
use image::{DynamicImage, EncodableLayout};
// absolutely goated.
use sdl3::{
    // might move to winit & wgpu but,... ehhhhhhhhh too lazy.... i love sdl
    Sdl,
    event::Event as SdlEvent,
    pixels::{Color, PixelFormat},
    rect::{Point, Rect},
    render::{Canvas, FRect, Texture, TextureCreator},
    sys::{
        properties::SDL_GetPointerProperty,
        rect::SDL_Point,
        video::{
            SDL_GetWindowProperties, SDL_HitTestResult, SDL_PROP_WINDOW_WIN32_HWND_POINTER,
            SDL_SetWindowHitTest, SDL_Window,
        },
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

use crate::{
    behavior::Behavior,
    events::Event,
    io::AsyncAnimationLoader,
    ui::{Component, Render, div},
    utils::{
        DirectionX, DirectionY, get_cursor_position, get_move_direction, get_png_list,
        resize_image_to_window, win_to_rect,
    },
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
    pub texture_cache: TextureCache,
    pub texture: Option<Rc<Texture>>,
}

pub struct DesktopGremlin {
    pub sdl: Sdl,
    pub current_gremlin: Option<Gremlin>,
    pub canvas: Canvas<Window>,
    pub should_exit: Arc<Mutex<bool>>,
    pub display_context: DisplayContext,
    pub async_loader: AsyncAnimationLoader,
    // pub texture_cache: Arc<Mutex<TextureCache<'a>>>,
    pub task_queue: VecDeque<GremlinTask>,
    pub task_channel: (Sender<GremlinTask>, Receiver<GremlinTask>),
    pub behaviors: Vec<Box<dyn Behavior>>,
    pub texture_creator: TextureCreator<WindowContext>,
    should_check_for_action: bool,
}

pub struct DisplayContext {
    pub usable_bounds: Rect,
}
pub struct LaunchArguments {
    pub w: u32,
    pub h: u32,
    pub title: String,
    pub window_flags: Vec<WindowFlags>,
}

pub const GLOBAL_FRAMERATE: u32 = 48;

impl LaunchArguments {
    pub fn parse_from_args(args: env::Args) {
        let mut launch_args = LaunchArguments::default();
        let args = args.collect::<Vec<String>>();
        for mut i in 0..args.len() {
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
        }
    }
}

impl Default for LaunchArguments {
    fn default() -> Self {
        Self {
            w: 200,
            h: 200,
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
unsafe extern "C" fn all_drag(
    _: *mut SDL_Window,
    _: *const SDL_Point,
    _: *mut c_void,
) -> SDL_HitTestResult {
    SDL_HitTestResult::NORMAL
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

        // window.set_mouse_grab(true);

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

        unsafe {
            SDL_SetWindowHitTest(window.raw(), Some(all_drag), null_mut());
        }

        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();

        let usable_bounds = video.get_primary_display()?.get_usable_bounds()?;

        let d_gremlin = DesktopGremlin {
            sdl,
            current_gremlin: None,
            texture_creator,
            canvas,
            should_exit: Arc::new(Mutex::new(false)),
            display_context: DisplayContext { usable_bounds },
            async_loader: Default::default(),
            // texture_cache: Default::default(),
            task_queue: Default::default(),
            task_channel: mpsc::channel(),
            behaviors: Vec::new(),
            should_check_for_action: true,
        };

        Ok(d_gremlin)
    }

    pub fn register_behavior(&mut self, behavior: Box<dyn Behavior>) {
        self.behaviors.push(behavior);
    }
    pub fn register_behaviors(&mut self, behavior: Vec<Box<dyn Behavior>>) {
        let mut behavior = behavior;
        self.behaviors.append(&mut behavior);
    }

    // spins up teh event lop
    pub fn go<'texture>(mut self) {
        let should_exit = Arc::new(Mutex::new(false));

        let texture_cache: Arc<Mutex<TextureCache>> = Default::default();
        let mut gremlin_texture: Option<Rc<Texture>> = None;

        let should_exit_tasketeer = Arc::clone(&should_exit);
        let (task_tx, _) = &self.task_channel;
        let task_tx_2 = task_tx.clone();
        let task_tx_1 = task_tx.clone();

        let gremlin_tasketeer = thread::spawn(move || {
            let mut rng = rand::rng();
            let should_exit = should_exit_tasketeer;
            let task_tx = task_tx_1;
            let _ = task_tx.send(GremlinTask::Play("INTRO".to_string()));
            let _ = task_tx.send(GremlinTask::Play("IDLE".to_string()));

            // will write the AI™™™™ here soon™™™™
            thread::sleep(Duration::from_millis(2000));

            while *should_exit.lock().unwrap() == false {
                thread::sleep(Duration::from_millis(200));
            }
            // let _ = task_tx.send(GremlinTask::Goto(mx as i32, my as i32));
        });

        self.current_gremlin = self
            .load_gremlin(
                r"C:\Users\ASUS\Documents\Projects\desktop_gremlin\assets\Gremlins\Mambo\config.txt".to_string()
            )
            .ok();

        let mut should_check_for_action = true;

        let mut move_target: Option<Point> = None;
        let velocity = 250.0;
        let mut current_animation_name = String::new();
        let mut is_dragging = false;
        let mut is_lmb_down = false;
        let (mut drag_start_x, mut drag_start_y) = (0.0, 0.0);
        let mut event_pump = self.sdl.event_pump().unwrap();
        let mut last_moved_at = Instant::now();

        let mut move_towards_cursor = false;
        let mut should_check_drag = false;

        let (mut gremlin_x, mut gremlin_y) = (0, 0);
        loop {
            let mut event_set = HashSet::new();
            while let Some(event) = event_pump.poll_event() {
                match event {
                    SdlEvent::Quit { .. } => {
                        let _ = task_tx_2.send(GremlinTask::PlayInterrupt("OUTRO".to_string()));
                    }
                    SdlEvent::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                        sdl3::mouse::MouseButton::Left => {
                            is_lmb_down = true;
                        }
                        _ => (),
                    },
                    SdlEvent::MouseMotion { x, y, .. } => {
                        if is_lmb_down && !is_dragging {
                            is_dragging = true;
                            let _ = task_tx_2.send(GremlinTask::PlayInterrupt("GRAB".to_string()));
                            let _ = &self.task_queue.clear();
                            (drag_start_x, drag_start_y) = (x, y);
                        }
                        if is_dragging && should_check_drag {
                            let (gremlin_x, gremlin_y) = get_window_pos(&self.canvas);
                            self.canvas.window_mut().set_position(
                                sdl3::video::WindowPos::Positioned(
                                    gremlin_x.saturating_add((x - drag_start_x) as i32),
                                ),
                                sdl3::video::WindowPos::Positioned(
                                    gremlin_y.saturating_add((y - drag_start_y) as i32),
                                ),
                            );
                        }
                        // only move every odd frame because moving the window will trigger another mousemove SdlEvent
                        should_check_drag = !should_check_drag;
                    }

                    SdlEvent::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                        sdl3::mouse::MouseButton::Left => {
                            if !is_dragging && is_lmb_down {
                                let _ =
                                    task_tx_2.send(GremlinTask::PlayInterrupt("CLICK".to_string()));
                                move_towards_cursor = !move_towards_cursor;
                                last_moved_at = Instant::now();
                            }
                            if is_dragging && is_lmb_down {
                                let _ =
                                    task_tx_2.send(GremlinTask::PlayInterrupt("PAT".to_string()));
                            }
                            let _ = task_tx_2.send(GremlinTask::Play("IDLE".to_string()));
                            is_dragging = false;
                            is_lmb_down = false;
                        }
                        _ => (),
                    },
                    SdlEvent::Window { win_event, .. } => match win_event {
                        sdl3::event::WindowEvent::Moved(x, y) => {
                            (gremlin_x, gremlin_y) = (x, y);
                        }
                        _ => {}
                    },

                    _ => {}
                }
                event_set.insert(Event::from(event));
            }
            // thread::sleep(Duration::from_millis(500));

            // handle gremlin movement
            let gremlin_position = Point::new(
                gremlin_x + ((self.canvas.window().size().0 / 2) as i32),
                gremlin_y + ((self.canvas.window().size().1 / 2) as i32),
            );
            if !is_dragging
                && move_towards_cursor
                && let Some(move_target) = move_target
            {
                let (dir_x, dir_y) = get_move_direction(move_target, {
                    let mut win_rect = win_to_rect(self.canvas.window());
                    if win_rect.contains_point(move_target) {
                        win_rect.resize(win_rect.width() + 100, win_rect.height() + 100);
                        println!("{:?}", win_rect);
                    }
                    win_rect
                });
                let tan = ((gremlin_position.y - move_target.y) as f32)
                    / ((gremlin_position.x - move_target.x) as f32);
                let alpha = tan.atan();

                let (velo_x, x_anim) = match dir_x {
                    DirectionX::None => (0.0, ""),
                    DirectionX::Left => (-velocity, "LEFT"),
                    DirectionX::Right => (velocity, "RIGHT"),
                };
                let (velo_y, y_anim) = match dir_y {
                    DirectionY::None => (0.0, ""),
                    DirectionY::Up => (-velocity, "UP"),
                    DirectionY::Down => (velocity, "DOWN"),
                };

                let animation_name = match (dir_x, dir_y) {
                    (DirectionX::None, DirectionY::None) => "RUNIDLE".to_string(),
                    (DirectionX::None, _) => "RUN".to_string() + y_anim,
                    (_, DirectionY::None) => "RUN".to_string() + x_anim,
                    (_, _) => y_anim.to_string() + x_anim,
                };
                if current_animation_name != animation_name {
                    let _ = task_tx_2.send(GremlinTask::PlayInterrupt(animation_name));
                    &self.task_queue.clear();
                }

                let (velo_x, velo_y) = (velo_x * alpha.cos().abs(), velo_y * alpha.sin().abs());

                self.canvas.window_mut().set_position(
                    sdl3::video::WindowPos::Positioned(
                        ((gremlin_x as f32) + velo_x * last_moved_at.elapsed().as_secs_f32())
                            as i32,
                    ),
                    sdl3::video::WindowPos::Positioned(
                        ((gremlin_y as f32) + velo_y * last_moved_at.elapsed().as_secs_f32())
                            as i32,
                    ),
                );
                last_moved_at = Instant::now();
            }

            let mut task_board = None;

            // check for tasks and append to task queue
            while let Ok(task) = self.task_channel.1.try_recv() {
                if let GremlinTask::PlayInterrupt(_) = &task {
                    task_board = Some(task);
                    break;
                }
                &self.task_queue.push_back(task);
            }

            if let None = task_board
                && should_check_for_action
            {
                task_board = self.task_queue.pop_front();
            }

            let mut cache_hit_index: Option<usize> = None;
            if let Some(task_board) = task_board
                && let Some(gremlin) = &mut self.current_gremlin
            {
                // update the texture according to the task
                match task_board {
                    GremlinTask::Play(animation_name)
                    | GremlinTask::PlayInterrupt(animation_name) => {
                        if let Some(animator) = &mut gremlin.animator
                            && animation_name == current_animation_name
                        {
                            animator.current_frame = 0;
                        } else if let Some(animation_props) =
                            gremlin.animation_map.get(animation_name.as_str())
                        {
                            let cache_lookup = {
                                lookup_cache(
                                    animation_name.as_str(),
                                    &texture_cache.lock().unwrap(),
                                )
                                .map(|a| a.0)
                            };
                            if let Some(index) = cache_lookup {
                                texture_cache.lock().unwrap().rearrange(index);
                                // unwrap safety: the mutex is guaranteed to not be poisoned and released after the rearrange cache function goes out of scope
                                let lock = &texture_cache.lock().unwrap();
                                // unwrap safety: the back element is guaranteed to exist because the index before rearranging exists.
                                let (animator, texture) = &lock.data.back().unwrap().1;
                                let _ = gremlin.animator.insert(animator.clone());
                                let _ = gremlin_texture.insert(texture.clone());
                                let _ = cache_hit_index.insert(index);
                            } else if let Ok(mut animation) =
                                <&AnimationProperties as TryInto<Animation>>::try_into(
                                    animation_props,
                                )
                            {
                                if animation.properties.sprite_count > 110 {
                                    animation.sprite_sheet.image = resize_image_to_window(
                                        animation.sprite_sheet.image,
                                        self.canvas.window(),
                                        animation_props.clone(),
                                    );
                                }

                                let texture_rc = Rc::new(
                                    animation
                                        .sprite_sheet
                                        .into_texture(&self.texture_creator)
                                        .unwrap(),
                                );
                                gremlin_texture = Some(texture_rc.clone());
                                let animator = Some((&animation).into());
                                gremlin.animator = animator;
                                if let Some(ref animator) = gremlin.animator {
                                    texture_cache.lock().unwrap().cache(
                                        animator.animation_properties.animation_name.clone(),
                                        (animator.clone(), texture_rc),
                                    );
                                }
                            }

                            should_check_for_action = false;
                            current_animation_name = animation_name;
                        }
                    }
                    GremlinTask::Goto(x, y) => {
                        move_target = Some(Point::new(x, y));
                    }
                    _ => {}
                }
            }

            // draws the next frame and update frame counter
            if let Some(gremlin) = &mut self.current_gremlin
                && let Some(gremlin_texture) = &gremlin_texture
                && let Some(animator) = &mut gremlin.animator
            {
                self.canvas.clear();
                self.canvas
                    .copy(&gremlin_texture, animator.get_frame_rect(), None)
                    .unwrap();
                self.canvas.present();
                if animator.current_frame + 1 == animator.animation_properties.sprite_count {
                    should_check_for_action = true;
                    if "OUTRO" == &current_animation_name {
                        println!("goodbye!");
                        break;
                    }
                }

                animator.current_frame =
                    (animator.current_frame + 1) % animator.animation_properties.sprite_count;
                thread::sleep(Duration::from_secs_f32(1.0 / (GLOBAL_FRAMERATE as f32)));
            }
            if move_towards_cursor {
                let (cursor_x, cursor_y) = get_cursor_position();
                let _ = move_target.insert(Point::new(cursor_x as i32, cursor_y as i32));
            } else {
                move_target.take();
            }
        }
    }

    fn init_ui() -> Component {
        div()
    }

    fn handle_sdl_events(&mut self, event_pump: &mut sdl3::EventPump) {}

    fn load_gremlin(&mut self, gremlin_txt_path: String) -> Result<Gremlin, GremlinLoadError> {
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

    pub fn update(&mut self) {
        let mut event_set = HashSet::new();
        let task_tx_2 = self.task_channel.0.clone();
        while let Some(event) = self.sdl.event_pump().unwrap().poll_event() {
            match event {
                SdlEvent::Quit { .. } => {
                    let _ = task_tx_2.send(GremlinTask::PlayInterrupt("OUTRO".to_string()));
                }
                SdlEvent::MouseButtonDown { mouse_btn, .. } => {
                    match mouse_btn {
                        sdl3::mouse::MouseButton::Left => {
                            // is_lmb_down = true;
                            // implemented in behavior
                        }
                        _ => (),
                    }
                }
                SdlEvent::MouseMotion { x, y, .. } => {
                    // implemented
                    // if is_lmb_down && !is_dragging {
                    //     is_dragging = true;
                    //     let _ = task_tx_2.send(GremlinTask::PlayInterrupt("GRAB".to_string()));
                    //     let _ = &self.task_queue.clear();
                    //     (drag_start_x, drag_start_y) = (x, y);
                    // }
                    // if is_dragging && should_check_drag {
                    //     let (gremlin_x, gremlin_y) = get_window_pos(&self.canvas);
                    //     self.canvas
                    //         .window_mut()
                    //         .set_position(
                    //             sdl3::video::WindowPos::Positioned(
                    //                 gremlin_x.saturating_add((x - drag_start_x) as i32)
                    //             ),
                    //             sdl3::video::WindowPos::Positioned(
                    //                 gremlin_y.saturating_add((y - drag_start_y) as i32)
                    //             )
                    //         );
                    // }
                    // // only move every odd frame because moving the window will trigger another mousemove SdlEvent
                    // should_check_drag = !should_check_drag;
                }

                SdlEvent::MouseButtonUp { mouse_btn, .. } => {
                    match mouse_btn {
                        sdl3::mouse::MouseButton::Left => {
                            // if !is_dragging && is_lmb_down {
                            //     let _ = task_tx_2.send(
                            //         GremlinTask::PlayInterrupt("CLICK".to_string())
                            //     );
                            //     move_towards_cursor = !move_towards_cursor;
                            //     last_moved_at = Instant::now();
                            // }
                            // if is_dragging && is_lmb_down {
                            //     let _ = task_tx_2.send(
                            //         GremlinTask::PlayInterrupt("PAT".to_string())
                            //     );
                            // }
                            // let _ = task_tx_2.send(GremlinTask::Play("IDLE".to_string()));
                            // is_dragging = false;
                            // is_lmb_down = false;
                        }
                        _ => (),
                    }
                }
                SdlEvent::Window { win_event, .. } => {
                    match win_event {
                        sdl3::event::WindowEvent::Moved(x, y) => {
                            // (gremlin_x, gremlin_y) = (x, y);
                        }
                        _ => {}
                    }
                }

                _ => {}
            }
            event_set.insert(Event::from(event));
        }
        // thread::sleep(Duration::from_millis(500));

        // handle gremlin movement
        // let gremlin_position = Point::new(
        //     gremlin_x + ((self.canvas.window().size().0 / 2) as i32),
        //     gremlin_y + ((self.canvas.window().size().1 / 2) as i32)
        // );

        // will move to behavior
        // if !is_dragging && move_towards_cursor && let Some(move_target) = move_target {
        //     let (dir_x, dir_y) = get_move_direction(move_target, {
        //         let mut win_rect = win_to_rect(self.canvas.window());
        //         if win_rect.contains_point(move_target) {
        //             win_rect.resize(win_rect.width() + 100, win_rect.height() + 100);
        //             println!("{:?}", win_rect);
        //         }
        //         win_rect
        //     });
        //     let tan =
        //         ((gremlin_position.y - move_target.y) as f32) /
        //         ((gremlin_position.x - move_target.x) as f32);
        //     let alpha = tan.atan();

        //     let (velo_x, x_anim) = match dir_x {
        //         DirectionX::None => (0.0, ""),
        //         DirectionX::Left => (-velocity, "LEFT"),
        //         DirectionX::Right => (velocity, "RIGHT"),
        //     };
        //     let (velo_y, y_anim) = match dir_y {
        //         DirectionY::None => (0.0, ""),
        //         DirectionY::Up => (-velocity, "UP"),
        //         DirectionY::Down => (velocity, "DOWN"),
        //     };

        //     let animation_name = match (dir_x, dir_y) {
        //         (DirectionX::None, DirectionY::None) => { "RUNIDLE".to_string() }
        //         (DirectionX::None, _) => { "RUN".to_string() + y_anim }
        //         (_, DirectionY::None) => { "RUN".to_string() + x_anim }
        //         (_, _) => { y_anim.to_string() + x_anim }
        //     };
        //     if current_animation_name != animation_name {
        //         let _ = task_tx_2.send(GremlinTask::PlayInterrupt(animation_name));
        //         &self.task_queue.clear();
        //     }

        //     let (velo_x, velo_y) = (velo_x * alpha.cos().abs(), velo_y * alpha.sin().abs());

        //     self.canvas
        //         .window_mut()
        //         .set_position(
        //             sdl3::video::WindowPos::Positioned(
        //                 ((gremlin_x as f32) + velo_x * last_moved_at.elapsed().as_secs_f32()) as i32
        //             ),
        //             sdl3::video::WindowPos::Positioned(
        //                 ((gremlin_y as f32) + velo_y * last_moved_at.elapsed().as_secs_f32()) as i32
        //             )
        //         );
        //     last_moved_at = Instant::now();
        // }

        let mut task_board = None;

        // check for tasks and append to task queue
        while let Ok(task) = self.task_channel.1.try_recv() {
            if let GremlinTask::PlayInterrupt(_) = &task {
                task_board = Some(task);
                break;
            }
            &self.task_queue.push_back(task);
        }

        if let None = task_board {
            task_board = self.task_queue.pop_front();
        }

        let mut cache_hit_index: Option<usize> = None;

        if let Some(task_board) = task_board
            && let Some(gremlin) = &mut self.current_gremlin
        {
            let texture_cache = &mut gremlin.texture_cache;
            // update the texture according to the task
            match task_board {
                GremlinTask::Play(animation_name) | GremlinTask::PlayInterrupt(animation_name) => {
                    // if
                    //     let Some(animator) = &mut gremlin.animator
                    //     // animation_name ==
                    // {
                    //     animator.current_frame = 0;
                    // }else
                    if let Some(animation_props) =
                        gremlin.animation_map.get(animation_name.as_str())
                    {
                        let cache_lookup =
                            { lookup_cache(animation_name.as_str(), &texture_cache).map(|a| a.0) };
                        if let Some(index) = cache_lookup {
                            texture_cache.rearrange(index);
                            let lock = texture_cache;
                            let (animator, texture) = &lock.data.back().unwrap().1;
                            let _ = gremlin.animator.insert(animator.clone());
                            let _ = gremlin.texture.insert(texture.clone());
                            let _ = cache_hit_index.insert(index);
                        } else if let Ok(mut animation) =
                            <&AnimationProperties as TryInto<Animation>>::try_into(animation_props)
                        {
                            if animation.properties.sprite_count > 110 {
                                animation.sprite_sheet.image = resize_image_to_window(
                                    animation.sprite_sheet.image,
                                    self.canvas.window(),
                                    animation_props.clone(),
                                );
                            }

                            let texture_rc: Rc<Texture> = Rc::new(
                                animation
                                    .sprite_sheet
                                    .into_texture(&self.texture_creator)
                                    .unwrap(),
                            );
                            gremlin.texture = Some(texture_rc.clone());
                            let animator = Some((&animation).into());
                            gremlin.animator = animator;
                            if let Some(ref animator) = gremlin.animator {
                                texture_cache.cache(
                                    animator.animation_properties.animation_name.clone(),
                                    (animator.clone(), texture_rc),
                                );
                            }
                        }

                        self.should_check_for_action = false;
                    }
                }
                GremlinTask::Goto(x, y) => {
                    // move_target = Some(Point::new(x, y));
                }
                _ => {}
            }
        }

        // draws the next frame and update frame counter
        if let Some(gremlin) = &mut self.current_gremlin
            && let Some(gremlin_texture) = &gremlin.texture
            && let Some(animator) = &mut gremlin.animator
        {
            self.canvas.clear();
            self.canvas
                .copy(&gremlin_texture, animator.get_frame_rect(), None)
                .unwrap();
            self.canvas.present();
            if animator.current_frame + 1 == animator.animation_properties.sprite_count {
                self.should_check_for_action = true;
                if "OUTRO" == &animator.animation_properties.animation_name {
                    println!("goodbye!");
                }
            }

            animator.current_frame =
                (animator.current_frame + 1) % animator.animation_properties.sprite_count;
            thread::sleep(Duration::from_secs_f32(1.0 / (GLOBAL_FRAMERATE as f32)));
        }
        // if move_towards_cursor {
        //     let (cursor_x, cursor_y) = get_cursor_position(&event_pump);
        //     let _ = move_target.insert(Point::new(cursor_x as i32, cursor_y as i32));
        // }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GremlinTask {
    Play(String),
    PlayInterrupt(String),
    Goto(i32, i32),
}

// impl Into<Rect> for FRect {
pub fn into_rect(f_rect: FRect) -> Rect {
    Rect::new(
        f_rect.x as i32,
        f_rect.y as i32,
        f_rect.w as u32,
        f_rect.h as u32,
    )
}
pub fn into_opt_rect(f_rect: Option<FRect>) -> Option<Rect> {
    if let Some(f_rect) = f_rect {
        return Some(Rect::new(
            f_rect.x as i32,
            f_rect.y as i32,
            f_rect.w as u32,
            f_rect.h as u32,
        ));
    }
    None
}

pub fn get_window_pos(canvas: &Canvas<Window>) -> (i32, i32) {
    canvas.window().position()
}

pub fn into_frect(rect: Rect) -> FRect {
    FRect {
        x: rect.x as f32,
        y: rect.y as f32,
        w: rect.w as f32,
        h: rect.h as f32,
    }
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
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
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
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> Result<()> {
        let color = canvas.draw_color();
        canvas.set_draw_color(self.color);
        canvas.fill_rect(rect).unwrap();
        canvas.set_draw_color(color);

        Ok(())
    }
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
    sprite_sheet: SpriteSheet,
    pub current_frame: u16,
    properties: AnimationProperties,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum AnimationKind {
    Walk(DirectionX, DirectionY),
    Intro,
    Idle,
    Exit,
    Hover,
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

#[derive(Default)]
struct TextureCache {
    data: VecDeque<(String, TextureCacheItem)>,
}
// /
type TextureCacheItem = (Animator, Rc<Texture>);

fn lookup_cache<'a>(
    animation_name: &str,
    cache: &'a TextureCache,
) -> Option<(usize, TextureCacheItem)> {
    cache
        .data
        .iter()
        .enumerate()
        .rev()
        .find(|a| a.1.0 == animation_name)
        .map(|a| (a.0, a.1.1.clone()))
}

impl TextureCache {
    // rearrange to purge cache later with a LRU policy
    fn rearrange(&mut self, access_index: usize) {
        if let Some(item) = self.data.remove(access_index) {
            self.data.push_back(item);
        }
    }

    fn print(&self) {
        let mut res = String::new();
        for (name, _) in &self.data {
            res += &(name.to_owned() + " ");
        }
        println!("{}", res)
    }
    fn cache(&mut self, name: String, texture: TextureCacheItem) {
        match &self.data.len() {
            CACHE_CAPACITY.. => {
                self.data.pop_front();
            }
            _ => (),
        }
        self.data.push_back((name, texture));
    }
}

const CACHE_CAPACITY: usize = 10;
