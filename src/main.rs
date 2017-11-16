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
    // Initialize the SDL2 library.
    let sdl_context = sdl2::init().expect("Failed to initialize SDL2 context");
    // Initialize the SDL2 video subsystem.
    let video_subsystem = sdl_context.video()
        .expect("Failed to initialize SDL2 video subsystem");

    // Create an SDL2 window.
    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .expect("Failed to create SDL2 window");

    // Obtain the SDL2 event pump.
    let mut event_pump = sdl_context.event_pump()
        .expect("Failed to obtain SDL2 event pump");

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

        // Delay the rendering of the next frame to match the required tick
        // rate.
        while fps_timer.get_ticks() < 1000 / FRAMES_PER_SECOND {}
        println!("Elapsed ticks: {}", fps_timer.get_ticks());
    }
}
