pub trait DataBus {
    type Data;
    fn set_data(&mut self, data: Self::Data);
    fn get_data(&self) -> Self::Data;
}

pub trait AddressBus {
    type Address;
    fn set_address(&mut self, data: Self::Address);
    fn get_address(&self) -> Self::Address;
}

pub trait DataBusLoad {
    type DataBus: DataBus;
    fn load_from_data(&mut self);
}

pub trait DataBusRead {
    type DataBus: DataBus;
    fn read_to_data(&self);
}

pub trait AddressBusLoad {
    type AddressBus: AddressBus;
    fn load_address(&mut self);
}

pub trait AddressBusRead {
    type AddressBus: AddressBus;
    fn read_address(&self);
}

pub trait ProcMemory {
    fn store(&mut self);
    fn fetch(&mut self);
}

pub mod i8080 {
    use crate::cpu::{AddressBus, DataBus, DataBusLoad, DataBusRead, ProcMemory};
    use crate::memory::{Memory, RamB8A16};
    use crate::register::bit8::{Register8, Register8Pair};
    use crate::register::Register;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[derive(Debug, Clone)]
    pub struct I8080DataReg {
        reg: Register8,
        bus: Rc<Cell<u8>>,
    }

    impl DataBusRead for I8080DataReg {
        type DataBus = u8;

        fn read_to_data(&self) {
            self.bus.set(self.reg.read())
        }
    }

    impl DataBusLoad for I8080DataReg {
        type DataBus = u8;

        fn load_from_data(&mut self) {
            self.reg.load(self.bus.get())
        }
    }

    impl DataBus for u8 {
        type Data = u8;

        fn set_data(&mut self, data: Self::Data) {
            *self = data
        }

        fn get_data(&self) -> Self::Data {
            *self
        }
    }

    impl AddressBus for u16 {
        type Address = u16;

        fn set_address(&mut self, data: Self::Address) {
            *self = data
        }

        fn get_address(&self) -> Self::Address {
            *self
        }
    }

    #[derive(Default, Debug)]
    pub struct I8080Console {
        data_bus: Rc<Cell<u8>>,
        address_bus: Rc<Cell<u16>>,
        memory: RamB8A16,
        regs: HashMap<I8080RegisterCode, I8080DataReg>,
        inst_reg: Register8,
        pc_reg: Register8Pair,
        sp_reg: Register8Pair,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub enum I8080RegisterCode {
        Acc,
        Flag,
        B,
        C,
        D,
        E,
        H,
        L,
        Tmp,
        Inst,
        W,
        Z,
    }

    impl I8080RegisterCode {
        pub fn pair(self) -> [Self; 2] {
            use I8080RegisterCode::*;
            match self {
                Acc | Flag => [Acc, Flag],
                B | C => [B, C],
                D | E => [D, C],
                W | Z => [W, Z],
                H | L => [H, L],
                other => panic!("No Pair for {:?}!", other),
            }
        }
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub enum I8080RegisterCode16 {
        BC,
        DE,
        HL,
        SP,
        PC,
        WZ,
    }

    impl I8080RegisterCode16 {
        pub fn split(self) -> [I8080RegisterCode; 2] {
            use I8080RegisterCode::*;
            use I8080RegisterCode16::*;
            match self {
                BC => [B, C],
                DE => [D, E],
                HL => [H, L],
                WZ => [W, Z],
                other => panic!("{:?} Can't be Split", other),
            }
        }
    }

    impl I8080Console {
        fn code_reg_as_u8(&self, code: I8080RegisterCode) -> u8 {
            self.regs
                .get(&code)
                .map(|r| r.reg.read())
                .unwrap_or_else(|| 0)
        }
        fn code_reg16_as_u16(&self, code: I8080RegisterCode16) -> u16 {
            u16::from_be_bytes(code.split().map(|c| self.code_reg_as_u8(c)))
        }
        fn load_code_reg16_from_wz(&mut self, code: I8080RegisterCode16) {
            use I8080RegisterCode::{W, Z};
            let [h, l] = code.split();
            self.code_reg_mut(W).read_to_data();
            self.code_reg_mut(h).load_from_data();
            self.code_reg_mut(Z).read_to_data();
            self.code_reg_mut(l).load_from_data();
        }
        fn load_wz_from_code_reg16(&mut self, code: I8080RegisterCode16) {
            use I8080RegisterCode::{W, Z};
            let [h, l] = code.split();
            self.code_reg_mut(h).read_to_data();
            self.code_reg_mut(W).load_from_data();
            self.code_reg_mut(l).read_to_data();
            self.code_reg_mut(Z).load_from_data();
        }
        pub fn code_reg_mut(&mut self, code: I8080RegisterCode) -> &mut I8080DataReg {
            self.regs.entry(code).or_insert_with(|| I8080DataReg {
                reg: Default::default(),
                bus: Rc::clone(&self.data_bus),
            })
        }
        pub fn code_reg16_read_to_address(&self, code: I8080RegisterCode16) {
            self.address_bus.set(self.code_reg16_as_u16(code))
        }
        pub fn fetch_instruction(&mut self, nth: usize) {
            use I8080RegisterCode::{Inst, W, Z};
            use I8080RegisterCode16::PC;
            self.code_reg16_read_to_address(PC);
            self.sp_reg.increment();
            match nth {
                0 => self.code_reg_mut(Inst).load_from_data(),
                1 => self.code_reg_mut(W).load_from_data(),
                2 => self.code_reg_mut(Z).load_from_data(),
                _ => panic!("Too Much Instruction Operand!"),
            }
        }
    }

    impl ProcMemory for I8080Console {
        fn store(&mut self) {
            self.memory
                .write(self.address_bus.get(), self.data_bus.get())
        }

        fn fetch(&mut self) {
            self.data_bus.set(self.memory.read(self.address_bus.get()));
        }
    }

    #[test]
    fn test() {
        use I8080RegisterCode::*;
        use I8080RegisterCode16::*;
        let mut c = I8080Console::default();
        let bus = Rc::clone(&c.data_bus);
        bus.set(35);
        c.code_reg_mut(B).load_from_data();
        bus.set(0x12);
        c.code_reg_mut(H).load_from_data();
        bus.set(0x34);
        c.code_reg_mut(L).load_from_data();
        c.code_reg_mut(B).read_to_data();
        c.code_reg16_read_to_address(HL);
        c.store();
        assert_eq!(c.memory.read(0x1234), 35);
    }
}
