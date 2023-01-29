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
    use crate::alu::bit8::{Adder, DecimalAdjuster, IncDecOperator, LogicalOperator, Rotator};
    use crate::alu::{StatusFlag, ALU};
    use crate::memory::{Memory, RamB8A16};
    use crate::processor::{AddressBus, DataBus, DataBusLoad, DataBusRead, ProcMemory};
    use crate::register::bit8::{Register8, Register8Pair};
    use crate::register::Register;
    use enumset::EnumSet;
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

    pub enum I8080AluCode {
        Add,
        AddCarried,
        Sub,
        SubBorrowed,
        Increment,
        Decrement,
        Daa,
        LogicAnd,
        LogicOr,
        LogicXor,
        RotateLeft,
        RotateRight,
        RotateLeftThroughCarry,
        RotateRightThroughCarry,
        ComplementAcc,
    }

    #[deny(unused_must_use)]
    impl I8080Console {
        fn code_reg_as_u8(&self, code: I8080RegisterCode) -> u8 {
            self.regs
                .get(&code)
                .map(|r| r.reg.read())
                .unwrap_or_else(|| 0)
        }
        fn code_reg16_as_u16(&self, code: I8080RegisterCode16) -> u16 {
            if code == I8080RegisterCode16::SP {
                self.sp_reg.as_u16()
            } else if code == I8080RegisterCode16::PC {
                self.pc_reg.as_u16()
            } else {
                u16::from_be_bytes(code.split().map(|c| self.code_reg_as_u8(c)))
            }
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
        fn acc_reg(&mut self) -> &mut I8080DataReg {
            use I8080RegisterCode::Acc;
            self.code_reg_mut(Acc)
        }
        fn tmp_reg(&mut self) -> &mut I8080DataReg {
            use I8080RegisterCode::Tmp;
            self.code_reg_mut(Tmp)
        }
        fn store_hl(&mut self) {
            use I8080RegisterCode16::HL;
            self.code_reg16_read_to_address(HL);
            self.store()
        }
        fn fetch_hl(&mut self) {
            use I8080RegisterCode16::HL;
            self.code_reg16_read_to_address(HL);
            self.fetch()
        }
        // fixme: unclean
        fn reg16_increment(&mut self, dst: I8080RegisterCode16) {
            let [h, l] = dst.split();
            let mut wz = Register8Pair::new(self.code_reg_mut(h).reg, self.code_reg_mut(l).reg);
            wz.increment();
            let (w, z) = wz.split();
            self.regs.get_mut(&h).unwrap().reg.load(w.read());
            self.regs.get_mut(&l).unwrap().reg.load(z.read());
        }

        #[must_use]
        pub fn code_reg_mut(&mut self, code: I8080RegisterCode) -> &mut I8080DataReg {
            self.regs.entry(code).or_insert_with(|| I8080DataReg {
                reg: Default::default(),
                bus: Rc::clone(&self.data_bus),
            })
        }
        pub fn code_reg16_read_to_address(&self, code: I8080RegisterCode16) {
            self.address_bus.set(self.code_reg16_as_u16(code))
        }

        pub fn fetch_instruction(&mut self) {
            use I8080RegisterCode16::PC;
            self.code_reg16_read_to_address(PC);
            self.fetch();
            self.pc_reg.increment();
        }

        pub fn move_reg_to_reg(&mut self, dst: I8080RegisterCode, src: I8080RegisterCode) {
            self.code_reg_mut(src).read_to_data();
            self.tmp_reg().load_from_data();
            self.tmp_reg().read_to_data();
            self.code_reg_mut(dst).load_from_data();
        }

        pub fn move_hl_mem_to_reg(&mut self, dst: I8080RegisterCode) {
            self.fetch_hl();
            self.code_reg_mut(dst).load_from_data();
        }

        pub fn store_reg_to_hl_mem(&mut self, src: I8080RegisterCode) {
            self.code_reg_mut(src).read_to_data();
            self.tmp_reg().load_from_data();
            self.tmp_reg().read_to_data();
            self.store_hl();
        }

        pub fn move_reg_immediate(&mut self, dst: I8080RegisterCode) {
            self.fetch_instruction();
            self.code_reg_mut(dst).load_from_data();
        }

        pub fn store_hl_immediate(&mut self) {
            self.fetch_instruction();
            self.tmp_reg().load_from_data();
            self.tmp_reg().read_to_data();
            self.store_hl();
        }

        /// special
        pub fn move_sp_from_hl(&mut self) {
            use I8080RegisterCode16::HL;
            self.sp_reg.load(self.code_reg16_as_u16(HL));
        }

        pub fn move_reg16_immediate(&mut self, dst: I8080RegisterCode16) {
            let [h, l] = dst.split();
            self.fetch_instruction();
            self.code_reg_mut(h).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(l).load_from_data();
        }

        /// practically, dst is Acc
        pub fn move_reg_direct(&mut self, dst: I8080RegisterCode) {
            use I8080RegisterCode::{W, Z};
            use I8080RegisterCode16::WZ;
            self.fetch_instruction();
            self.code_reg_mut(W).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(Z).load_from_data();
            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(dst).load_from_data();
        }

        /// practically, src is Acc
        pub fn store_reg_direct(&mut self, src: I8080RegisterCode) {
            use I8080RegisterCode::{W, Z};
            use I8080RegisterCode16::WZ;
            self.fetch_instruction();
            self.code_reg_mut(W).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(Z).load_from_data();
            self.code_reg16_read_to_address(WZ);
            self.code_reg_mut(src).read_to_data();
            self.store();
        }

        /// practically, dst is HL
        pub fn move_reg16_direct(&mut self, dst: I8080RegisterCode16) {
            use I8080RegisterCode::{W, Z};
            use I8080RegisterCode16::WZ;
            let [h, l] = dst.split();
            self.fetch_instruction();
            self.code_reg_mut(W).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(Z).load_from_data();
            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(h).load_from_data();

            self.reg16_increment(WZ);

            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(l).load_from_data();
        }

        /// practically, src is HL
        pub fn store_reg16_direct(&mut self, src: I8080RegisterCode16) {
            use I8080RegisterCode::{W, Z};
            use I8080RegisterCode16::WZ;
            let [h, l] = src.split();
            self.fetch_instruction();
            self.code_reg_mut(W).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(Z).load_from_data();
            self.code_reg16_read_to_address(WZ);
            self.code_reg_mut(h).read_to_data();
            self.store();

            self.reg16_increment(WZ);

            self.code_reg16_read_to_address(WZ);
            self.code_reg_mut(l).read_to_data();
            self.store();
        }

        pub fn move_indirect(&mut self, dst: I8080RegisterCode, src: I8080RegisterCode16) {
            self.code_reg16_read_to_address(src);
            self.fetch();
            self.code_reg_mut(dst).load_from_data();
        }

        pub fn store_indirect(&mut self, dst: I8080RegisterCode, src: I8080RegisterCode16) {
            self.code_reg_mut(dst).read_to_data();
            self.code_reg16_read_to_address(src);
            self.store();
        }

        fn exchange8(&mut self, dst: I8080RegisterCode, src: I8080RegisterCode) {
            let src_tmp = self.code_reg_mut(src).reg.read();
            let dst_tmp = self.code_reg_mut(dst).reg.read();
            self.code_reg_mut(src).reg.load(dst_tmp);
            self.code_reg_mut(dst).reg.load(src_tmp);
        }

        /// special
        pub fn exchange16(&mut self, dst: I8080RegisterCode16, src: I8080RegisterCode16) {
            for (d, s) in dst.split().into_iter().zip(src.split()) {
                self.exchange8(d, s)
            }
        }

        fn flag_decode(flag: StatusFlag) -> u8 {
            match flag {
                StatusFlag::Zero => 64,
                StatusFlag::Sign => 128,
                StatusFlag::Parity => 4,
                StatusFlag::Carry => 1,
                StatusFlag::AuxiliaryCarry => 16,
            }
        }

        fn flag_scramble(status: EnumSet<StatusFlag>) -> u8 {
            status
                .into_iter()
                .fold(2, |acc, f| acc + Self::flag_decode(f))
        }

        fn flag_collect(flags: u8) -> EnumSet<StatusFlag> {
            EnumSet::all()
                .into_iter()
                .filter(|&f| Self::flag_decode(f) & flags > 0)
                .collect()
        }

        fn flag_status(&self) -> EnumSet<StatusFlag> {
            use I8080RegisterCode::Flag;
            Self::flag_collect(
                self.regs
                    .get(&Flag)
                    .map(|r| r.reg.read())
                    .unwrap_or_default(),
            )
        }

        fn alu_from_code(&self, code: I8080AluCode) -> Box<dyn ALU<Data = u8, Flag = StatusFlag>> {
            use I8080AluCode::*;
            use StatusFlag::Carry;
            match code {
                Add => Box::new(Adder::adder()),
                AddCarried => {
                    if self.flag_status().contains(Carry) {
                        Box::new(Adder::carried_adder())
                    } else {
                        Box::new(Adder::adder())
                    }
                }
                Sub => Box::new(Adder::subber()),
                SubBorrowed => {
                    if self.flag_status().contains(Carry) {
                        Box::new(Adder::borrowed_subber())
                    } else {
                        Box::new(Adder::subber())
                    }
                }
                Increment => Box::new(IncDecOperator::Increase),
                Decrement => Box::new(IncDecOperator::Decrease),
                Daa => Box::new(DecimalAdjuster::from_status(self.flag_status())),
                LogicAnd => Box::new(LogicalOperator::And),
                LogicOr => Box::new(LogicalOperator::Or),
                LogicXor => Box::new(LogicalOperator::Xor),
                RotateLeft => Box::new(Rotator::rotate_left()),
                RotateRight => Box::new(Rotator::rotate_right()),
                RotateLeftThroughCarry => Box::new(
                    Rotator::rotate_right()
                        .through_carry()
                        .carried(self.flag_status().contains(Carry)),
                ),
                RotateRightThroughCarry => Box::new(
                    Rotator::rotate_left()
                        .through_carry()
                        .carried(self.flag_status().contains(Carry)),
                ),
                ComplementAcc => Box::new(LogicalOperator::Not),
            }
        }

        fn alu_op(&mut self, alu: Box<dyn ALU<Data = u8, Flag = StatusFlag>>) {
            use I8080RegisterCode::Flag;
            let (res, flag) = alu.op(self.acc_reg().reg.read(), self.tmp_reg().reg.read());
            self.data_bus.set(res);
            self.code_reg_mut(Flag).reg.load(Self::flag_scramble(flag));
        }

        pub fn alu_with_reg(&mut self, alu: I8080AluCode, rhs: I8080RegisterCode) {
            self.code_reg_mut(rhs).read_to_data();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.acc_reg().load_from_data();
        }

        pub fn alu_with_mem(&mut self, alu: I8080AluCode) {
            self.fetch_hl();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.acc_reg().load_from_data();
        }

        pub fn alu_with_immediate(&mut self, alu: I8080AluCode) {
            self.fetch_instruction();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.acc_reg().load_from_data();
        }

        pub fn cmp_with_reg(&mut self, rhs: I8080RegisterCode) {
            use I8080AluCode::Sub;
            self.code_reg_mut(rhs).read_to_data();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(Sub));
        }

        pub fn cmp_with_mem(&mut self) {
            use I8080AluCode::Sub;
            self.fetch_hl();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(Sub));
        }

        pub fn cmp_with_immediate(&mut self) {
            use I8080AluCode::Sub;
            self.fetch_instruction();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(Sub));
        }

        /// practically used for Carry Flag.
        pub fn flag_complement(&mut self, flag: StatusFlag) {
            use I8080RegisterCode::Flag;
            let status = self.flag_status();
            let mut flag_reg = self.code_reg_mut(Flag).reg.masked(Self::flag_decode(flag));
            if status.contains(flag) {
                flag_reg.load(0)
            } else {
                flag_reg.load(!0)
            }
        }

        /// practically used for Carry Flag.
        pub fn flag_set(&mut self, flag: StatusFlag) {
            use I8080RegisterCode::Flag;
            self.code_reg_mut(Flag)
                .reg
                .masked(Self::flag_decode(flag))
                .load(!0);
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

    #[test]
    fn movement() {
        use I8080RegisterCode::*;
        use I8080RegisterCode16::*;
        let mut memory = RamB8A16::default();
        memory.flash(&(0..=255u8).cycle().take(65535).collect::<Vec<_>>(), 0);
        let mut c = I8080Console {
            memory,
            ..Default::default()
        };
        c.move_reg_immediate(B);
        c.move_reg_to_reg(C, B);
        assert_eq!(c.regs.get(&C).unwrap().reg.read(), 0);
        c.move_reg_immediate(D);
        c.move_reg_to_reg(E, D);
        assert_eq!(c.regs.get(&E).unwrap().reg.read(), 1);
        // load (0x0203) == 3
        c.move_reg_direct(Acc);
        assert_eq!(c.regs.get(&Acc).unwrap().reg.read(), 3);
        // loadx (0x0405) == 0x0506
        c.move_reg16_direct(HL);
        assert_eq!(c.code_reg16_as_u16(HL), 0x0506);
    }

    #[test]
    fn alu() {
        use I8080AluCode::*;
        use I8080RegisterCode::*;
        use StatusFlag::*;
        let mut c = I8080Console::default();
        c.acc_reg().reg.load(20);
        c.code_reg_mut(B).reg.load(30);
        c.alu_with_reg(Add, B);
        assert_eq!(c.acc_reg().reg.read(), 50);
        c.acc_reg().reg.load(192);
        c.code_reg_mut(B).reg.load(64);
        c.alu_with_reg(Add, B);
        assert_eq!(c.acc_reg().reg.read(), 0);
        assert_eq!(
            I8080Console::flag_collect(c.code_reg_mut(Flag).reg.read()),
            Carry | Parity | Zero,
        );
        assert_eq!(c.code_reg_mut(Flag).reg.read(), 0b0100_0111);
        c.flag_complement(Carry);
        assert_eq!(
            I8080Console::flag_collect(c.code_reg_mut(Flag).reg.read()),
            Parity | Zero,
        );
        c.flag_complement(Carry);
        assert_eq!(
            I8080Console::flag_collect(c.code_reg_mut(Flag).reg.read()),
            Carry | Parity | Zero,
        );
        c.flag_set(AuxiliaryCarry);
        assert_eq!(
            I8080Console::flag_collect(c.code_reg_mut(Flag).reg.read()),
            AuxiliaryCarry | Carry | Parity | Zero,
        );
    }
}
