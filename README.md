# Desktop Gremlin

>  ## **Major Disclaimer**
> 
>  This project is completely inspired by **[KurtVelasco](https://github.com/KurtVelasco)**'s [DesktopGremlin](https://github.com/KurtVelasco/Desktop_Gremlin) project, implemented in C#

### Drag, pet, have them follow you around! Desktop Gremlin is a desktop pet application that allows you to load your fun little gremlins and have them accompany you through the hells of computers. 
Written in Rust for lower footprint and uses SDL3 for cross-platform convenience

<img width="512" height="361" alt="Screenshot and preview" src="https://github.com/user-attachments/assets/dd4c6aa7-d9ee-4da7-8edc-cb38b3c00e74" />


## Roadmap
- [x] Load and play gremlin sprite sheets
- [x] Parse files from `Desktop_Gremlin`
- [ ] Restructure project and seperate into different modules (WIP)
- [x] Handle click events and cursor events (WIP)
- [ ] OSD for configuration, resizing and a drag handle to drag gremlins around
- [ ] Implement locating cursor position in macOS and Hyperland
- [ ] Finish a simple UI tree implementation and seperate into another crate
- [ ] Test & support Hyperland
- [ ] Migrate to `winit` and `wgpu`

## Special thanks
- `SDL3` (you are goated!) and `sdl3-rs` authors (thank you so so so much!)
- `image` crate authors (awesomesauce stuffs)
