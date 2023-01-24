pub trait Memory {
    type Data;
    type Address;
    fn write(&self, address: Self::Address, data: Self::Data);
    fn read(&self, address: Self::Address) -> Self::Data;
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
