pub mod bits;

pub mod register;

pub mod alu;

pub mod memory;

pub mod cpu {
    pub trait CpuDataBus {
        type Data;
        fn load_data(&mut self, data: Self::Data);
        fn read_data(&self) -> Self::Data;
    }

    pub trait CpuAddressBus {
        type Address;
        fn load_address(&mut self, data: Self::Address);
        fn read_address(&self) -> Self::Address;
    }

    pub mod i8080 {
        use crate::cpu::{CpuAddressBus, CpuDataBus};
        use crate::memory::Memory;
        use crate::register::bit8::{Register8, Register8Pair};
        use crate::register::Register;
        use std::collections::HashMap;

        #[derive(Default, Debug)]
        struct I8080 {
            regs: HashMap<I8080RegisterCode, Register8>,
            data_bus_reg: Register8,
            address_bus_reg: Register8Pair,
        }

        #[derive(Debug, Eq, PartialEq, Hash)]
        enum I8080RegisterCode {
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
            pub fn reg_load(&mut self, code: I8080RegisterCode) {
                let data = self.read_data();
                self.regs.entry(code).or_default().load(data);
            }
            pub fn reg_read(&mut self, code: I8080RegisterCode) {
                self.data_bus_reg
                    .load(self.regs.entry(code).or_default().read());
            }
            pub fn reg_address_load(&mut self, code: I8080RegisterCode) {
                self.address_bus_reg.load(u16::from_be_bytes(
                    code.pair().map(|c| self.regs.entry(c).or_default().read()),
                ))
            }
            pub fn store<M: Memory<Data = u8, Address = u16>>(&self, m: &mut M) {
                m.write(self.address_bus_reg.read(), self.read_data())
            }
            pub fn fetch<M: Memory<Data = u8, Address = u16>>(&mut self, m: &mut M) {
                self.load_data(m.read(self.address_bus_reg.read()))
            }
        }

        impl CpuDataBus for I8080 {
            type Data = u8;

            fn load_data(&mut self, data: Self::Data) {
                self.data_bus_reg.load(data)
            }

            fn read_data(&self) -> Self::Data {
                self.data_bus_reg.read()
            }
        }

        impl CpuAddressBus for I8080 {
            type Address = u16;

            fn load_address(&mut self, data: Self::Address) {
                self.address_bus_reg.load(data)
            }

            fn read_address(&self) -> Self::Address {
                self.address_bus_reg.read()
            }
        }

        #[test]
        fn test() {
            use I8080RegisterCode::*;
            let mut cpu = I8080::default();
            cpu.load_data(64);
            cpu.reg_load(B);
            cpu.load_data(32);
            cpu.reg_load(C);
            assert_eq!(cpu.read_data(), 32);
            cpu.reg_read(B);
            assert_eq!(cpu.read_data(), 64);
            cpu.reg_address_load(B);
            assert_eq!(cpu.read_address(), 64 * 256 + 32)
        }
    }
}

pub mod instruction {
    enum Instruction {}
}
