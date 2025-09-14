use rand::Rng;
use crate::memory::Memory;
use winit::window::Window;
use rodio::{source::SineWave, OutputStream, Sink, Source};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::thread::sleep;

#[derive(Debug)]
pub enum Instruction {
    //SystemJump(u16),                       0NNN - Jump to RCA 1802 program (legacy) 
    ClearDisplay,                          // 00E0 - Clear the display
    ReturnFromSubroutine,                  // 00EE - Return from subroutine

    JumpToAddress(u16),                    // 1NNN - Jump to address
    CallSubroutine(u16),                   // 2NNN - Call subroutine
    SkipIfVxEqualsByte(usize, u8),             // 3XNN - Skip next instruction if Vx == byte
    SkipIfVxNotEqualsByte(usize, u8),          // 4XNN - Skip next instruction if Vx != byte
    SkipIfVxEqualsVy(usize, usize),               // 5XY0 - Skip if Vx == Vy

    SetVxToByte(usize, u8),                    // 6XNN - Set Vx = byte
    AddByteToVx(usize, u8),                    // 7XNN - Vx += byte

    SetVxToVy(usize, usize),                      // 8XY0
    SetVxToVxOrVy(usize, usize),                  // 8XY1
    SetVxToVxAndVy(usize, usize),                 // 8XY2
    SetVxToVxXorVy(usize, usize),                 // 8XY3
    AddVyToVxWithCarry(usize, usize),              // 8XY4
    SubtractVyFromVxWithBorrow(usize, usize),      // 8XY5
    ShiftVxRightByOne(usize, usize),                  // 8XY6
    SetVxToVyMinusVx(usize, usize),               // 8XY7
    ShiftVxLeftByOne(usize, usize),                   // 8XYE

    SkipIfVxNotEqualsVy(usize, usize),            // 9XY0

    SetIToAddress(u16),                     // ANNN
    JumpToV0PlusAddress(u16),               // BNNN
    SetVxToRandomAndByte(usize, u8),           // CXNN
    DrawSprite(usize, usize, u8),                  // DXYN

    SkipIfKeyInVxPressed(usize),               // EX9E
    SkipIfKeyInVxNotPressed(usize),            // EXA1

    SetVxToDelayTimer(usize),                  // FX07
    WaitForKeyPressAndStoreInVx(usize),        // FX0A
    SetDelayTimerToVx(usize),                  // FX15
    SetSoundTimerToVx(usize),                  // FX18
    AddVxToI(usize),                            // FX1E
    SetIToSpriteAddressForDigitVx(usize),      // FX29
    StoreBcdOfVxAtI(usize),                     // FX33
    StoreRegistersV0ThroughVxInMemory(usize),  // FX55
    ReadRegistersV0ThroughVxFromMemory(usize), // FX65

    Invalid(u16), // Invalid
}

pub struct Cpu{

    v: [u8; 16],
    i: u16,
    pc: u16,
    sp: usize,
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    halted: bool,
    wait_register: Option<usize>,

    mem: Memory,

    stream: rodio::OutputStream,
    sink: rodio::Sink,

    window: Option<Arc<Window>>,
}

impl Cpu{

    pub fn new()->Cpu{

        let v = [0x00; 16];
        let i = 0x0000;
        let pc = 0x0200;
        let sp = 0x00;
        let delay_timer = 0x00;
        let sound_timer = 0x00;

        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());
        let source = SineWave::new(440.0).repeat_infinite();
        sink.append(source);

        let stack = [0x0000; 16];
        let halted = false;
        let wait_register = None;

        let mem = Memory::new();

        Cpu{ v, i, pc, sp, delay_timer, sound_timer, stack, halted, wait_register, mem, stream: stream_handle, sink, window: None }
    }

    pub fn is_halted(&self) -> bool{

        self.halted
    }

    pub fn get_register(&self, index: usize)->u8{
        assert!(index < 16);

        self.v[index]
    }

    pub fn set_register(&mut self, index: usize, data: u8){
        assert!(index < 16);

        self.v[index] = data;
    }

    pub fn push_stack(&mut self)->Result<(), &'static str>{
        if self.sp < 16{

            self.stack[self.sp] = self.pc + 2;
            self.sp += 1;
            Ok(())
        }else{

            Err("stack overflow")
        }

        
    }

    pub fn pop_stack(&mut self)->Option<u16>{
        
        if self.sp > 0{

            self.sp -= 1;
            Some(self.stack[self.sp])
        }else{

            None
        }
    }

    pub fn fetch_instruction(&mut self)->u16{

        let instruction = self.mem.read_16(self.pc as usize);
        instruction
    }

    fn get_nibble(opcode: u16, index:usize)->u8{
        assert!(index < 4);

        match index{

            0 => ((opcode >> 12) & 0xF) as u8,
            1 => ((opcode >> 8) & 0xF) as u8,
            2 => ((opcode >> 4) & 0xF) as u8,
            3 => (opcode & 0xF) as u8,
            _ => {
                eprintln!("Nibble out of range (0..4)");
                0
            }
        }
        
    }

    fn get_x(opcode: u16)->usize{

        Self::get_nibble(opcode, 1) as usize
    }

    fn get_y(opcode: u16)->usize{

        Self::get_nibble(opcode, 2) as usize
    }

    fn get_nn(opcode: u16)->u8{

        (opcode & 0x00FF) as u8
    }

    fn get_nnn(opcode: u16)->u16{

        opcode & 0x0FFF
    }

    fn invalid_opcode(opcode: u16) {
        eprintln!("Warning: Unknown opcode {:04X}, ignoring.", opcode);
    }

    pub fn get_wait_register(&self) -> Option<usize>{

        self.wait_register
    }

    pub fn set_wait_register(&mut self, option: Option<usize>){

        self.wait_register = option;
    }

    pub fn resume(&mut self){

        self.halted = false;
    }

    pub fn write_byte_to_mem(&mut self, byte: u8, index: usize){

        self.mem.write_byte(index, byte);
    }

    pub fn set_pc(&mut self, addr: u16){

        self.pc = addr;
    }

    pub fn get_sound_timer(&self) -> u8{

        self.sound_timer
    }

    pub fn get_delay_timer(&self) -> u8{

        self.delay_timer
    }

    pub fn decrement_sound_timer(&mut self){

        self.sound_timer -= 1;
    }

    pub fn decrement_delay_timer(&mut self){

        self.delay_timer -= 1;
    }

    pub fn set_window(&mut self, window: Arc<Window>){

        self.window = Some(window);
    }

    pub fn get_window(&self) -> Option<Arc<Window>> {
        self.window.as_ref().map(Arc::clone)
    }

    pub fn decode_instruction(opcode: u16)->Instruction{

        let first_nibble = Self::get_nibble(opcode, 0);

        match first_nibble{

            0x0 => {
                let second_nibble = Self::get_nibble(opcode, 1);
                if second_nibble == 0x0{// 00E0 or 00EE

                    let last_two = Self::get_nn(opcode);
                    match last_two{

                        0xE0 => Instruction::ClearDisplay,
                        0xEE => Instruction::ReturnFromSubroutine,
                        _ => Instruction::Invalid(opcode),
                    }
                }else{// 0NNN

                    Instruction::Invalid(opcode)
                }
            },
            0x1 => {// 1NNN
                let address = Self::get_nnn(opcode);

                Instruction::JumpToAddress(address)
            },
            0x2 => {// 2NNN
                let address = Self::get_nnn(opcode);

                Instruction::CallSubroutine(address)
            },
            0x3 => {// 3XNN
                let v_x = Self::get_x(opcode);
                let byte = Self::get_nn(opcode);

                Instruction::SkipIfVxEqualsByte(v_x, byte)
            },
            0x4 => {// 4XNN
                let v_x = Self::get_x(opcode);
                let byte = Self::get_nn(opcode);

                Instruction::SkipIfVxNotEqualsByte(v_x, byte)
            },
            0x5 => {// 5XY0
                if Self::get_nibble(opcode, 3) == 0{
                    let v_x = Self::get_x(opcode);
                    let v_y = Self::get_y(opcode);

                    Instruction::SkipIfVxEqualsVy(v_x, v_y)
                }else{

                    Instruction::Invalid(opcode)
                }
            },
            0x6 => {// 6XNN
                let v_x = Self::get_x(opcode);
                let byte = Self::get_nn(opcode);

                Instruction::SetVxToByte(v_x, byte)
            },
            0x7 => {// 7XNN
                let v_x = Self::get_x(opcode);
                let byte = Self::get_nn(opcode);

                Instruction::AddByteToVx(v_x, byte)
            },
            0x8 => {
                let v_x = Self::get_x(opcode);
                let v_y = Self::get_y(opcode);
                let last_nibble = Self::get_nibble(opcode, 3);
                
                match last_nibble{

                    0x0 => Instruction::SetVxToVy(v_x, v_y),
                    0x1 => Instruction::SetVxToVxOrVy(v_x, v_y),
                    0x2 => Instruction::SetVxToVxAndVy(v_x, v_y),
                    0x3 => Instruction::SetVxToVxXorVy(v_x, v_y),
                    0x4 => Instruction::AddVyToVxWithCarry(v_x, v_y),
                    0x5 => Instruction::SubtractVyFromVxWithBorrow(v_x, v_y),
                    0x6 => Instruction::ShiftVxRightByOne(v_x, v_y),
                    0x7 => Instruction::SetVxToVyMinusVx(v_x, v_y),
                    0xE => Instruction::ShiftVxLeftByOne(v_x, v_y),
                    _ => Instruction::Invalid(opcode),
                }
            },
            0x9 => {// 9XY0
                if Self::get_nibble(opcode, 3) ==0{
                    let v_x = Self::get_x(opcode);
                    let v_y = Self::get_y(opcode);

                    Instruction::SkipIfVxNotEqualsVy(v_x, v_y)
                }else{

                    Instruction::Invalid(opcode)
                }
            },
            0xA => {// ANNN
                let address = Self::get_nnn(opcode);

                Instruction::SetIToAddress(address)
            },
            0xB => {// BNNN
                let address = Self::get_nnn(opcode);

                Instruction::JumpToV0PlusAddress(address)
            },
            0xC => {// CXNN
                let v_x = Self::get_x(opcode);
                let byte = Self::get_nn(opcode);

                Instruction::SetVxToRandomAndByte(v_x, byte)
            },
            0xD => {// DXYN
                let v_x = Self::get_x(opcode);
                let v_y = Self::get_y(opcode);
                let n = Self::get_nibble(opcode, 3);

                Instruction::DrawSprite(v_x, v_y, n)
            },
            0xE => {
                let last_byte = Self::get_nn(opcode);
                let v_x = Self::get_x(opcode);
                match last_byte{

                    0x9E => Instruction::SkipIfKeyInVxPressed(v_x),
                    0xA1 => Instruction::SkipIfKeyInVxNotPressed(v_x),
                    _ => Instruction::Invalid(opcode),
                }

            },
            0xF => {
                let last_byte = Self::get_nn(opcode);
                let v_x = Self::get_x(opcode);
                match last_byte{

                    0x07 => Instruction::SetVxToDelayTimer(v_x),
                    0x0A => Instruction::WaitForKeyPressAndStoreInVx(v_x),
                    0x15 => Instruction::SetDelayTimerToVx(v_x),
                    0x18 => Instruction::SetSoundTimerToVx(v_x),
                    0x1E => Instruction::AddVxToI(v_x),
                    0x29 => Instruction::SetIToSpriteAddressForDigitVx(v_x),
                    0x33 => Instruction::StoreBcdOfVxAtI(v_x),
                    0x55 => Instruction::StoreRegistersV0ThroughVxInMemory(v_x),
                    0x65 => Instruction::ReadRegistersV0ThroughVxFromMemory(v_x),
                    _ => Instruction::Invalid(opcode),
                }

            },
            _ => Instruction::Invalid(opcode),
            
        }
    }

    pub fn execute_instruction(&mut self, instruction:Instruction, keypad: &mut crate::keypad::Keypad, display: &mut crate::display::Display){
        let mut pc_modified = false;

        match instruction{

                Instruction::ClearDisplay => {//Clear the display.

                    display.clear();
                },
                Instruction::ReturnFromSubroutine => {// The interpreter sets the program counter to the address at the top of the stack, then subtracts 1 from the stack pointer.

                    match self.pop_stack(){

                        Some(address) =>{

                            self.pc = address;
                            pc_modified = true;
                        },
                        None => {
                            panic!("Stack underflow (cant pop a stack that is empty)")
                        }
                    }
                },                  

                Instruction::JumpToAddress(address) => {//The interpreter sets the program counter to nnn.

                    //println!("Executing JumpToAddress({:X})", address);
                    //println!("PC before: {:X}, PC after: {:X}", self.pc, address);
                    self.pc = address;
                    pc_modified = true;
                },
                Instruction::CallSubroutine(address) => {// The interpreter increments the stack pointer, then puts the current PC on the top of the stack. The PC is then set to nnn.

                    match self.push_stack(){

                        Ok(()) => {

                            self.pc = address;
                            pc_modified = true;
                        },
                        Err(e) => {
                            
                            panic!("Error: {:?}", e);
                        }
                    }
                },
                Instruction::SkipIfVxEqualsByte(v_x, byte) => {// The interpreter compares register Vx to kk, and if they are equal, increments the program counter by 2.

                    if self.get_register(v_x) == byte{

                        self.pc += 2;
                    }
                },
                Instruction::SkipIfVxNotEqualsByte(v_x, byte) => {// The interpreter compares register Vx to kk, and if they are not equal, increments the program counter by 2.

                    if self.get_register(v_x) != byte{

                        self.pc += 2;
                    }
                },
                Instruction::SkipIfVxEqualsVy(v_x, v_y) => {// The interpreter compares register Vx to register Vy, and if they are equal, increments the program counter by 2.

                    if self.get_register(v_x) == self.get_register(v_y){

                        self.pc += 2;
                    }
                },

                Instruction::SetVxToByte(v_x, byte) => {// The interpreter puts the value nn into register Vx.

                    self.set_register(v_x, byte);
                },
                Instruction::AddByteToVx(v_x, byte) => {// Adds the value nn to the value of register Vx, then stores the result in Vx.

                    self.set_register(v_x, self.get_register(v_x).wrapping_add(byte));
                },
                Instruction::SetVxToVy(v_x, v_y) => {// Stores the value of register Vy in register Vx.

                    self.set_register(v_x, self.get_register(v_y));
                },
                Instruction::SetVxToVxOrVy(v_x, v_y) => {// Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.

                    self.set_register(v_x, self.get_register(v_x) | self.get_register(v_y));
                },
                Instruction::SetVxToVxAndVy(v_x, v_y) => {// Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.

                    self.set_register(v_x, self.get_register(v_x) & self.get_register(v_y));
                },
                Instruction::SetVxToVxXorVy(v_x, v_y) => {// Performs a bitwise XOR on the values of Vx and Vy, then stores the result in Vx.

                    self.set_register(v_x, self.get_register(v_x) ^ self.get_register(v_y));
                },
                Instruction::AddVyToVxWithCarry(v_x, v_y) => {// The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.

                    let (sum, carry) = self.get_register(v_x).overflowing_add(self.get_register(v_y));
                    self.set_register(v_x, sum);
                    if carry{
                        self.set_register(0xF, 1);
                    }else{
                        self.set_register(0xF, 0);
                    }
                },
                Instruction::SubtractVyFromVxWithBorrow(v_x, v_y) => {// If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.

                    let (result, borrow) = self.get_register(v_x).overflowing_sub(self.get_register(v_y));
                    self.set_register(v_x, result);
                    if borrow{

                        self.set_register(0xF, 0);
                    }else{

                        self.set_register(0xF, 1);
                    }
                },
                Instruction::ShiftVxRightByOne(v_x, v_y) => {// Shifts Vy to the right by one and stores the shifted bit in Vf.

                    let original = self.get_register(v_y);
                    let bit = original & 0x1;
                    let shifted = original >> 1;
                    self.set_register(v_x, shifted);
                    self.set_register(0xF, bit);
                },
                Instruction::SetVxToVyMinusVx(v_x, v_y) => {// If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.

                    let (result, borrow) = self.get_register(v_y).overflowing_sub(self.get_register(v_x));
                    self.set_register(v_x, result);
                    if borrow{

                        self.set_register(0xF, 0);
                    }else{

                        self.set_register(0xF, 1);
                    }
                },
                Instruction::ShiftVxLeftByOne(v_x, v_y) => {// Shifts Vy to the left by one and stores the shifted bit in Vf.

                    let original = self.get_register(v_y);
                    let bit = (original & 0x80) >> 7;
                    let shifted = original << 1;
                    self.set_register(v_x, shifted);
                    self.set_register(0xF, bit);
                },

                Instruction::SkipIfVxNotEqualsVy(v_x, v_y) => {// The values of Vx and Vy are compared, and if they are not equal, the program counter is increased by 2.

                    if self.get_register(v_x) != self.get_register(v_y){

                        self.pc += 2;
                    }
                },

                Instruction::SetIToAddress(address) => {// The value of register I is set to nnn.

                    self.i = address;
                },
                Instruction::JumpToV0PlusAddress(address) => {// The program counter is set to nnn plus the value of V0.

                    self.pc = address + self.get_register(0x0) as u16;
                    pc_modified = true;
                },
                Instruction::SetVxToRandomAndByte(v_x, byte) => {// The interpreter generates a random number from 0 to 255, which is then ANDed with the value nn. The results are stored in Vx.

                    let rnd = rand::rng().random_range(0..=255) as u8;
                    self.set_register(v_x, rnd & byte);
                },
                Instruction::DrawSprite(v_x, v_y, n) => {// The interpreter reads n bytes from memory, starting at the address stored in I. These bytes are then displayed as sprites on screen at coordinates (Vx, Vy). Sprites are XORed onto the existing screen. If this causes any pixels to be erased, VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is outside the coordinates of the display, it wraps around to the opposite side of the screen.

                    let mut collision = false;
                    self.set_register(0xF, 0);
                    let v_x = self.get_register(v_x);
                    let v_y = self.get_register(v_y);

                    //println!("Drawing sprite at ({}, {}) with {} rows", v_x, v_y, n);
                    for row in 0..n{

                        let sprite_byte = self.mem.read_byte((self.i + row as u16) as usize);
                        let y = ((v_y + row) % 32) as usize;

                        //println!("Row {}: {:08b}", row, sprite_byte);
                        for bit_index in 0..8{

                            let x = ((v_x + bit_index) % 64) as usize;
                            if sprite_byte & (0b1000_0000 >> bit_index) != 0{

                                let was_on = display.get_pixel(x, y);
                                display.flip_pixel(x, y);
                                if was_on{

                                    collision = true;
                                }

                                //println!(
                                  //  "  Pixel ON at ({}, {}), was {}",
                                    //x,
                                    //y,
                                    //if was_on { "ON" } else { "OFF" }
                                //);
                            }
                        }
                    }

                    if collision{

                        self.set_register(0xF, 1);
                    }else{

                        self.set_register(0xF, 0);
                    }
                    //println!("VF set to {}\n", self.get_register(0xF));
                },

                Instruction::SkipIfKeyInVxPressed(v_x) => {// Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.

                    let key = self.get_register(v_x);
                    if keypad.is_pressed(key as usize){

                        //println!("CPU sees key {:X} pressed", key);
                        self.pc += 2;
                    }
                },
                Instruction::SkipIfKeyInVxNotPressed(v_x) => {// Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.

                    let key = self.get_register(v_x);
                    if !keypad.is_pressed(key as usize){

                        //intln!("CPU sees key {:X} not pressed", key);
                        self.pc += 2;
                    }
                },

                Instruction::SetVxToDelayTimer(v_x) => {// The value of DT is placed into Vx.

                    self.set_register(v_x, self.delay_timer);
                },
                Instruction::WaitForKeyPressAndStoreInVx(v_x) => {// All execution stops until a key is pressed, then the value of that key is stored in Vx.

                    self.halted = true;
                    self.wait_register = Some(v_x);
                },
                Instruction::SetDelayTimerToVx(v_x) => {// DT is set equal to the value of Vx.

                    self.delay_timer = self.get_register(v_x);
                },
                Instruction::SetSoundTimerToVx(v_x) => {// ST is set equal to the value of Vx.

                    self.sound_timer = self.get_register(v_x);
                },
                Instruction::AddVxToI(v_x) => {// The values of I and Vx are added, and the results are stored in I.

                    self.i = self.i + self.get_register(v_x) as u16;
                },
                Instruction::SetIToSpriteAddressForDigitVx(v_x) => {// The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx

                    let font_start: usize = 0x50;
                    let sprite_index = (self.get_register(v_x)  as usize * 5) + font_start;
                    self.i = sprite_index as u16;
                },
                Instruction::StoreBcdOfVxAtI(v_x) => {// The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2
                    let value = self.get_register(v_x);
                    let (hundreds, tens, ones) = ((value / 100) % 10, (value / 10) % 10, value % 10);
                    self.mem.write_byte(self.i as usize, hundreds);
                    self.mem.write_byte((self.i + 1) as usize, tens);
                    self.mem.write_byte((self.i + 2) as usize, ones);

                },
                Instruction::StoreRegistersV0ThroughVxInMemory(v_x) => {// The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.

                    for (index, register) in (0x0..=v_x).enumerate(){
                        
                        self.mem.write_byte(self.i as usize + index, self.get_register(register));
                    }
                },
                Instruction::ReadRegistersV0ThroughVxFromMemory(v_x) => {// The interpreter reads values from memory starting at location I into registers V0 through Vx.

                    for (index, register) in (0x0..=v_x).enumerate(){

                        self.set_register(register, self.mem.read_byte(self.i as usize + index));
                    }
                },

                Instruction::Invalid(opcode) => {// Catch all for invalid opcodes

                    Self::invalid_opcode(opcode);
                },
        }

        if !pc_modified{

            self.pc += 2;
        }

    }

    pub fn cycle(&mut self, keypad: &mut crate::keypad::Keypad, display: &mut crate::display::Display){

        let opcode = self.fetch_instruction();

        //println!("--- Cycle Start ---");
        //println!("PC: {:03X}", self.pc);
        //println!("Opcode: {:04X}", opcode);
        //println!("Registers: {:?}", self.v);
        //println!("I: {:03X}", self.i);
        //println!("SP: {}", self.sp);
        //println!("Stack: {:?}", &self.stack[..self.sp as usize]);
        //println!("DT: {}, ST: {}", self.delay_timer, self.sound_timer);

        let instruction = Self::decode_instruction(opcode);

        //println!("Decoded instruction: {:?}", instruction);

        self.execute_instruction(instruction, keypad, display);

        if self.sound_timer > 0{

            //println!("Playing audio");
            self.sink.play();
        }else{

            self.sink.pause();
            //println!("Pausing audio");
        }

    }

    

}