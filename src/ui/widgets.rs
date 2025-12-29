use image::DynamicImage;

use crate::{
    gremlin::{GLOBAL_PIXEL_FORMAT, into_opt_rect},
    ui::{Composable, Notify, Render},
    utils::img_get_bytes_global,
};

pub struct Image {
    data: DynamicImage,
}

impl Image {
    pub fn new(file_dir: &str) -> anyhow::Result<Self> {
        Ok(Image {
            data: image::open(file_dir)?,
        })
    }
}

impl Render for Image {
    /// size of Image and rendering texture should be the same, otherwise the function would do panic
    fn render(
        &self,
        texture: &mut sdl3::render::Texture,
        rect: Option<sdl3::render::FRect>, // styles: Option<Vec<RenderStyle>>
    ) -> anyhow::Result<()> {
        texture.with_lock(into_opt_rect(rect), |buffer, _| {
            buffer.swap_with_slice(img_get_bytes_global(&self.data).unwrap().as_mut_slice())
        })?;

        Ok(())
    }

    fn render_canvas(
        &self,
        canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
        rect: Option<sdl3::render::FRect>, // styles: Option<Vec<RenderStyle>>s
    ) -> anyhow::Result<()> {
        let texture = canvas.texture_creator();

        let mut texture = texture.create_texture_static(
            GLOBAL_PIXEL_FORMAT,
            self.data.width(),
            self.data.height(),
        )?;

        let image_bytes = img_get_bytes_global(&self.data).unwrap();
        let image_bytes = image_bytes.as_slice();

        texture.update(
            None,
            image_bytes,
            (self.data.width() as usize) * GLOBAL_PIXEL_FORMAT.bytes_per_pixel(),
        )?;

        canvas.copy(&texture, None, rect)?;
        drop(texture);
        Ok(())
    }
}

impl Notify for Image {
    fn notify(&self, event: super::ComponentEvent) {}
}

impl Composable for Image {}

// kinda too lazy to implement this rn so maybe later
pub struct LazyImage {}
