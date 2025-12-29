use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use sdl3::render::Texture;

use crate::{
    behavior::Behavior,
    gremlin::{Animation, AnimationProperties, GremlinTask},
    utils::{TextureCache, resize_image_to_window},
};

#[derive(Default)]
pub struct GremlinRender {
    pub current_animation_name: String,
    pub texture_cache: Arc<Mutex<TextureCache>>,
    pub gremlin_texture: Option<Rc<Texture>>,
}

impl GremlinRender {
    pub fn new() -> Box<Self> {
        Default::default()
    }
}

impl Behavior for GremlinRender {
    fn setup(&mut self, _: &mut crate::gremlin::DesktopGremlin) {}

    fn update(&mut self, application: &mut crate::gremlin::DesktopGremlin, _: &super::ContextData) {
        let mut task_board = None;

        // check for tasks and append to task queue
        while let Ok(task) = application.task_channel.1.try_recv() {
            if let GremlinTask::PlayInterrupt(_) = &task {
                task_board = Some(task);
                break;
            }
            let _ = &application.task_queue.push_back(task);
        }

        if let None = task_board
            && application.should_check_for_action
        {
            task_board = application.task_queue.pop_front();
        }

        let mut cache_hit_index: Option<usize> = None;
        if let Some(task_board) = task_board
            && let Some(gremlin) = &mut application.current_gremlin
        {
            // update the texture according to the task
            match task_board {
                GremlinTask::Play(animation_name) | GremlinTask::PlayInterrupt(animation_name) => {
                    if let Some(animator) = &mut gremlin.animator
                        && animation_name == self.current_animation_name
                    {
                        animator.current_frame = 0;
                    } else if let Some(animation_props) =
                        gremlin.animation_map.get(animation_name.as_str())
                    {
                        let cache_lookup = {
                            self.texture_cache
                                .lock()
                                .unwrap()
                                .lookup(animation_name.clone())
                                .map(|a| a.0)
                        };

                        if let Some(index) = cache_lookup {
                            self.texture_cache.lock().unwrap().rearrange(index);
                            // unwrap safety: the mutex is guaranteed to not be poisoned and released after the rearrange cache function goes out of scope
                            let lock = &self.texture_cache.lock().unwrap();
                            // unwrap safety: the back element is guaranteed to exist because the index before rearranging exists.
                            let (animator, texture) = &lock.data.back().unwrap().1;
                            let _ = gremlin.animator.insert(animator.clone());
                            let _ = self.gremlin_texture.insert(texture.clone());
                            let _ = cache_hit_index.insert(index);
                        } else if let Ok(mut animation) =
                            <&AnimationProperties as TryInto<Animation>>::try_into(animation_props)
                        {
                            if animation.properties.sprite_count > 110 {
                                animation.sprite_sheet.image = resize_image_to_window(
                                    animation.sprite_sheet.image,
                                    application.canvas.window(),
                                    animation_props.clone(),
                                );
                            }

                            let texture_rc = Rc::new(
                                animation
                                    .sprite_sheet
                                    .into_texture(&application.texture_creator)
                                    .unwrap(),
                            );

                            self.gremlin_texture.insert(texture_rc.clone());

                            let animator = Some((&animation).into());
                            drop(animation);

                            gremlin.animator = animator;
                            if let Some(ref animator) = gremlin.animator {
                                self.texture_cache.lock().unwrap().cache(
                                    animator.animation_properties.animation_name.clone(),
                                    (animator.clone(), texture_rc),
                                );
                            }
                        }

                        application.should_check_for_action = false;
                        self.current_animation_name = animation_name;
                    }
                }
                _ => {}
            }
        }

        // draws the next frame and update frame counter
        if let Some(gremlin) = &mut application.current_gremlin
            && let Some(gremlin_texture) = &self.gremlin_texture
            && let Some(animator) = &mut gremlin.animator
        {
            application.canvas.clear();
            application
                .canvas
                .copy(&gremlin_texture, animator.get_frame_rect(), None)
                .unwrap();
            application.canvas.present();
            if animator.current_frame + 1 == animator.animation_properties.sprite_count {
                application.should_check_for_action = true;
                if "OUTRO" == &self.current_animation_name {
                    println!("goodbye!");
                    *application.should_exit.lock().unwrap() = true;
                }
            }

            animator.current_frame =
                (animator.current_frame + 1) % animator.animation_properties.sprite_count;
        }
    }
}
