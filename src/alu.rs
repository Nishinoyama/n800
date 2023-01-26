use enumset::{EnumSet, EnumSetType};

pub trait ALU {
    type Flag: Flag;
    type Data;
    fn op(&self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>);
}

pub trait Flag: Sized + Copy + EnumSetType {}

#[derive(Debug, EnumSetType)]
pub enum StatusFlag {
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

pub mod bit8 {
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
        (res, status)
    }

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct Adder {
        neg: bool,
        cin: bool,
    }

    impl Adder {
        pub fn adder() -> Self {
            Self::default()
        }
        pub fn subber() -> Self {
            Self {
                neg: true,
                cin: true,
            }
        }
        pub fn carried_adder() -> Self {
            Self {
                neg: false,
                cin: true,
            }
        }
        pub fn borrowed_subber() -> Self {
            Self {
                neg: true,
                cin: false,
            }
        }
    }

    impl ALU for Adder {
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
    pub struct LogicalAnder {}

    impl ALU for LogicalAnder {
        type Flag = StatusFlag;
        type Data = u8;

        fn op(&self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>) {
            (lhs & rhs, StatusFlag::AuxiliaryCarry.into())
        }
    }

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct DecimalAdjuster {
        carry: bool,
        auxiliary: bool,
    }

    impl DecimalAdjuster {
        pub fn from_status(status: EnumSet<StatusFlag>) -> Self {
            Self {
                carry: status.contains(StatusFlag::Carry),
                auxiliary: status.contains(StatusFlag::AuxiliaryCarry),
            }
        }
    }

    impl ALU for DecimalAdjuster {
        type Flag = StatusFlag;
        type Data = u8;

        fn op(&self, _lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>) {
            let mut lsb = rhs & 0xf;
            if self.auxiliary || lsb >= 10 {
                lsb += 6;
            }
            let mut msb = rhs >> 4;
            if lsb >= 0x10 {
                msb += 1;
            }
            if self.carry || msb >= 10 {
                msb += 6;
            }
            let res = (msb << 4) | (lsb & 0xf);
            let mut status = StatusFlag::set_by_result(res);
            if lsb >= 0x10 {
                // unspecified
                status |= StatusFlag::AuxiliaryCarry;
            }
            if msb >= 0x10 || self.carry {
                status |= StatusFlag::Carry;
            }
            (res, status)
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn alu() {
            use StatusFlag::*;
            let adder = Adder::adder();
            assert_eq!(adder.op(10, 3), (13, EnumSet::empty()));
            assert_eq!(adder.op(103, 191), (38, Carry | AuxiliaryCarry));
            assert_eq!(
                adder.op(1, 255),
                (0, Carry | Zero | Parity | AuxiliaryCarry)
            );
            assert_eq!(adder.op(1, 254), (255, Parity | Sign));
            assert_eq!(adder.op(0x19, 0x28), (0x41, AuxiliaryCarry | Parity));
            let c_adder = Adder::carried_adder();
            assert_eq!(
                c_adder.op(0, 255),
                (0, Carry | Zero | Parity | AuxiliaryCarry)
            );
            assert_eq!(c_adder.op(6, 9), (16, AuxiliaryCarry.into()));
            let subber = Adder::subber();
            assert_eq!(subber.op(18, 3), (15, Parity | AuxiliaryCarry));
            assert_eq!(subber.op(16, 19), (253, Sign | Carry | AuxiliaryCarry));
            assert_eq!(subber.op(9, 9), (0, Zero | Parity));
            let b_subber = Adder::borrowed_subber();
            assert_eq!(b_subber.op(10, 3), (6, Parity.into()));
            let alus: Vec<Box<dyn ALU<Flag = StatusFlag, Data = u8>>> = vec![
                (Box::new(Adder::adder())),
                (Box::new(Adder::subber())),
                (Box::new(LogicalAnder {})),
            ];
            println!(
                "{:?}",
                alus.iter().map(|alu| alu.op(31, 41)).collect::<Vec<_>>()
            );
        }

        #[test]
        fn daa() {
            for lhs in 0..=255 {
                if let Ok(ld) = format!("{lhs:x}").parse::<u8>() {
                    for rhs in 0..=255 {
                        if let Ok(rd) = format!("{rhs:x}").parse::<u8>() {
                            let (res, status) = Adder::adder().op(lhs, rhs);
                            let (res, status) = DecimalAdjuster::from_status(status).op(0, res);
                            if let Ok(res) = format!("{res:x}").parse::<u8>() {
                                assert_eq!(res, (ld + rd) % 100);
                                assert_eq!(status.contains(StatusFlag::Carry), (ld + rd) >= 100)
                            } else {
                                panic!("decimal adjust fail")
                            }
                        }
                    }
                }
            }
        }
    }
}
