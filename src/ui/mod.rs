use std::collections::HashSet;

use bad_signals::signals::signals::Signal;
use sdl3::{
    pixels::Color,
    rect::{Point, Rect},
    render::{Canvas, FRect, Texture},
    video::Window,
};
pub mod widgets;

use crate::{
    gremlin::{SizeUnit, into_frect, into_opt_rect, into_rect},
    utils::calculate_pix_from_parent,
};

pub struct Component {
    rendered_by: Box<dyn Composable>,
    location: Rect,
    event_listeners: HashSet<Signal<ComponentEvent>>,
    children: Vec<Component>,
    preferred_size: (SizeUnit, SizeUnit),
}

impl Component {
    pub fn new(renderable: Box<dyn Composable>) -> Self {
        Component {
            rendered_by: renderable,
            location: Rect::new(0, 0, 0, 0),
            event_listeners: Default::default(),
            children: Default::default(),
            preferred_size: (SizeUnit::Percentage(100), SizeUnit::Percentage(100)),
        }
    }

    pub fn add_child(mut self, component: Component) -> Self {
        self.children.push(component);
        self
    }

    pub fn add_children(mut self, mut components: Vec<Component>) -> Self {
        self.children.append(&mut components);
        self
    }

    pub fn set_preferred_size(mut self, size: (SizeUnit, SizeUnit)) -> Self {
        self.preferred_size = size;
        self
    }
}

pub trait Composable: Render + Notify {}

pub trait Notify {
    fn notify(&self, event: ComponentEvent);
}

#[derive(Default, Debug, Clone)]
pub struct Div {
    pub styles: Option<Vec<RenderStyle>>,
    pub text: String,
}

impl Div {
    pub fn style(mut self, style: RenderStyle) -> Self {
        if let Some(ref mut styles) = self.styles {
            styles.push(style);
        } else {
            self.styles = Some(vec![style]);
        }

        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RenderStyle {
    BackgroundColor(Color),
    Position(Position),
}

pub fn compose<T: Composable + 'static>(from: T) -> Component {
    // move composable component into this scope
    let composition = from;
    Component::new(Box::new(composition))
}

#[derive(Debug, Clone, Copy)]
pub enum Position {
    // u32 offsets
    Relative(SizeUnit, SizeUnit),
    Fixed(SizeUnit, SizeUnit),
}

pub fn p_fixed(ml: u32, mr: u32, unit: SizeUnit) -> Position {
    let sz = match unit {
        SizeUnit::Pixel(_) => (SizeUnit::Pixel(ml), SizeUnit::Pixel(mr)),
        SizeUnit::Percentage(_) => (SizeUnit::Percentage(ml), SizeUnit::Percentage(mr)),
    };
    Position::Fixed(sz.0, sz.1)
}

impl Default for Position {
    fn default() -> Self {
        Position::Relative(SizeUnit::Pixel(0), SizeUnit::Pixel(0))
    }
}

impl Render for Div {
    fn render(
        &self,
        texture: &mut sdl3::render::Texture,
        rect: Option<sdl3::render::FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()
        //no auto layouts for now ...

        // rgba
        // static DEFAULT_COLOR: LazyLock<Color> = LazyLock::new(|| Color::BLACK);
        let mut background_color = Color::BLACK;
        const FRAGMENT_SHADER: fn(&mut (u8, u8, u8, u8), Color) -> () = |components, color| {
            components.0 = color.r;
            components.1 = color.g;
            components.2 = color.b;
            components.3 = color.a;
        };

        let window_rect = FRect::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);

        let mut rendering_rect = if let Some(rect) = rect {
            rect
        } else {
            // FRect::new(0.0, 0.0, texture.width() as f32, texture.height() as f32)
            window_rect
        };

        if let Some(styles) = &self.styles {
            for style in styles {
                match style {
                    RenderStyle::BackgroundColor(color) => {
                        background_color = *color;
                        println!("{:?}", color);
                    }
                    RenderStyle::Position(position) => match position {
                        Position::Relative(size_unit, size_unit1) => {
                            rendering_rect.x += calculate_pix_from_parent(
                                (texture.width(), texture.height()),
                                (*size_unit, SizeUnit::Pixel(0)),
                            )
                            .0 as f32;
                            rendering_rect.y += calculate_pix_from_parent(
                                (texture.width(), texture.height()),
                                (SizeUnit::Pixel(0), *size_unit1),
                            )
                            .1 as f32;
                        }
                        Position::Fixed(size_unit, size_unit1) => {
                            rendering_rect.x = calculate_pix_from_parent(
                                (texture.width(), texture.height()),
                                (*size_unit, SizeUnit::Pixel(0)),
                            )
                            .0 as f32;
                            rendering_rect.y = calculate_pix_from_parent(
                                (texture.width(), texture.height()),
                                (SizeUnit::Pixel(0), *size_unit1),
                            )
                            .1 as f32;
                        }
                    },
                    _ => {}
                }
            }
        }

        texture.with_lock(into_rect(rendering_rect), move |buffer, _stride| {
            let mut i = 0;
            while i + 3 < buffer.len() {
                let mut color_components = (buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]);
                FRAGMENT_SHADER(&mut color_components, background_color);
                (buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]) = color_components;
                i += 3;
            }
        })?;
        // }
        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
        rect: Option<sdl3::render::FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        // todo!()
        let draw_color = canvas.draw_color();
        let mut target_draw_color = Color::BLACK;
        if let Some(styles) = &self.styles {
            for style in styles {
                match style {
                    RenderStyle::BackgroundColor(color) => {
                        target_draw_color = *color;
                    }
                    _ => {}
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
    pub root: Component,
}

pub fn div() -> Component {
    let div = Div::new();
    Component::new(div)
}

// should this be rendering backend agnostic?
impl Composable for Div {}
impl Div {
    pub fn new() -> Box<Self> {
        Box::new(Default::default())
    }
}
impl Notify for Div {
    fn notify(&self, _: ComponentEvent) {}
}
// pub type Renderer = impl FnMut(&mut [u8], u8) -> anyhow::Result<()>;
impl Default for UI {
    fn default() -> Self {
        let component = Component::new(Div::new());

        Self { root: component }
    }
}

fn render_tree(
    component: &Component,
    texture: &mut Texture,
    parent_rect: Rect,
) -> anyhow::Result<()> {
    let render_rect_size = calculate_pix_from_parent(
        (parent_rect.w as u32, parent_rect.h as u32),
        (component.preferred_size.0, component.preferred_size.1),
    );

    println!("{:?}", render_rect_size);
    let render_rect = {
        Rect::new(
            /*offsets in the future maybe*/ 0,
            0,
            render_rect_size.0,
            render_rect_size.1,
        )
    };
    component
        .rendered_by
        .as_ref()
        .render(texture, Some(into_frect(render_rect)))?;
    for child in &component.children {
        render_tree(child, texture, render_rect)?;
    }
    Ok(())
}

fn render_tree_canvas(
    component: &Component,
    canvas: &mut Canvas<Window>,
    parent_rect: Rect,
) -> anyhow::Result<()> {
    let render_rect_size = calculate_pix_from_parent(
        (parent_rect.w as u32, parent_rect.h as u32),
        (component.preferred_size.0, component.preferred_size.1),
    );

    println!("{:?}", render_rect_size);
    let render_rect = { Rect::new(0, 0, render_rect_size.0, render_rect_size.1) };
    component
        .rendered_by
        .as_ref()
        .render_canvas(canvas, Some(into_frect(render_rect)))?;

    for child in &component.children {
        render_tree_canvas(child, canvas, render_rect)?;
    }

    Ok(())
}

impl Render for UI {
    fn render(
        &self,
        texture: &mut Texture,
        parent_rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        render_tree(
            &self.root,
            texture,
            into_rect(parent_rect.unwrap_or(FRect::new(
                0.0,
                0.0,
                texture.width() as f32,
                texture.height() as f32,
            ))),
        )?;

        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        render_tree_canvas(
            &self.root,
            canvas,
            into_rect(rect.unwrap_or(FRect::new(
                0.0,
                0.0,
                canvas.window().size().0 as f32,
                canvas.window().size().1 as f32,
            ))),
        )?;
        Ok(())
    }
}

struct Button {
    div: Div,
}

impl Render for Button {
    fn render(
        &self,
        texture: &mut Texture,
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        self.div.render(texture, rect)?;
        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>s
    ) -> anyhow::Result<()> {
        self.div.render_canvas(canvas, rect)?;
        Ok(())
    }
}

impl Notify for Button {
    fn notify(&self, event: ComponentEvent) {
        match event {
            ComponentEvent::OnMouseDown {
                global_pointer_location,
            } => {
                println!("{:?}", global_pointer_location);
            }
            _ => {}
        }
        self.div.notify(event);
    }
}

impl Composable for Button {}

// impl UI {
//     // pub fn render
// }

#[derive(Debug, Clone, Copy, Hash)]
pub enum ComponentEvent {
    OnMouseDown { global_pointer_location: Point },
    OnMouseHover { pointer_location: Point },
    OnMouseUp { pointer_location: Point },
}

pub trait Render {
    /// use of render() is quite expensive as most operations are done software-wise
    /// texture needs to be an sdl streaming texture and in rgba24 format
    fn render(
        &self,
        texture: &mut Texture,
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()>;

    /// render_canvas() utilizes SDL's Render API, abstracting away platform specific
    /// GPU backend. this is more generally recommended
    fn render_canvas(
        &self,
        canvas: &mut Canvas<Window>,
        rect: Option<FRect>, // styles: Option<Vec<RenderStyle>>s
    ) -> anyhow::Result<()>;
}
