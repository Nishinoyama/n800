use crate::bits::BitwiseOps;
use strum::IntoEnumIterator;

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

pub trait FlagSetScrambled<F: ALUFlag, D>: FlagSet<F> {
    fn scrambled(self) -> D;
}

pub trait ALUFlag: Sized + Copy {}

pub mod bit8 {
    use crate::alu::{ALUFlag, FlagSet};
    use crate::bits::BitwiseOps;
    use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
    use strum::IntoEnumIterator;

    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
    pub struct FlagSetU8 {
        bits: u8,
    }

    impl BitwiseOps for FlagSetU8 {
        const ALL_ONE: Self = Self { bits: u8::MAX };
        const ALL_ZERO: Self = Self { bits: u8::MIN };
    }

    impl BitAnd for FlagSetU8 {
        type Output = Self;

        fn bitand(mut self, rhs: Self) -> Self::Output {
            self.bits &= rhs.bits;
            self
        }
    }

    impl BitOr for FlagSetU8 {
        type Output = Self;

        fn bitor(mut self, rhs: Self) -> Self::Output {
            self.bits |= rhs.bits;
            self
        }
    }

    impl BitAndAssign for FlagSetU8 {
        fn bitand_assign(&mut self, rhs: Self) {
            self.bits &= rhs.bits;
        }
    }

    impl BitOrAssign for FlagSetU8 {
        fn bitor_assign(&mut self, rhs: Self) {
            self.bits |= rhs.bits;
        }
    }

    impl Not for FlagSetU8 {
        type Output = Self;

        fn not(mut self) -> Self::Output {
            self.bits = !self.bits;
            self
        }
    }

    impl<F: ALUFlag + IntoEnumIterator + Eq> FlagSet<F> for FlagSetU8 {
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
    use crate::alu::{ALUFlag, FlagSet, FlagSetScrambled};
    use strum::IntoEnumIterator;
    use strum_macros::EnumIter;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
    enum TestFlag {
        Ovf,
        Zero,
    }

    impl TestFlag {
        fn nth(self) -> usize {
            TestFlag::iter().position(|x| x == self).unwrap()
        }
        fn into_u8(self) -> u8 {
            match self {
                TestFlag::Ovf => 8,
                TestFlag::Zero => 1,
            }
        }
    }

    impl ALUFlag for TestFlag {}

    impl FlagSetScrambled<TestFlag, u8> for FlagSetU8 {
        fn scrambled(self) -> u8 {
            self.into_flags()
                .into_iter()
                .fold(2, |acc, f: TestFlag| acc | f.into_u8())
        }
    }

    #[test]
    fn iter() {
        use TestFlag::*;
        assert_eq!(TestFlag::iter().collect::<Vec<_>>(), vec![Ovf, Zero]);
        let fs = FlagSetU8::default().set(Ovf).set(Zero).set(Ovf);
        assert_eq!(fs.scrambled(), 11);
        let fs = fs.reset(Zero);
        assert_eq!(fs.scrambled(), 10);
    }
}
