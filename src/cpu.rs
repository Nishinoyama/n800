use crate::memory::Memory;

pub trait CpuDataBus {
    type Data;
    fn load_data(&mut self, data: Self::Data);
    fn data(&self) -> Self::Data;
}

pub trait CpuAddressBus {
    type Address;
    fn load_address(&mut self, data: Self::Address);
    fn address(&self) -> Self::Address;
}

pub trait CpuMemory: CpuDataBus + CpuAddressBus {
    fn store(&mut self);
    fn fetch(&mut self);
}

pub mod i8080 {
    use crate::cpu::{CpuAddressBus, CpuDataBus, CpuMemory};
    use crate::instruction::Instruction;
    use crate::memory::Memory;
    use crate::register::bit8::{Register8, Register8Pair};
    use crate::register::Register;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    pub struct I8080 {
        regs: HashMap<I8080RegisterCode, Register8>,
        data_bus_reg: Register8,
        address_bus_reg: Register8Pair,
        memory: Arc<Mutex<dyn Memory<Address = u16, Data = u8>>>,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub enum I8080RegisterCode {
        A,
        F,
        B,
        C,
        D,
        E,
        S,
        P,
        H,
        L,
    }

    impl I8080RegisterCode {
        pub fn pair(self) -> [Self; 2] {
            use I8080RegisterCode::*;
            match self {
                A | F => [A, F],
                B | C => [B, C],
                D | E => [D, C],
                S | P => [S, P],
                H | L => [H, L],
            }
        }
    }

    impl I8080 {
        fn reg_as_u8(&self, code: I8080RegisterCode) -> u8 {
            self.regs.get(&code).map(|x| x.read()).unwrap_or_default()
        }
        fn reg_as_u16(&self, code: I8080RegisterCode) -> u16 {
            u16::from_be_bytes(
                code.pair()
                    .map(|c| self.regs.get(&c).map(|x| x.read()).unwrap_or_default()),
            )
        }
        pub fn reg_load(&mut self, code: I8080RegisterCode) {
            let data = self.data();
            self.regs.entry(code).or_default().load(data);
        }
        pub fn reg_read(&mut self, code: I8080RegisterCode) {
            self.data_bus_reg
                .load(self.regs.entry(code).or_default().read());
        }
        pub fn reg_address_load(&mut self, code: I8080RegisterCode) {
            self.address_bus_reg.load(self.reg_as_u16(code));
        }
    }

    impl CpuDataBus for I8080 {
        type Data = u8;

        fn load_data(&mut self, data: Self::Data) {
            self.data_bus_reg.load(data)
        }

        fn data(&self) -> Self::Data {
            self.data_bus_reg.read()
        }
    }

    impl CpuAddressBus for I8080 {
        type Address = u16;

        fn load_address(&mut self, data: Self::Address) {
            self.address_bus_reg.load(data)
        }

        fn address(&self) -> Self::Address {
            self.address_bus_reg.read()
        }
    }

    impl CpuMemory for I8080 {
        fn store(&mut self) {
            let address = self.address();
            let data = self.data();
            self.memory.lock().unwrap().write(address, data);
        }

        fn fetch(&mut self) {
            let address = self.address();
            self.memory.lock().unwrap().read(address);
        }
    }

    struct I8080Load {
        dst: I8080RegisterCode,
        src: u8,
    }

    impl Instruction<I8080> for I8080Load {
        fn execute(&self, cpu: &mut I8080) {
            cpu.load_data(self.src);
            cpu.reg_load(self.dst);
        }
    }

    struct I8080Store {
        dst: u16,
        src: u8,
    }

    impl Instruction<I8080> for I8080Store {
        fn execute(&self, cpu: &mut I8080) {
            cpu.load_address(self.dst);
            cpu.load_data(self.src);
        }
    }

    struct Lcd {
        memory: Arc<Mutex<dyn Memory<Address = u16, Data = u8>>>,
    }

    #[test]
    fn test() {
        use crate::memory::RamB8A16;
        use I8080RegisterCode::*;
        let mut memory = RamB8A16::new();
        memory.flash(&[1, 2, 3, 4, 5], 0);
        let memory: Arc<Mutex<(dyn Memory<Data = u8, Address = u16>)>> =
            Arc::new(Mutex::new(memory));
        let mut cpu = I8080 {
            regs: Default::default(),
            data_bus_reg: Default::default(),
            address_bus_reg: Default::default(),
            memory: Arc::clone(&memory),
        };
        let lcd_memory = Arc::clone(&memory);
        let _ = std::thread::spawn(move || {
            let lcd = Lcd { memory: lcd_memory };
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let d = lcd.memory.lock().unwrap().read(1);
                println!("{:08b}", d)
            }
        });
        cpu.load_data(64);
        cpu.reg_load(B);
        cpu.load_data(32);
        cpu.reg_load(C);
        assert_eq!(cpu.data(), 32);
        cpu.reg_read(B);
        assert_eq!(cpu.data(), 64);
        cpu.reg_address_load(B);
        assert_eq!(cpu.address(), 64 * 256 + 32);
        std::thread::sleep(std::time::Duration::from_secs(1));

        cpu.load_address(1);
        cpu.load_data(3);
        cpu.store();
        let mut b = memory.lock().unwrap();
        assert_eq!(b.read(1), 3);
        drop(b);
        std::thread::sleep(std::time::Duration::from_secs(1));

        cpu.load_address(2);
        cpu.load_data(5);
        cpu.store();
        let b = memory.lock().unwrap();
        assert_eq!(b.read(2), 5);
    }
}
