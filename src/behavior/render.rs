use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use sdl3::render::Texture;

use crate::{
    behavior::Behavior,
    gremlin::{Animation, AnimationProperties, Animator, DEFAULT_COLUMN_COUNT, GremlinTask},
    utils::{TextureCache, sdl_resize},
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
                            let lock: &std::sync::MutexGuard<'_, TextureCache> =
                                &self.texture_cache.lock().unwrap();
                            // unwrap safety: the back element is guaranteed to exist because the index before rearranging exists.
                            let (animator, texture) = &lock.data.back().unwrap().1;
                            let _ = gremlin.animator.insert(animator.clone());
                            let _ = self.gremlin_texture.insert(texture.clone());
                            let _ = cache_hit_index.insert(index);
                        } else if let Ok(animation) =
                            <&AnimationProperties as TryInto<Animation>>::try_into(animation_props)
                        {
                            let mut animator: Animator = (&animation).into();

                            let texture_rc = Rc::new({
                                let scale_factor = (1, 1);
                                let (sprite_width, sprite_height) =
                                    application.canvas.window().size();
                                let (target_width, target_height) = (
                                    (DEFAULT_COLUMN_COUNT * sprite_width * scale_factor.0)
                                        / scale_factor.1,
                                    (animation
                                        .properties
                                        .sprite_count
                                        .div_ceil(DEFAULT_COLUMN_COUNT)
                                        * sprite_height
                                        * scale_factor.0)
                                        / scale_factor.1,
                                );
                                animator.sprite_size = (sprite_width, sprite_height);
                                animator.texture_size = (target_width, target_height);

                                sdl_resize(
                                    &animation.sprite_sheet.image,
                                    animator.texture_size,
                                    &mut application.canvas,
                                )
                                .unwrap()
                            });

                            let _ = self.gremlin_texture.insert(texture_rc.clone());
                            drop(animation);

                            gremlin.animator = Some(animator);

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
