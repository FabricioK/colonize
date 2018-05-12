#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate bincode;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate noise;
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;

use bincode::{deserialize, serialize};
use noise::{NoiseFn, Perlin, Seedable};
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use serde::{Serialize, Serializer};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Error as IoError, ErrorKind, Read, Write};
use std::path::Path;

// The target FPS to render at.
const FRAMES_PER_SECOND: u64 = 60;

// Window parameters.
const WINDOW_TITLE: &str = "colonize";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

const FONT_PATH: &str = "./assets/fonts/NotoSans/NotoSans-Regular.ttf";
const FONT_SIZE: u16 = 12;

const CAMERA_MOVE_SPEED: i32 = 1;

// Size of a bincode serialized representation of the `GameInput` struct, in
// bytes. This **MUST** be updated whenever the `GameInput` struct is changed.
const BINCODED_GAME_INPUT_SIZE: usize = 44;
// Filename that input recordings are saved to.
const RECORDING_FILENAME: &str = "recording.ci";
// Filename that the game state is saved to.
const STATE_FILENAME: &str = "state.sav";

// Size of a row of voxels, as measured in voxels per dimension of the chunk.
const VOXEL_ROW_SIZE: usize = 32;
// Number of rows of voxels located in a single chunk.
const VOXEL_ROW_COUNT: usize = VOXEL_ROW_SIZE * VOXEL_ROW_SIZE;
// Size of a box of voxels (i.e. the size of an array which holds
// VOXEL_ROW_SIZE**3 voxels).
const VOXEL_BOX_SIZE: usize = VOXEL_ROW_SIZE * VOXEL_ROW_SIZE * VOXEL_ROW_SIZE;

// Size of a voxel (in pixels) when rendered.
const VOXEL_RECT_SIZE: usize = 16;

// Mask for determining if a row is allocated.
const ROW_TAG_ALLOCATED: u16 = 0b1000_0000_0000_0000;
// Mask for retrieving just the tag data from a voxel data row offset.
const ROW_TAG_MASK: u16 = 0b0111_1111_1111_1111;

// The seed used for the height map.
const HEIGHT_MAP_SEED: u32 = 100;

// A list of all available types of materials.
const M_AIR: u8 = 0;
const M_STONE: u8 = 1;

// The number of different materials available.
const M_COUNT: u8 = 2;

const HEIGHT_MAP_WIDTH: usize = 32;
const HEIGHT_MAP_HEIGHT: usize = 32;

// The number of controllers we will monitor.
const CONTROLLER_COUNT: usize = 1;
const KEYBOARD_CONTROLLER_INDEX: usize = 0;

// Global read-only array of pre-filled arrays of voxels for every material.
lazy_static! {
    static ref VOXEL_ROWS: [[Voxel; VOXEL_ROW_SIZE]; M_COUNT as usize] = {
        let mut rows = [[Voxel { material: M_AIR }; VOXEL_ROW_SIZE]; M_COUNT as usize];

        for i in 0..M_COUNT {
            rows[i as usize] = [Voxel { material: i }; VOXEL_ROW_SIZE];
        }

        rows
    };

    static ref COLOR_WHITE: Color = Color::RGB(255, 255, 255);
    static ref COLOR_BLACK: Color = Color::RGB(0, 0, 0);
    static ref COLOR_GREY: Color = Color::RGB(127, 127, 127);
}

thread_local! {
    /// Whether the game is still running.
    static GLOBAL_RUNNING: RefCell<bool> = RefCell::new(true);
}

#[derive(Deserialize, Serialize)]
struct Game {
    chunk: Chunk,
    x_offset: i32,
    y_offset: i32,
}

struct State {
    game: Game,

    recording_file: Option<File>,
    input_recording_index: Option<usize>,

    playback_file: Option<File>,
    input_playback_index: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GameInput {
    controllers: Vec<ControllerInput>,
}

impl GameInput {
    fn new(controller_count: usize) -> Self {
        assert!(controller_count > 0);
        Self {
            controllers: new_vec(ControllerInput::default(), controller_count),
        }
    }

    fn get_controller(&mut self, index: usize) -> &mut ControllerInput {
        assert!(index < self.controllers.len());

        &mut self.controllers[index]
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ControllerInput {
    move_up: GameButtonState,
    move_down: GameButtonState,
    move_left: GameButtonState,
    move_right: GameButtonState,
}

impl Default for ControllerInput {
    fn default() -> Self {
        Self {
            move_up: Default::default(),
            move_down: Default::default(),
            move_left: Default::default(),
            move_right: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct GameButtonState {
    /// Whether the button is down at the end of the frame.
    ended_down: bool,
    /// Number of transitions from down to up or up to down during the last
    /// frame.
    half_transition_count: usize,
}

impl Default for GameButtonState {
    fn default() -> Self {
        Self {
            ended_down: false,
            half_transition_count: 0,
        }
    }
}

fn begin_recording_input(state: &mut State, input_recording_index: usize) -> Result<(), IoError> {
    debug!("Begin recording input {}", input_recording_index);
    state.input_recording_index = Some(input_recording_index);

    let f = File::create(RECORDING_FILENAME)?;
    state.recording_file = Some(f);

    {
        let mut f = File::create(STATE_FILENAME)?;
        let encoded = serialize(&state.game)
            .unwrap_or_else(|err| panic!("Failed to serialize game: {}", err));
        f.write_all(&encoded)?;
    }

    Ok(())
}

fn end_recording_input(state: &mut State) {
    debug!("End recording input {:?}", state.input_recording_index);
    state.recording_file = None;
    state.input_recording_index = None;
}

fn begin_input_playback(state: &mut State, input_playback_index: usize) -> Result<(), IoError> {
    debug!("Begin input playback {}", input_playback_index);
    state.input_playback_index = Some(input_playback_index);
    let f = File::open(RECORDING_FILENAME)?;
    state.playback_file = Some(f);

    {
        let mut buf = Vec::new();
        let mut f = File::open(STATE_FILENAME)?;
        f.read_to_end(&mut buf)?;
        state.game = deserialize(&buf)
            .unwrap_or_else(|err| panic!("Failed to deserialize game: {}", err));
    }

    Ok(())
}

fn end_input_playback(state: &mut State) {
    debug!("End input playback {:?}", state.input_playback_index);
    state.playback_file = None;
    state.input_playback_index = None;
}

fn record_input(state: &mut State, new_input: &GameInput) -> Result<(), IoError> {
    let mut f = state.recording_file.as_ref().unwrap_or_else(|| panic!("File handle missing"));

    let encoded = serialize(new_input)
        .unwrap_or_else(|err| panic!("Failed to serialize game input: {}", err));
    f.write_all(&encoded)?;

    Ok(())
}

fn playback_input(state: &mut State, new_input: &mut GameInput) -> Result<(), IoError> {
    let mut buf = [0; BINCODED_GAME_INPUT_SIZE];
    let res = {
        let mut f = state.playback_file.as_ref().unwrap_or_else(|| panic!("File handle missing"));
        f.read_exact(&mut buf)
    };
    if let Err(err) = res {
        match err.kind() {
            // Once we've finished playback, we close and re-open the handle to
            // restart playback.
            ErrorKind::UnexpectedEof => {
                let playback_index = state.input_playback_index.unwrap_or_else(|| panic!("Playback index missing"));
                end_input_playback(state);
                begin_input_playback(state, playback_index)?;
                return Ok(());
            },
            _ => return Err(err),
        }
    }

    *new_input = deserialize(&buf)
        .unwrap_or_else(|err| panic!("Failed to deserialize game input: {}", err));
    Ok(())
}

struct RenderContext<'ttf_module, 'rwops> {
    canvas: Canvas<Window>,
    font: Font<'ttf_module, 'rwops>,
}

#[derive(Debug)]
struct WorldGenerator;

impl WorldGenerator {
    // Generate a height map with values from [-1.0..1.0].
    fn generate_height_map() -> [[f64; HEIGHT_MAP_WIDTH]; HEIGHT_MAP_HEIGHT] {
        let mut map = [[0.0; HEIGHT_MAP_WIDTH]; HEIGHT_MAP_HEIGHT];
        let perlin = Perlin::new().set_seed(HEIGHT_MAP_SEED);

        for y in 0..HEIGHT_MAP_HEIGHT {
            for x in 0..HEIGHT_MAP_WIDTH {
                let nx = x as f64 / HEIGHT_MAP_WIDTH as f64 - 0.5;
                let ny = y as f64 / HEIGHT_MAP_HEIGHT as f64 - 0.5;
                map[y][x] = perlin.get([nx, ny]);
            }
        }

        map
    }

    fn generate_chunk_primer() -> ChunkPrimer {
        let height_map = WorldGenerator::generate_height_map();
        let mut data = [Voxel { material: M_AIR }; VOXEL_BOX_SIZE];

        for y in 0..HEIGHT_MAP_HEIGHT {
            for x in 0..HEIGHT_MAP_WIDTH {
                // Set every block on the Z-axis below the height map to a
                // non-air tile.
                let height = (height_map[y][x] * VOXEL_ROW_SIZE as f64).max(0.0) as usize;

                for z in 0..height {
                    data[y << 10 | z << 5 | x].material = M_STONE;
                }
            }
        }

        ChunkPrimer::from_data(data)
    }

    fn generate_chunk() -> Chunk {
        let primer = WorldGenerator::generate_chunk_primer();
        let chunk = Chunk::from_chunk_primer(primer);
        chunk
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
struct Voxel {
    material: u8,
}

fn material_to_color(material: u8) -> Color {
    match material {
        0 => *COLOR_WHITE,
        1 => *COLOR_GREY,
        _ => panic!("Unexpected material: {}", material),
    }
}

struct ChunkPrimer {
    data: [Voxel; VOXEL_BOX_SIZE],
}

impl ChunkPrimer {
    fn from_data(data: [Voxel; VOXEL_BOX_SIZE]) -> Self {
        Self {
            data,
        }
    }

    fn get_index(x: usize, y: usize, z: usize) -> usize {
        z << 10 | y << 5 | x
    }

    fn get_row(&self, y: usize, z: usize) -> &[Voxel] {
        let idx = Self::get_index(0, y, z);
        &self.data[idx..idx + VOXEL_ROW_SIZE]
    }
}

#[derive(Deserialize, Serialize)]
struct Chunk {
    // An array of voxel row metadata.
    //
    // Each element in the array either represents the type of the material at
    // the specified voxel location (for a packed row) or the offset at which
    // the data can be found in the row data array.
    #[serde(with = "a1024")]
    rows: [u16; VOXEL_ROW_COUNT],
    row_data: Vec<Voxel>,
}

impl Chunk {
    fn from_chunk_primer(primer: ChunkPrimer) -> Self {
        let mut rows = [0; VOXEL_ROW_COUNT];
        let mut row_data = Vec::new();

        for z in 0..VOXEL_ROW_SIZE {
            for y in 0..VOXEL_ROW_SIZE {
                let row = primer.get_row(y, z);

                // If every voxel in the row is the same, we used a packed
                // representation for it.
                let material = row[0].material;
                let packed = row == VOXEL_ROWS[material as usize];

                let offset;
                if packed {
                    offset = material as u16;
                } else {
                    offset = row_data.len() as u16 | ROW_TAG_ALLOCATED;
                    row_data.extend_from_slice(row);
                }

                rows[z * VOXEL_ROW_SIZE + y] = offset;
            }
        }

        Self {
            rows,
            row_data,
        }
    }

    fn read_row(&self, y: usize, z: usize) -> &[Voxel] {
        let row = self.rows[z * VOXEL_ROW_SIZE + y];

        let allocated = (row & ROW_TAG_ALLOCATED) != 0;
        let offset = (row & ROW_TAG_MASK) as usize;

        // If the row has been allocated (i.e. it is not packed), return the
        // row. Otherwise, retrieve the representation of a packed row from the
        // global row array and return that.
        if allocated {
            &self.row_data[offset..offset + VOXEL_ROW_SIZE - 1]
        } else {
            &VOXEL_ROWS[offset][..]
        }
    }
}

fn draw_rectangle(canvas: &mut Canvas<Window>, rect: &Rect) {
    canvas.fill_rect(*rect)
        .unwrap_or_else(|err| panic!("Failed to render rect: {}", err));
}

fn draw_texture(canvas: &mut Canvas<Window>, texture: &Texture) {
    canvas.copy(texture, None, None)
        .unwrap_or_else(|err| panic!("Failed to render texture: {}", err));
}

fn render_chunk<'t, 'r>(ctx: &mut RenderContext<'t, 'r>, chunk: &Chunk, x_offset: i32, y_offset: i32) {
    for z in 0..VOXEL_ROW_SIZE {
        for y in 0..VOXEL_ROW_SIZE {
            let row = chunk.read_row(y, z);
            for (x, voxel) in row.iter().enumerate() {
                // We don't render air tiles.
                if voxel.material == M_AIR {
                    continue;
                }

                let color = material_to_color(voxel.material);
                ctx.canvas.set_draw_color(color);

                let rect = Rect::new((x as i32 + x_offset) * VOXEL_RECT_SIZE as i32, (y as i32 + y_offset) * VOXEL_RECT_SIZE as i32, VOXEL_RECT_SIZE as u32, VOXEL_RECT_SIZE as u32);

                draw_rectangle(&mut ctx.canvas, &rect);
            }
        }
    }
}

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

/// Constructs a `Vec<T>` of a specified length populated with the specified value.
fn new_vec<T>(value: T, len: usize) -> Vec<T>
where
    T: Copy,
{
    let mut vec = Vec::new();
    vec.resize(len, value);
    vec
}

pub fn serialize_array<S, T>(array: &[T], serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer, T: Serialize {
    array.serialize(serializer)
}

// From: https://github.com/serde-rs/serde/issues/631#issuecomment-386264396
#[macro_export]
macro_rules! serde_array { ($m:ident, $n:expr) => {
    pub mod $m {
        use std::{ptr, mem};
        use serde::{Deserialize, Deserializer, de};
        pub use $crate::serialize_array as serialize;

        pub fn deserialize<'de, D, T>(deserializer: D) -> Result<[T; $n], D::Error>
        where D: Deserializer<'de>, T: Deserialize<'de> + 'de {
            let slice: Vec<T> = Deserialize::deserialize(deserializer)?;
            if slice.len() != $n {
                return Err(de::Error::custom("input slice has wrong length"));
            }
            unsafe {
                let mut result: [T; $n] = mem::uninitialized();
                for (src, dst) in slice.into_iter().zip(&mut result[..]) {
                    ptr::write(dst, src);
                }
                Ok(result)
            }
        }
    }
}}

serde_array!(a1024, 1024);

fn main() {
    // Initialize the logger.
    env_logger::init();

    let (sdl_context, ttf_context, canvas) = init();

    let font = ttf_context.load_font(FONT_PATH, FONT_SIZE)
        .unwrap_or_else(|err| panic!("Failed to load font: {}", err));

    let texture_creator = canvas.texture_creator();
    let _textures = load_textures(&texture_creator);

    let mut render_ctx = RenderContext { canvas, font };

    // Obtain the SDL2 event pump.
    let mut event_pump = sdl_context.event_pump()
        .unwrap_or_else(|err| panic!("Failed to obtain SDL2 event pump: {}", err));

    let chunk = WorldGenerator::generate_chunk();

    let game = Game {
        chunk,
        x_offset: 0,
        y_offset: 0,
    };

    let mut state = State {
        game,
        recording_file: None,
        input_recording_index: None,
        playback_file: None,
        input_playback_index: None,
    };

    let mut new_input = GameInput::new(CONTROLLER_COUNT);
    let mut old_input = GameInput::new(CONTROLLER_COUNT);

    // Verify the recorded constant for the `GameInput` bincode serialization
    // size versus the result of an actual serialization, to alert the developer
    // that it needs to be changed.
    assert_eq!(serialize(&new_input).unwrap().len(), BINCODED_GAME_INPUT_SIZE);

    // Create a timer which will be used to time the interval between frames.
    let mut fps_timer = Timer::new();

    'running: loop {
        let running = GLOBAL_RUNNING.with(|g| *g.borrow());
        if !running {
            break 'running;
        }

        // Start the frame timer.
        fps_timer.reset();

        {
            let old_keyboard_controller = old_input.get_controller(KEYBOARD_CONTROLLER_INDEX);
            let new_keyboard_controller = new_input.get_controller(KEYBOARD_CONTROLLER_INDEX);
            *new_keyboard_controller = ControllerInput::default();
            new_keyboard_controller.move_up.ended_down = old_keyboard_controller.move_up.ended_down;
            new_keyboard_controller.move_down.ended_down = old_keyboard_controller.move_down.ended_down;
            new_keyboard_controller.move_left.ended_down = old_keyboard_controller.move_left.ended_down;
            new_keyboard_controller.move_right.ended_down = old_keyboard_controller.move_right.ended_down;

            // TODO: iterate over the non-keyboard controllers here.

            // Events
            process_pending_events(&mut state, &mut event_pump, new_keyboard_controller);
        }

        if let Some(_) = state.input_recording_index {
            record_input(&mut state, &new_input)
                .unwrap_or_else(|err| panic!("Input recording failed: {}", err));
        }

        if let Some(_) = state.input_playback_index {
            playback_input(&mut state, &mut new_input)
                .unwrap_or_else(|err| panic!("Input recording failed: {}", err));
        }

        // Update & render
        update_and_render(&mut render_ctx, &texture_creator, &mut state.game, &mut new_input);

        let temp = new_input;
        new_input = old_input;
        old_input = temp;

        // Delay the rendering of the next frame to match the required tick
        // rate.
        while fps_timer.get_ticks() < 1000 / FRAMES_PER_SECOND {}
        trace!("Elapsed ticks: {}", fps_timer.get_ticks());
    }
}

fn init() -> (sdl2::Sdl, sdl2::ttf::Sdl2TtfContext, Canvas<Window>) {
    // Initialize the SDL2 library.
    let sdl_context = sdl2::init().unwrap_or_else(
        |err| panic!("Failed to initialize SDL2 context: {}", err));
    // Initialize the SDL2 video subsystem.
    let video_subsystem = sdl_context.video().unwrap_or_else(
        |err| panic!("Failed to initialize SDL2 video subsystem: {}", err));
    // Initialize the SDL2 TTF API.
    let ttf_context = sdl2::ttf::init().unwrap_or_else(
        |err| panic!("Failed to initialize SDL2 TTF context: {}", err));

    // Create an SDL2 window.
    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap_or_else(|err| panic!("Failed to create SDL2 window: {}", err));

    // Initialize a window-based renderer.
    let canvas = window.into_canvas().build()
        .unwrap_or_else(|err| panic!("Failed to initialize renderer: {}", err));

    (sdl_context, ttf_context, canvas)
}

fn process_key_press(new_state: &mut GameButtonState, is_down: bool) {
    assert!(new_state.ended_down != is_down);
    new_state.ended_down = is_down;
    new_state.half_transition_count += 1;
}

fn process_pending_events(state: &mut State, event_pump: &mut EventPump, new_controller: &mut ControllerInput) {
    for event in event_pump.poll_iter() {
        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;
        trace!("SDL2 event: {:?}", event);
        match event {
            Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } |
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => GLOBAL_RUNNING.with(|g| *g.borrow_mut() = false),
            e @ Event::KeyDown { repeat: false, .. } | e @ Event::KeyUp { repeat: false, .. } => {
                let (keycode, is_down) = match e {
                    Event::KeyDown { keycode, .. } => (keycode, true),
                    Event::KeyUp { keycode, .. } => (keycode, false),
                    _ => unreachable!(),
                };

                match keycode {
                    Some(Keycode::W) => process_key_press(&mut new_controller.move_up, is_down),
                    Some(Keycode::S) => process_key_press(&mut new_controller.move_down, is_down),
                    Some(Keycode::A) => process_key_press(&mut new_controller.move_left, is_down),
                    Some(Keycode::D) => process_key_press(&mut new_controller.move_right, is_down),
                    Some(Keycode::L) => {
                        if !is_down {
                            continue;
                        }

                        if state.input_recording_index.is_none() {
                            begin_recording_input(state, 1)
                                .unwrap_or_else(|err| panic!("Failed to begin recording input: {}", err));
                        } else {
                            end_recording_input(state);
                            begin_input_playback(state, 1)
                                .unwrap_or_else(|err| panic!("Failed to begin input playback: {}", err));
                        }
                    }
                    _ => {},
                }
            },
            _ => {},
        }
    }
}

fn load_textures<'a>(texture_creator: &'a TextureCreator<WindowContext>) -> Vec<Texture<'a>> {
    use sdl2::image::LoadTexture;

    // Initialize the vector which will hold our list of textures.
    let mut textures = Vec::new();

    // Load a texture.
    let texture_path = Path::new("./assets/textures/game_scene/block.png");
    let texture = texture_creator.load_texture(texture_path)
        .unwrap_or_else(|err| panic!("Failed to load texture: {}", err));
    textures.push(texture);

    textures
}

fn update_and_render(ctx: &mut RenderContext, texture_creator: &TextureCreator<WindowContext>, game: &mut Game, input: &mut GameInput) {
    let keyboard_controller = input.get_controller(0);
    if keyboard_controller.move_up.ended_down {
        game.y_offset -= CAMERA_MOVE_SPEED;
    } else if keyboard_controller.move_down.ended_down {
        game.y_offset += CAMERA_MOVE_SPEED;
    }
    if keyboard_controller.move_left.ended_down {
        game.x_offset -= CAMERA_MOVE_SPEED;
    } else if keyboard_controller.move_right.ended_down {
        game.x_offset += CAMERA_MOVE_SPEED;
    }

    render(ctx, texture_creator, game);
}

fn render(ctx: &mut RenderContext, texture_creator: &TextureCreator<WindowContext>, game: &mut Game) {
    ctx.canvas.set_draw_color(*COLOR_BLACK);
    // Clear the current rendering target with the drawing color.
    ctx.canvas.clear();

    render_chunk(ctx, &game.chunk, game.x_offset, game.y_offset);

    // Render the UI.
    let surface = ctx.font.render(&format!("Seed: {}", HEIGHT_MAP_SEED)).solid(*COLOR_WHITE)
        .unwrap_or_else(|err| panic!("Failed to render text: {}", err));
    let texture = texture_creator.create_texture_from_surface(surface)
        .unwrap_or_else(|err| panic!("Failed to create texture from surface: {}", err));
    draw_texture(&mut ctx.canvas, &texture);

    // Display the composed backbuffer to the screen.
    ctx.canvas.present();
}
