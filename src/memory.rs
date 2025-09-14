const FONTSET: [u8; 80] = [
            
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ];
const FONTSET_START: usize = 0x50;

pub struct Memory{

    ram: [u8; 4096]
}

impl Memory{

    pub fn new()->Memory{

        let ram:[u8; 4096] = [0; 4096];
        let mut memory = Memory{ram};
        memory.load_font();

        memory
    }

    fn load_font(&mut self){

        for (index, &byte) in FONTSET.iter().enumerate(){

            self.ram[FONTSET_START + index] = byte;
        }
    }

    pub fn load_rom(&mut self, rom_bytes: &[u8], start_addr: usize){
        assert!(start_addr + rom_bytes.len() <= 4096);

        for (index, &byte) in rom_bytes.iter().enumerate(){

            self.ram[start_addr + index] = byte;
        }
    }

    pub fn read_byte(&self, addr: usize)->u8{
        assert!(addr < 4096);

        self.ram[addr]
    }

    pub fn write_byte(&mut self, addr: usize, byte: u8){
        assert!(addr < 4096);

        //println!("Memory[{:03X}] <= {:02X}", addr, byte);
        self.ram[addr] = byte;
    }

    pub fn read_16(&self, addr: usize)->u16{
        assert!(addr + 1 < 4096);

        let first_byte = self.ram[addr];
        let second_byte = self.ram[addr + 1];

        ((first_byte as u16) << 8) | (second_byte as u16)
    }
}