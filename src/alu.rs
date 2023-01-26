use enumset::{EnumSet, EnumSetType};

pub trait ALU {
    type Flag: Flag;
    type Data;
    fn op(&self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>);
}

pub trait Flag: Sized + Copy + EnumSetType {}

#[derive(Debug, EnumSetType)]
enum StatusFlag {
    /// result is zero.
    Zero,
    /// result is signed.
    Sign,
    /// result parity sum is even.
    Parity,
    /// result cause carrying
    Carry,
    /// result is too large to fit a word
    Overflow,
    /// result on bcd overflowing
    AuxiliaryCarry,
}

impl StatusFlag {
    pub fn set_by_result(result: u8) -> EnumSet<StatusFlag> {
        let mut flag_set = EnumSet::new();
        if result == 0 {
            flag_set |= Self::Zero;
        }
        if result >= 0x80 {
            flag_set |= Self::Sign;
        }
        if result.count_ones() % 2 == 0 {
            flag_set |= Self::Parity;
        }
        flag_set
    }
}

impl Flag for StatusFlag {}

#[cfg(test)]
mod tests {
    use crate::alu::{StatusFlag, ALU};
    use enumset::EnumSet;

    fn primary_adder(cin: bool, lhs: u8, rhs: u8) -> (u8, EnumSet<StatusFlag>) {
        let auxiliary_carry = if cin {
            lhs % 0x10 + rhs % 0x10 + 1 >= 0x10
        } else {
            lhs % 0x10 + rhs % 0x10 >= 0x10
        };
        let (rhs, carry_ovf) = if cin {
            rhs.overflowing_add(1)
        } else {
            (rhs, false)
        };
        let (res, ovf) = lhs.overflowing_add(rhs);
        let mut status = StatusFlag::set_by_result(res);
        if carry_ovf | ovf {
            status |= StatusFlag::Carry;
        }
        if auxiliary_carry {
            status |= StatusFlag::AuxiliaryCarry;
        }
        println!("{:x} + {:x} = {:x} ({:?})", lhs, rhs, res, status);
        (res, status)
    }

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    struct TestAdder {
        neg: bool,
        cin: bool,
    }

    impl TestAdder {
        fn adder() -> Self {
            Self::default()
        }
        fn subber() -> Self {
            Self {
                neg: true,
                cin: true,
            }
        }
        fn carried_adder() -> Self {
            Self {
                neg: false,
                cin: true,
            }
        }
        fn borrowed_subber() -> Self {
            Self {
                neg: true,
                cin: false,
            }
        }
    }

    impl ALU for TestAdder {
        type Flag = StatusFlag;
        type Data = u8;

        fn op(&self, lhs: Self::Data, mut rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>) {
            if self.neg {
                rhs = !rhs;
            }
            let (res, mut status) = primary_adder(self.cin, lhs, rhs);
            if self.neg {
                status ^= StatusFlag::Carry;
                status ^= StatusFlag::AuxiliaryCarry;
            }
            (res, status)
        }
    }

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    struct TestBitwiseAnd {}

    impl ALU for TestBitwiseAnd {
        type Flag = StatusFlag;
        type Data = u8;

        fn op(&self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>) {
            (lhs & rhs, StatusFlag::AuxiliaryCarry.into())
        }
    }

    #[test]
    fn alu() {
        use StatusFlag::*;
        let adder = TestAdder::adder();
        assert_eq!(adder.op(10, 3), (13, EnumSet::empty()));
        assert_eq!(adder.op(103, 191), (38, Carry | AuxiliaryCarry));
        assert_eq!(
            adder.op(1, 255),
            (0, Carry | Zero | Parity | AuxiliaryCarry)
        );
        assert_eq!(adder.op(1, 254), (255, Parity | Sign));
        assert_eq!(adder.op(0x19, 0x28), (0x41, AuxiliaryCarry | Parity));
        let c_adder = TestAdder::carried_adder();
        assert_eq!(
            c_adder.op(0, 255),
            (0, Carry | Zero | Parity | AuxiliaryCarry)
        );
        assert_eq!(c_adder.op(6, 9), (16, AuxiliaryCarry.into()));
        let subber = TestAdder::subber();
        assert_eq!(subber.op(18, 3), (15, Parity | AuxiliaryCarry));
        assert_eq!(subber.op(16, 19), (253, Sign | Carry | AuxiliaryCarry));
        assert_eq!(subber.op(9, 9), (0, Zero | Parity));
        let b_subber = TestAdder::borrowed_subber();
        assert_eq!(b_subber.op(10, 3), (6, Parity.into()));
        let alus: Vec<Box<dyn ALU<Flag = StatusFlag, Data = u8>>> = vec![
            (Box::new(TestAdder::adder())),
            (Box::new(TestAdder::subber())),
            (Box::new(TestBitwiseAnd {})),
        ];
        println!(
            "{:?}",
            alus.iter().map(|alu| alu.op(31, 41)).collect::<Vec<_>>()
        );
    }
}
