
pub struct Display{

    buffer: [[bool; 64]; 32],
    needs_update: bool,
}

impl Display{

    pub fn new() -> Display{

        let buffer = [[false; 64]; 32];
        let needs_update = false;
        Display{ buffer, needs_update }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> bool{

        if Self::bound(x, y){

            self.buffer[y][x]
        }else {
            
            false
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, value: bool){

        if Self::bound(x, y){

            self.buffer[y][x] = value;
            self.needs_update = true;
        }
    }

    pub fn flip_pixel(&mut self, x: usize, y: usize){

        if Self::bound(x, y){

            self.buffer[y][x] = !self.buffer[y][x];
            self.needs_update = true;
        }
    }

    fn bound(x: usize, y: usize) -> bool{

        x < 64 && y < 32
    }

    pub fn clear(&mut self){

        self.buffer = [[false; 64]; 32];
        self.needs_update = true;
    }

    pub fn needs_update(&self) -> bool{

        self.needs_update
    }

    pub fn set_needs_update(&mut self, value: bool){

        self.needs_update = value;
    }

    pub fn get_buffer(&self) -> &[[bool; 64]]{

        self.buffer.as_ref()
    }

}