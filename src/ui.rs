use std::{ collections::HashSet, sync::LazyLock };

use bad_signals::signals::signals::Signal;
use sdl3::{ pixels::Color, rect::Point, render::{ Canvas, FRect, Texture }, video::Window };

use crate::sprite::{ GLOBAL_PIXEL_FORMAT, into_opt_rect, into_rect };

pub struct Component {
    rendered_by: Box<dyn Composable>,
    event_listeners: HashSet<Signal<ComponentEvent>>,
    children: Vec<Component>,
}

impl Component {
    pub fn new(renderable: Box<dyn Composable>) -> Self {
        Component {
            rendered_by: renderable,
            event_listeners: Default::default(),
            children: Default::default(),
        }
    }

    pub fn add_child(&mut self, component: Component) {
        self.children.push(component);
    }
}

pub trait Composable: Render + Notify {}

pub trait Notify {
    fn notify(&self, event: ComponentEvent);
}
pub struct Div;

pub enum RenderStyle {
    BackgroundColor(Color),
}

impl Render for Div {
    fn render(
        &self,
        texture: &mut sdl3::render::Texture,
        rect: Option<sdl3::render::FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()
        //no auto layouts for now ...
        // if let Some(rect) = rect {

        // rgba
        // static DEFAULT_COLOR: LazyLock<Color> = LazyLock::new(|| Color::BLACK);
        let mut background_color = Color::BLACK;
        const FRAGMENT_SHADER: fn(&mut (u8, u8, u8, u8), Color) -> () = |components, color| {
            components.0 = color.r;
            components.1 = color.g;
            components.2 = color.b;
            components.3 = color.a;
        };
        if let Some(styles) = styles {
            for style in styles {
                match style {
                    RenderStyle::BackgroundColor(color) => {
                        background_color = color;
                    }
                }
            }
        }
        texture.with_lock(into_opt_rect(rect), move |buffer, _stride| {
            let mut i = 0;
            while i + 3 < buffer.len() {
                let mut color_components = (buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]);
                FRAGMENT_SHADER(&mut color_components, background_color);
                i += 3;
            }
        })?;
        // }
        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
        rect: Option<sdl3::render::FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()
        let draw_color = canvas.draw_color();
        let mut target_draw_color = Color::BLACK;
        if let Some(styles) = styles {
            for style in styles {
                match style {
                    RenderStyle::BackgroundColor(color) => {
                        target_draw_color = color;
                    }
                }
            }
        }
        canvas.set_draw_color(target_draw_color);
        canvas.fill_rect(rect)?;
        canvas.set_draw_color(draw_color);

        Ok(())
    }
}

pub struct UI {
    root: Option<Component>,
}
// should this be rendering backend agnostic?

// pub type Renderer = impl FnMut(&mut [u8], u8) -> anyhow::Result<()>;

impl Render for UI {
    fn render(
        &self,
        texture: &mut Texture,
        rect: Option<FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()

        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()
        let size = canvas.window().size();
        self.render(
            &mut canvas
                .texture_creator()
                .create_texture_streaming(GLOBAL_PIXEL_FORMAT, size.0, size.1)?,
            rect,
            styles
        )?;
        Ok(())
    }
}

// impl UI {
//     // pub fn render
// }

#[derive(Debug, Clone, Copy, Hash)]
pub enum ComponentEvent {
    OnMouseDown {
        global_pointer_location: Point,
    },
    OnMouseHover {
        pointer_location: Point,
    },
    OnMouseUp {
        pointer_location: Point,
    },
}

pub trait Render {
    /// use of render() is quite expensive as most operations are done software-wise
    /// texture needs to be an sdl streaming texture and in rgba24 format
    fn render(
        &self,
        texture: &mut Texture,
        rect: Option<FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()>;

    /// render_canvas() utilizes SDL's Render API, abstracting away platform specific
    /// GPU backend. this is more generally recommended
    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>,
        styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()>;
    // where T: Into<Option<FRect>>;
}
