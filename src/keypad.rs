
pub struct Keypad{

    keys: [bool; 16],
}

impl Keypad{

    pub fn new() -> Keypad{

        let keys = [false; 16];

        Keypad{ keys }
    }

    pub fn press(&mut self, key: usize){
        assert!(key < 16);

        //println!("Keypad state: {:X} pressed", key);
        self.keys[key] = true;
    }

    pub fn release(&mut self, key: usize){
        assert!(key < 16);

        //println!("Keypad state: {:X} pressed", key);
        self.keys[key] = false;
    }

    pub fn is_pressed(&self, key: usize) -> bool{
        assert!(key < 16);

        self.keys[key]
    }

    pub fn get_keys(&self) -> [bool; 16]{

        self.keys
    }

    pub fn wait_for_press(&self) -> Option<u8>{

        if let Some(index) = self.keys.iter().position(|&key| key){

            Some(index as u8)
        }else{

            None
        }
    }

}