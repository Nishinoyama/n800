/// Processor that can fetch data from its memory and make it store data.
pub trait ProcMemory {
    fn store(&mut self);
    fn fetch(&mut self);
}

pub trait ProcInstructionFetch {
    fn fetch_instruction(&mut self);
}

pub trait ProcImmediateValue: ProcInstructionFetch {
    fn fetch_immediate_value(&mut self) {
        self.fetch_instruction();
    }
}

pub mod i8080 {
    use crate::alu::bit8::{Adder, DecimalAdjuster, IncDecOperator, LogicalOperator, Rotator};
    use crate::alu::{StatusFlag, ALU};
    use crate::bus::{AddressBus, DataBus, DataBusLoad, DataBusRead};
    use crate::memory::{Memory, RamB8A16};
    use crate::processor::ProcMemory;
    use crate::register::bit8::{Register8, Register8Pair};
    use crate::register::Register;
    use enumset::EnumSet;
    use std::borrow::BorrowMut;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::io::Read;
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
        halted: bool,
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
        PcH,
        PcL,
        SpH,
        SpL,
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
        PSW,
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
                PSW => [Acc, Flag],
                BC => [B, C],
                DE => [D, E],
                HL => [H, L],
                WZ => [W, Z],
                SP => [SpH, SpL],
                PC => [PcH, PcL],
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
        DecimalAdjust,
        LogicAnd,
        LogicOr,
        LogicXor,
        RotateLeft,
        RotateRight,
        RotateLeftThroughCarry,
        RotateRightThroughCarry,
        ComplementAcc,
    }

    pub enum I8080JumpCondition {
        Anytime,
        OnNonZero,
        OnZero,
        OnNonCarry,
        OnCarry,
        OnParityOdd,
        OnParityEven,
        OnPlus,
        OnMinus,
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
            u16::from_be_bytes(code.split().map(|c| self.code_reg_as_u8(c)))
        }
        fn load_reg16_from_reg16(&mut self, dst: I8080RegisterCode16, src: I8080RegisterCode16) {
            dst.split().into_iter().zip(src.split()).for_each(|(d, s)| {
                self.code_reg_mut(s).read_to_data();
                self.code_reg_mut(d).load_from_data();
            })
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
        fn fetch_operand_to_wz(&mut self) {
            use I8080RegisterCode::{W, Z};
            // little endian
            self.fetch_instruction();
            self.code_reg_mut(Z).load_from_data();
            self.fetch_instruction();
            self.code_reg_mut(W).load_from_data();
        }
        fn reg16_increment(&mut self, dst: I8080RegisterCode16) {
            let [h, l] = dst
                .split()
                .map(|c| self.regs.remove(&c).map(|r| r.reg).unwrap_or_default());
            let mut inc = Register8Pair::new(h, l);
            inc.increment();
            dst.split()
                .into_iter()
                .zip(inc.split())
                .for_each(|(c, reg)| {
                    self.regs.insert(
                        c,
                        I8080DataReg {
                            reg,
                            bus: Rc::clone(&self.data_bus),
                        },
                    );
                })
        }

        fn reg16_decrement(&mut self, dst: I8080RegisterCode16) {
            let [h, l] = dst
                .split()
                .map(|c| self.regs.remove(&c).map(|r| r.reg).unwrap_or_default());
            let mut inc = Register8Pair::new(h, l);
            inc.decrement();
            dst.split()
                .into_iter()
                .zip(inc.split())
                .for_each(|(c, reg)| {
                    self.regs.insert(
                        c,
                        I8080DataReg {
                            reg,
                            bus: Rc::clone(&self.data_bus),
                        },
                    );
                })
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
            self.reg16_increment(PC);
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
            use I8080RegisterCode16::{HL, SP};
            self.load_reg16_from_reg16(SP, HL);
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
            use I8080RegisterCode16::WZ;
            self.fetch_operand_to_wz();
            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(dst).load_from_data();
        }

        /// practically, src is Acc
        pub fn store_reg_direct(&mut self, src: I8080RegisterCode) {
            use I8080RegisterCode16::WZ;
            self.fetch_operand_to_wz();
            self.code_reg16_read_to_address(WZ);
            self.code_reg_mut(src).read_to_data();
            self.store();
        }

        /// practically, dst is HL
        pub fn move_reg16_direct(&mut self, dst: I8080RegisterCode16) {
            use I8080RegisterCode16::WZ;
            let [h, l] = dst.split();
            self.fetch_operand_to_wz();
            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(l).load_from_data();

            self.reg16_increment(WZ);

            self.code_reg16_read_to_address(WZ);
            self.fetch();
            self.code_reg_mut(h).load_from_data();
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
                DecimalAdjust => Box::new(DecimalAdjuster::from_status(self.flag_status())),
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

        pub fn alu_with_reg_to_reg(&mut self, alu: I8080AluCode, rhs: I8080RegisterCode) {
            self.code_reg_mut(rhs).read_to_data();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.code_reg_mut(rhs).load_from_data();
        }

        pub fn alu_with_mem(&mut self, alu: I8080AluCode) {
            self.fetch_hl();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.acc_reg().load_from_data();
        }

        pub fn alu_with_mem_to_mem(&mut self, alu: I8080AluCode) {
            self.fetch_hl();
            self.tmp_reg().load_from_data();
            self.alu_op(self.alu_from_code(alu));
            self.store();
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

        fn flag_objected_by(cond: I8080JumpCondition) -> (StatusFlag, bool) {
            use I8080JumpCondition::*;
            use StatusFlag::*;
            match cond {
                OnNonZero => (Zero, false),
                OnZero => (Zero, true),
                OnNonCarry => (Carry, false),
                OnCarry => (Carry, true),
                OnParityOdd => (Parity, false),
                OnParityEven => (Parity, true),
                OnPlus => (Sign, false),
                OnMinus => (Sign, true),
                _ => unreachable!(),
            }
        }

        fn satisfying_condition(&mut self, cond: I8080JumpCondition) -> bool {
            use I8080JumpCondition::*;
            let set = |cond| {
                let (flag, set) = Self::flag_objected_by(cond);
                self.flag_status().contains(flag) == set
            };
            match cond {
                Anytime => true,
                other => set(other),
            }
        }

        pub fn jump_immediate(&mut self, cond: I8080JumpCondition) {
            use I8080RegisterCode16::{PC, WZ};
            self.fetch_operand_to_wz();
            if self.satisfying_condition(cond) {
                self.load_reg16_from_reg16(PC, WZ);
            }
        }

        pub fn call_immediate(&mut self, cond: I8080JumpCondition) {
            use I8080RegisterCode16::{PC, SP, WZ};
            self.fetch_operand_to_wz();
            if self.satisfying_condition(cond) {
                let [pch, pcl] = PC.split();
                self.reg16_increment(SP);
                self.code_reg_mut(pch).read_to_data();
                self.code_reg16_read_to_address(SP);
                self.store();

                self.reg16_decrement(SP);
                self.code_reg_mut(pcl).read_to_data();
                self.code_reg16_read_to_address(SP);
                self.store();

                self.load_reg16_from_reg16(PC, WZ);
            }
        }
        pub fn ret(&mut self, cond: I8080JumpCondition) {
            use I8080RegisterCode::{W, Z};
            use I8080RegisterCode16::{PC, SP, WZ};
            if self.satisfying_condition(cond) {
                self.code_reg16_read_to_address(SP);
                self.fetch();
                self.code_reg_mut(Z).load_from_data();
                self.reg16_increment(SP);

                self.code_reg16_read_to_address(SP);
                self.fetch();
                self.code_reg_mut(W).load_from_data();
                self.reg16_increment(SP);

                self.load_reg16_from_reg16(PC, WZ);
            }
        }

        /// restart, that equals `call n*8`
        pub fn restart(&mut self, n: u8) {
            use I8080RegisterCode16::{PC, SP, WZ};
            let [pch, pcl] = [0, n * 8];
            self.reg16_decrement(SP);
            self.data_bus.set(pch);
            self.code_reg16_read_to_address(SP);
            self.store();

            self.reg16_decrement(SP);
            self.data_bus.set(pcl);
            self.code_reg16_read_to_address(SP);
            self.store();

            self.load_reg16_from_reg16(PC, WZ);
        }

        /// special, pchl
        pub fn pchl(&mut self) {
            use I8080RegisterCode16::{HL, PC};
            self.load_reg16_from_reg16(PC, HL);
        }

        pub fn push_reg16(&mut self, code: I8080RegisterCode16) {
            use I8080RegisterCode16::SP;
            let [h, l] = code.split();
            self.reg16_decrement(SP);
            self.code_reg_mut(h).read_to_data();
            self.code_reg16_read_to_address(SP);
            self.store();

            self.reg16_decrement(SP);
            self.code_reg_mut(l).read_to_data();
            self.code_reg16_read_to_address(SP);
            self.store();
        }

        pub fn pop_reg16(&mut self, code: I8080RegisterCode16) {
            use I8080RegisterCode16::{PC, SP};
            let [h, l] = code.split();
            self.code_reg16_read_to_address(SP);
            self.fetch();
            self.code_reg_mut(h).load_from_data();
            self.reg16_increment(SP);

            self.code_reg16_read_to_address(SP);
            self.fetch();
            self.code_reg_mut(l).load_from_data();
            self.reg16_increment(SP);
        }

        /// special
        pub fn exchange_stack_top_with_hl(&mut self) {
            use I8080RegisterCode::{H, L, W, Z};
            use I8080RegisterCode16::{HL, SP, WZ};
            let [h, l] = self.code_reg16_as_u16(HL).to_be_bytes();

            self.code_reg16_read_to_address(SP);
            self.fetch();
            self.code_reg_mut(Z).load_from_data();
            self.code_reg_mut(L).read_to_data();
            self.store();
            self.reg16_increment(SP);

            self.code_reg16_read_to_address(SP);
            self.fetch();
            self.code_reg_mut(W).load_from_data();
            self.code_reg_mut(H).read_to_data();
            self.store();
            self.reg16_decrement(SP);
            self.load_reg16_from_reg16(HL, WZ);
        }

        /// special
        pub fn sphl(&mut self) {
            use I8080RegisterCode16::{HL, SP};
            self.load_reg16_from_reg16(SP, HL)
        }

        /// special
        pub fn input(&mut self) {
            self.fetch_instruction();
            let acc = match self.data_bus.get() {
                _ => {
                    let mut buf = [0];
                    std::io::stdin().read_exact(&mut buf).unwrap();
                    buf[0]
                }
            };
            self.data_bus.set(acc);
            self.acc_reg().load_from_data()
        }

        /// special
        pub fn output(&mut self) {
            use I8080RegisterCode::Acc;
            self.fetch_instruction();
            self.acc_reg().read_to_data();
            println!("{}", self.data_bus.get() as char);
        }

        /// special
        pub fn enable_interrupt(&mut self) {}

        /// special
        pub fn disable_interrupt(&mut self) {}

        /// special
        pub fn halt(&mut self) {
            self.halted = true;
        }

        /// special
        pub fn no_op(&mut self) {}

        fn reg16_code_from_bits(bits: u8) -> I8080RegisterCode16 {
            use I8080RegisterCode16::*;
            match bits {
                0 => BC,
                1 => DE,
                2 => HL,
                3 => SP,
                _ => unreachable!(),
            }
        }

        fn reg_code_from_bits(bits: u8) -> I8080RegisterCode {
            use I8080RegisterCode::*;
            match bits {
                0 => B,
                1 => C,
                2 => D,
                3 => E,
                4 => H,
                5 => L,
                7 => Acc,
                _ => unreachable!(),
            }
        }

        fn condition_code_from_bits(bits: u8) -> I8080JumpCondition {
            use I8080JumpCondition::*;
            match bits {
                0 => OnNonZero,
                1 => OnZero,
                2 => OnNonCarry,
                3 => OnCarry,
                4 => OnParityOdd,
                5 => OnParityEven,
                6 => OnPlus,
                7 => OnMinus,
                _ => unreachable!(),
            }
        }

        pub fn execute(&mut self) {
            use I8080AluCode::*;
            use I8080JumpCondition::*;
            use I8080RegisterCode::*;
            use I8080RegisterCode16::*;
            self.fetch_instruction();
            self.code_reg_mut(Inst).load_from_data();
            let (op, dst, src) = {
                let inst = self.code_reg_mut(Inst).reg.read();
                ((inst >> 6), (inst >> 3) & 0x7, inst & 0x7)
            };
            match op {
                0 => match (dst, src) {
                    (0, 0) => self.no_op(),
                    (_, 0) => self.no_op(), // <= unspecified
                    (0, 7) => self.alu_with_reg(RotateLeft, Acc),
                    (1, 7) => self.alu_with_reg(RotateRight, Acc),
                    (2, 7) => self.alu_with_reg(RotateLeftThroughCarry, Acc),
                    (3, 7) => self.alu_with_reg(RotateRightThroughCarry, Acc),
                    (4, 2) => self.store_reg16_direct(HL),
                    (4, 7) => self.alu_with_reg(DecimalAdjust, Acc),
                    (5, 2) => self.move_reg16_direct(HL),
                    (5, 7) => self.alu_with_reg(ComplementAcc, Acc),
                    (6, 2) => self.store_reg_direct(Acc),
                    (6, 4) => self.alu_with_mem_to_mem(Increment),
                    (6, 5) => self.alu_with_mem_to_mem(Decrement),
                    (6, 6) => self.store_hl_immediate(),
                    (6, 7) => self.flag_complement(StatusFlag::Carry),
                    (7, 2) => self.move_reg_direct(Acc),
                    (7, 7) => self.flag_set(StatusFlag::Carry),
                    (dst, 1) if dst % 2 == 0 => {
                        self.move_reg16_immediate(Self::reg16_code_from_bits(dst / 2))
                    }
                    (dst, 2) if dst % 2 == 0 => {
                        self.move_indirect(Acc, Self::reg16_code_from_bits(dst / 2))
                    }
                    (dst, 2) if dst % 2 == 1 => {
                        self.move_indirect(Acc, Self::reg16_code_from_bits(dst / 2))
                    }
                    (dst, 3) if dst % 2 == 0 => {
                        self.reg16_increment(Self::reg16_code_from_bits(dst / 2));
                    }
                    (dst, 3) if dst % 2 == 1 => {
                        self.reg16_increment(Self::reg16_code_from_bits(dst / 2));
                    }
                    (dst, 4) => self.alu_with_reg_to_reg(Increment, Self::reg_code_from_bits(dst)),
                    (dst, 5) => self.alu_with_reg_to_reg(Decrement, Self::reg_code_from_bits(dst)),
                    (dst, 6) => self.move_reg_immediate(Self::reg_code_from_bits(dst)),
                    (rhs, 1) if rhs % 2 == 1 => {
                        let hl = self.code_reg16_as_u16(HL);
                        let rp = self.code_reg16_as_u16(Self::reg16_code_from_bits(rhs / 2));
                        let (res, carry) = hl.overflowing_add(rp);
                        if carry {
                            self.code_reg_mut(Flag)
                                .reg
                                .masked(Self::flag_decode(StatusFlag::Carry))
                                .load(!0)
                        } else {
                            self.code_reg_mut(Flag)
                                .reg
                                .masked(Self::flag_decode(StatusFlag::Carry))
                                .load(0)
                        }
                        HL.split()
                            .into_iter()
                            .zip(res.to_be_bytes())
                            .for_each(|(c, x)| self.code_reg_mut(c).reg.load(x));
                    }
                    _ => self.no_op(),
                },
                1 => match (dst, src) {
                    (6, 6) => self.halt(),
                    (dst, 6) => self.move_hl_mem_to_reg(Self::reg_code_from_bits(dst)),
                    (6, src) => self.store_reg_to_hl_mem(Self::reg_code_from_bits(src)),
                    (dst, src) => self.move_reg_to_reg(
                        Self::reg_code_from_bits(dst),
                        Self::reg_code_from_bits(src),
                    ),
                },
                2 => match (dst, src) {
                    (0, 6) => self.alu_with_mem(Add),
                    (0, src) => self.alu_with_reg(Add, Self::reg_code_from_bits(src)),
                    (1, 6) => self.alu_with_mem(AddCarried),
                    (1, src) => self.alu_with_reg(AddCarried, Self::reg_code_from_bits(src)),
                    (2, 6) => self.alu_with_mem(Sub),
                    (2, src) => self.alu_with_reg(Sub, Self::reg_code_from_bits(src)),
                    (3, 6) => self.alu_with_mem(SubBorrowed),
                    (3, src) => self.alu_with_reg(SubBorrowed, Self::reg_code_from_bits(src)),
                    (4, 6) => self.alu_with_mem(LogicAnd),
                    (4, src) => self.alu_with_reg(LogicAnd, Self::reg_code_from_bits(src)),
                    (5, 6) => self.alu_with_mem(LogicXor),
                    (5, src) => self.alu_with_reg(LogicXor, Self::reg_code_from_bits(src)),
                    (6, 6) => self.alu_with_mem(LogicOr),
                    (6, src) => self.alu_with_reg(LogicOr, Self::reg_code_from_bits(src)),
                    (7, 6) => self.cmp_with_mem(),
                    (7, src) => self.cmp_with_reg(Self::reg_code_from_bits(src)),
                    _ => unreachable!(),
                },
                3 => match (dst, src) {
                    (1, 1) => self.ret(Anytime),
                    (2, 1) => self.ret(Anytime), // <= unspecified
                    (4, 1) => self.pop_reg16(PSW),
                    (5, 1) => self.pchl(),
                    (7, 1) => self.sphl(),
                    (0, 3) => self.jump_immediate(Anytime),
                    (1, 3) => self.jump_immediate(Anytime), // <= unspecified
                    (2, 3) => self.output(),
                    (3, 3) => self.input(),
                    (4, 3) => self.exchange_stack_top_with_hl(),
                    (5, 3) => self.exchange16(HL, DE),
                    (6, 3) => self.disable_interrupt(),
                    (7, 3) => self.enable_interrupt(),
                    (1, 5) => self.call_immediate(Anytime),
                    (3, 5) => self.call_immediate(Anytime), // <= unspecified
                    (4, 5) => self.push_reg16(PSW),
                    (5, 5) => self.call_immediate(Anytime), // <= unspecified
                    (7, 5) => self.call_immediate(Anytime), // <= unspecified
                    (0, 6) => self.alu_with_immediate(Add),
                    (1, 6) => self.alu_with_immediate(AddCarried),
                    (2, 6) => self.alu_with_immediate(Sub),
                    (3, 6) => self.alu_with_immediate(SubBorrowed),
                    (4, 6) => self.alu_with_immediate(LogicAnd),
                    (5, 6) => self.alu_with_immediate(LogicXor),
                    (6, 6) => self.alu_with_immediate(LogicOr),
                    (7, 6) => self.cmp_with_immediate(),
                    (cond, 0) => self.ret(Self::condition_code_from_bits(cond)),
                    (cond, 2) => self.jump_immediate(Self::condition_code_from_bits(cond)),
                    (cond, 4) => self.call_immediate(Self::condition_code_from_bits(cond)),
                    (dst, 1) if dst % 2 == 0 => self.pop_reg16(Self::reg16_code_from_bits(dst / 2)),
                    (dst, 5) if dst % 2 == 0 => {
                        self.push_reg16(Self::reg16_code_from_bits(dst / 2))
                    }
                    (n, 7) => self.restart(n),
                    _ => self.no_op(),
                },
                _ => unreachable!(),
            }
        }

        pub fn run(&mut self) {
            self.halted = false;
            while !self.halted {
                self.execute();
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
        // load (0x0302) == 2
        c.move_reg_direct(Acc);
        assert_eq!(c.regs.get(&Acc).unwrap().reg.read(), 2);
        // loadx (0x0504) == 0x0504
        c.move_reg16_direct(HL);
        assert_eq!(c.code_reg16_as_u16(HL), 0x0504);
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

    #[test]
    fn run() {
        use I8080RegisterCode16::PC;
        let mut memory = RamB8A16::default();
        memory.flash(&(0..=255u8).cycle().take(65535).collect::<Vec<_>>(), 0);
        let mut c = I8080Console {
            memory,
            ..Default::default()
        };
        c.run();
        println!("{:?}", c);
        println!("{}", c.code_reg16_as_u16(PC));
    }
}
