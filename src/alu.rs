use crate::bits::BitwiseOps;
use strum::IntoEnumIterator;

pub trait ALU {
    type Data;
    /// flag scrambler is containing!
    fn op(self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, Self::Data);
}

pub trait FlagSet<F: ALUFlag>: Sized + BitwiseOps {
    fn set(self, flag: F) -> Self;
    fn reset(self, flag: F) -> Self;
    fn get(&self, flag: F) -> bool;
    // FIXME: implement me as an iterator!
    fn into_flags(self) -> Vec<F>
    where
        F: IntoEnumIterator,
    {
        F::iter().filter(|&f| self.get(f)).collect()
    }
    fn from_flags<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = F>,
        Self: FromIterator<F>,
    {
        iter.into_iter().collect()
    }
}

pub trait ALUFlag: Sized + Copy {}

pub mod bit8 {
    use crate::alu::{ALUFlag, FlagSet};
    use crate::bits::BitwiseOps;
    use std::marker::PhantomData;
    use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
    use strum::IntoEnumIterator;

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct FlagSetU8<F> {
        bits: u8,
        flag: PhantomData<F>,
    }

    impl<F> Default for FlagSetU8<F> {
        fn default() -> Self {
            Self {
                bits: u8::MIN,
                flag: Default::default(),
            }
        }
    }

    impl<F: ALUFlag + Eq> BitwiseOps for FlagSetU8<F> {}

    impl<F> BitAnd for FlagSetU8<F> {
        type Output = Self;

        fn bitand(mut self, rhs: Self) -> Self::Output {
            self.bits &= rhs.bits;
            self
        }
    }

    impl<F> BitOr for FlagSetU8<F> {
        type Output = Self;

        fn bitor(mut self, rhs: Self) -> Self::Output {
            self.bits |= rhs.bits;
            self
        }
    }

    impl<F> BitAndAssign for FlagSetU8<F> {
        fn bitand_assign(&mut self, rhs: Self) {
            self.bits &= rhs.bits;
        }
    }

    impl<F> BitOrAssign for FlagSetU8<F> {
        fn bitor_assign(&mut self, rhs: Self) {
            self.bits |= rhs.bits;
        }
    }

    impl<F> Not for FlagSetU8<F> {
        type Output = Self;

        fn not(mut self) -> Self::Output {
            self.bits = !self.bits;
            self
        }
    }

    impl<F: ALUFlag + IntoEnumIterator + Eq> FlagSet<F> for FlagSetU8<F> {
        fn set(mut self, flag: F) -> Self {
            self.bits |= 1 << F::iter().position(|f| f == flag).unwrap();
            self
        }
        fn reset(mut self, flag: F) -> Self {
            self.bits &= !(1 << F::iter().position(|f| f == flag).unwrap());
            self
        }
        fn get(&self, flag: F) -> bool {
            self.bits & (1 << F::iter().position(|f| f == flag).unwrap()) > 0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::alu::bit8::FlagSetU8;
    use crate::alu::{ALUFlag, FlagSet, ALU};
    use strum::IntoEnumIterator;
    use strum_macros::EnumIter;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
    enum TestFlag {
        Ovf,
        Zero,
    }

    impl TestFlag {
        fn into_u8(self) -> u8 {
            match self {
                TestFlag::Ovf => 8,
                TestFlag::Zero => 1,
            }
        }
    }

    impl ALUFlag for TestFlag {}

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
        type Data = u8;

        fn op(self, lhs: Self::Data, mut rhs: Self::Data) -> (Self::Data, Self::Data) {
            if self.neg {
                rhs = !rhs;
            }
            let (res, ovf) = if self.cin {
                let (res, ovf0) = rhs.overflowing_add(lhs);
                let (res, ovf1) = res.overflowing_add(1);
                (res, ovf0 | ovf1)
            } else {
                rhs.overflowing_add(lhs)
            };
            let mut status = 2;
            if self.neg ^ ovf {
                status += TestFlag::Ovf.into_u8();
            };
            if res == 0 {
                status += TestFlag::Zero.into_u8();
            };
            (res, status)
        }
    }

    fn scramble(flags: FlagSetU8<TestFlag>) -> u8 {
        flags
            .into_flags()
            .into_iter()
            .fold(2, |acc, f| acc | f.into_u8())
    }

    #[test]
    fn iter() {
        use TestFlag::*;
        assert_eq!(TestFlag::iter().collect::<Vec<_>>(), vec![Ovf, Zero]);
        let fs = FlagSetU8::default().set(Ovf).set(Zero).set(Ovf);
        assert_eq!(scramble(fs), 11);
        let ft = fs.reset(Zero);
        assert_eq!(scramble(ft), 10);
        assert_eq!(scramble(fs | ft), 11);
        assert_eq!(scramble(fs & ft), 10);
        assert_eq!(scramble(!fs & ft), 2);
    }

    #[test]
    fn alu() {
        assert_eq!(TestAdder::adder().op(10, 3), (13, 2));
        assert_eq!(TestAdder::adder().op(103, 191), (38, 10));
        assert_eq!(TestAdder::adder().op(1, 255), (0, 11));
        assert_eq!(TestAdder::carried_adder().op(0, 255), (0, 11));
        assert_eq!(TestAdder::carried_adder().op(6, 9), (16, 2));
        assert_eq!(TestAdder::subber().op(10, 3), (7, 2));
        assert_eq!(TestAdder::subber().op(10, 13), (253, 10));
        assert_eq!(TestAdder::subber().op(13, 13), (0, 3));
        assert_eq!(TestAdder::borrowed_subber().op(10, 3), (6, 2));
    }
}
