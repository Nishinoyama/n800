pub trait Memory: Send {
    type Data;
    type Address;
    fn write(&mut self, address: Self::Address, data: Self::Data);
    fn read(&self, address: Self::Address) -> Self::Data;
}

#[derive(Debug)]
pub struct RamB8A16 {
    ram: [u8; u16::MAX as usize],
}

impl Default for RamB8A16 {
    fn default() -> Self {
        Self {
            ram: [0; u16::MAX as usize],
        }
    }
}

impl RamB8A16 {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn flash(&mut self, data: &[u8], displacement: u16) {
        let displacement = displacement as usize;
        for (i, &x) in data.iter().enumerate() {
            let i = (displacement + i) % (u16::MAX as usize);
            self.ram[i] = x;
        }
    }
}

impl Memory for RamB8A16 {
    type Data = u8;
    type Address = u16;

    fn write(&mut self, address: Self::Address, data: Self::Data) {
        self.ram[address as usize] = data
    }

    fn read(&self, address: Self::Address) -> Self::Data {
        self.ram[address as usize]
    }
}

mod gb {
    struct GBMemory {
        /// 0x0000-0x3FFF
        cartridge_rom_00: [u8; 32768],
        /// 0x4000-0x7FFF
        cartridge_rom_01: [u8; 32768],
        /// 0x8000-0x9FFF
        video_ram: [u8; 8192],
        /// 0xA000-0xBFFF
        cartridge_ram: [u8; 8192],
        /// 0xC000-0xCFFF
        work_ram_0: [u8; 4096],
        /// 0xD000-0xDFFF
        work_ram_1: [u8; 4096],
        /// 0xFE00-0xFE9F
        sprite_attribute_table: [u8; 256],
        /// 0xFF00-0xFF7F
        io_ports: [u8; 128],
        /// 0XFF80-0xFFFE
        high_rom: [u8; 127],
        /// 0xFFFF
        interrupt_enable_register: u8,
    }
}
