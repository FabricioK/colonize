#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate sdl2;
extern crate time;

// The target FPS to render at.
const FRAMES_PER_SECOND: u64 = 60;

// Window parameters.
const WINDOW_TITLE: &str = "colonize";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

/// A timer used to count elapsed ticks (i.e. milliseconds).
struct Timer {
    start: u64,
}

impl Timer {
    /// Creates and starts a new timer.
    fn new() -> Self {
        Timer {
            start: time::precise_time_ns(),
        }
    }

    /// Resets the timer.
    fn reset(&mut self) {
        self.start = time::precise_time_ns();
    }

    /// Returns the number of ticks elapsed since the timer was started.
    fn get_ticks(&self) -> u64 {
        (time::precise_time_ns() - self.start) / 1_000_000
    }
}

fn main() {
    use sdl2::image::LoadTexture;
    use sdl2::pixels::Color;
    use std::path::Path;

    let (sdl_context, mut canvas) = init();

    // Set the draw color for the canvas.
    canvas.set_draw_color(Color::RGB(255, 0, 0));

    // Create and load a texture.
    let texture_path = Path::new("./assets/textures/game_scene/block.png");
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator.load_texture(texture_path)
        .unwrap_or_else(|err| panic!("Failed to load texture: {}", err));

    // Obtain the SDL2 event pump.
    let mut event_pump = sdl_context.event_pump()
        .unwrap_or_else(|err| panic!("Failed to obtain SDL2 event pump: {}", err));

    // Create a timer which will be used to time the interval between frames.
    let mut fps_timer = Timer::new();

    'running: loop {
        // Start the frame timer.
        fps_timer.reset();

        // Events
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;
            use sdl2::keyboard::Keycode;
            println!("SDL2 event: {:?}", event);
            match event {
                Event::Quit { .. } |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } |
                    Event::KeyDown { keycode: Some(Keycode::Q), .. } => break 'running,
                _ => {},
            }
        }

        // Logic

        // Rendering
        render(&mut canvas, &texture);

        // Delay the rendering of the next frame to match the required tick
        // rate.
        while fps_timer.get_ticks() < 1000 / FRAMES_PER_SECOND {}
        println!("Elapsed ticks: {}", fps_timer.get_ticks());
    }
}

fn init() -> (sdl2::Sdl, sdl2::render::Canvas<sdl2::video::Window>) {
    // Initialize the SDL2 library.
    let sdl_context = sdl2::init().unwrap_or_else(
        |err| panic!("Failed to initialize SDL2 context: {}", err));
    // Initialize the SDL2 video subsystem.
    let video_subsystem = sdl_context.video().unwrap_or_else(
        |err| panic!("Failed to initialize SDL2 video subsystem: {}", err));

    // Create an SDL2 window.
    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap_or_else(|err| panic!("Failed to create SDL2 window: {}", err));

    // Initialize a window-based renderer.
    let canvas = window.into_canvas().build()
        .unwrap_or_else(|err| panic!("Failed to initialize renderer: {}", err));

    (sdl_context, canvas)
}

fn render(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
          texture: &sdl2::render::Texture) {
    // Clear the current rendering target with the drawing color.
    canvas.clear();
    // Copy the texture to the screen.
    canvas.copy(texture, None, None)
        .unwrap_or_else(|err| panic!("Failed to render texture: {}", err));
    // Display the composed backbuffer to the screen.
    canvas.present();
}
