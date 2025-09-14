pub mod memory;
pub mod cpu;
pub mod keypad;
pub mod display;
use std::{fs, sync::{Arc, Mutex}, thread, time::{self, Instant, Duration}};
use winit::{ application::ApplicationHandler, event::*, 
            event_loop::{ActiveEventLoop, EventLoop}, 
            window::{Window, WindowId, WindowAttributes},
            dpi::PhysicalSize,
            keyboard::{ PhysicalKey, KeyCode },
        };

use crate::{cpu::Cpu, keypad::Keypad, display::Display};
use pixels::Pixels;

const WINDOW_SCALE: u16 = 15;
const ON: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const OFF: [u8; 4] = [0xC1, 0x72, 0x22, 0xFF];

fn main() {
    
    println!("Initializing app and peripherals...");
    let game_loop = EventLoop::new().unwrap();
    let mut game_app = App::new();
    println!("Loading rom...");
    game_app.load_rom("PONG.ch8", 0x200);

    start_cpu_thread(Arc::clone(&game_app.cpu), Arc::clone(&game_app.display), Arc::clone(&game_app.keypad));

    println!("Starting loop...");
    game_loop.run_app(&mut game_app).unwrap();
    
}

struct App<'w>{

    window: Option<Arc<Window>>,
    keypad: Arc<Mutex<keypad::Keypad>>,
    display: Arc<Mutex<display::Display>>,
    pixels: Option<Pixels<'w>>,
    cpu: Arc<Mutex<cpu::Cpu>>,

    fps: time::Duration,
    last_frame: time::Instant,
}

impl<'w> App<'w>{

    fn new() -> App<'w>{

        let keypad = Arc::new(Mutex::new(Keypad::new()));
        let display = Arc::new(Mutex::new(Display::new()));
        let cpu = Arc::new(Mutex::new(cpu::Cpu::new()));

        let fps = time::Duration::from_millis(16);
        let last_frame = time::Instant::now();
        App { window: None, keypad, display, pixels: None, cpu, fps, last_frame }
    }

    fn render_display(&mut self){

        if let Some(pixels) = self.pixels.as_mut(){

            let display = self.display.lock().unwrap();
            let frame_buffer = display.get_buffer();
            let pixel_frame = pixels.frame_mut();
            
            for y in 0..32{

                for x in 0..64{

                    let pixel = frame_buffer[y][x];
                    let idx: usize = (y * 64 + x) * 4;
                    if pixel{
                        
                        pixel_frame[idx..idx + 4].copy_from_slice(&ON);
                    }else{

                        pixel_frame[idx..idx + 4].copy_from_slice(&OFF);
                    }
                }
            }
        }
    }

    fn load_rom(&mut self, filename: &str, start_addr: u16){

        let rom_bytes = fs::read(filename).unwrap();

        for index in 0..rom_bytes.len(){

            let byte = rom_bytes[index];
            let mut cpu = self.cpu.lock().unwrap();
            cpu.write_byte_to_mem(byte, index + start_addr as usize);
        }
    
        let mut cpu = self.cpu.lock().unwrap();
        cpu.set_pc(start_addr);
    }

}

impl<'w> ApplicationHandler for App<'w>{

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        
        let size = PhysicalSize::new(64 * WINDOW_SCALE, 32 * WINDOW_SCALE);
        let window_attributes = WindowAttributes::default()
            .with_title("RustChip Chip-8 emulator")
            .with_inner_size(size);
        
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let surface_texture = pixels::SurfaceTexture::new(

            window.inner_size().width,
            window.inner_size().height,
            Arc::clone(&window),
        );

        let pixels = pixels::PixelsBuilder::new(64, 32, surface_texture)
            .build()
            .unwrap();

        self.window = Some(Arc::clone(&window));
        self.pixels = Some(pixels);

        if let Ok(mut cpu) = self.cpu.lock(){

            cpu.set_window(Arc::clone(&window));
        }

    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent){
        
        match event{

            WindowEvent::RedrawRequested => {

                self.render_display();
                if let Some(pixels) = &mut self.pixels{

                    pixels.render().unwrap();
                }
            }
            WindowEvent::CloseRequested => {

                event_loop.exit();
            }
            WindowEvent::KeyboardInput{event: KeyEvent{ physical_key, state, ..}, ..} => {

                println!("Key event: {:?} {:?}", physical_key, state);
                let mut keypad = self.keypad.lock().unwrap();
                match physical_key{

                    PhysicalKey::Code(KeyCode::Digit1) => {

                        let key = 0x1;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::Digit2) => {

                        let key = 0x2;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::Digit3) => {

                        let key = 0x3;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::Digit4) => {

                        let key = 0xC;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyQ) => {

                        let key = 0x4;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyW) => {

                        let key = 0x5;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyE) => {

                        let key = 0x6;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyR) => {

                        let key = 0xD;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyA) => {

                        let key = 0x7;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyS) => {

                        let key = 0x8;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyD) => {

                        let key = 0x9;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyF) => {

                        let key = 0xE;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyZ) => {

                        let key = 0xA;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyX) => {

                        let key = 0x0;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyC) => {

                        let key = 0xB;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    PhysicalKey::Code(KeyCode::KeyV) => {

                        let key = 0xF;
                        if state.is_pressed(){ keypad.press(key) } else { keypad.release(key)};
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {}
}

fn start_cpu_thread(cpu: Arc<Mutex<Cpu>>, display: Arc<Mutex<Display>>, keypad: Arc<Mutex<Keypad>>){

    thread::spawn(move || {
        let cpu_hz = 500;
        let cpu_period = Duration::from_secs_f64(1.0 / cpu_hz as f64);
        let mut last_cpu_tick = Instant::now();

        let timer_hz = 60;
        let timer_period = Duration::from_secs_f64(1.0 / timer_hz as f64);
        let mut last_timer_tick = Instant::now();

        loop {
            let now = Instant::now();

            // --- CPU cycle ---
            if now - last_cpu_tick >= cpu_period {
                let halted = {
                    let cpu_guard = cpu.lock().unwrap();
                    cpu_guard.is_halted()
                };

                if halted {
                    // Only check for key press while halted
                    if let Some(v_x) = {
                        let cpu_guard = cpu.lock().unwrap();
                        cpu_guard.get_wait_register()
                    } {
                        if let Some(key) = keypad.lock().unwrap().wait_for_press() {
                            let mut cpu_guard = cpu.lock().unwrap();
                            cpu_guard.set_register(v_x, key);
                            cpu_guard.set_wait_register(None);
                            cpu_guard.resume();
                        }
                    }
                } else {
                    // Normal cycle
                    let mut cpu_guard = cpu.lock().unwrap();
                    let mut display_guard = display.lock().unwrap();
                    let mut keypad_guard = keypad.lock().unwrap();
                    cpu_guard.cycle(&mut keypad_guard, &mut display_guard);
                }

                last_cpu_tick += cpu_period;
            }

            // --- Timers ---
            if now - last_timer_tick >= timer_period {
                let mut cpu_guard = cpu.lock().unwrap();
                if cpu_guard.get_delay_timer() > 0 {
                    cpu_guard.decrement_delay_timer();
                }
                if cpu_guard.get_sound_timer() > 0 {
                    cpu_guard.decrement_sound_timer();
                }

                last_timer_tick += timer_period;

                let mut display = display.lock().unwrap();
                if display.needs_update(){

                    if let Some(window) = cpu_guard.get_window(){

                        window.request_redraw();
                    }
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    });

}