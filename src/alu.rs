use strum::IntoEnumIterator;

pub trait FlagSet<F: ALUFlag>: Sized {
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

#[cfg(test)]
mod tests {
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
    impl FlagSet<TestFlag> for u8 {
        fn set(self, flag: TestFlag) -> Self {
            self | (1 << flag.nth())
        }
        fn reset(self, flag: TestFlag) -> Self {
            self & !(1 << flag.nth())
        }
        fn get(&self, flag: TestFlag) -> bool {
            self & (1 << flag.nth()) > 0
        }
    }

    impl FlagSetScrambled<TestFlag, u8> for u8 {
        fn scrambled(self) -> u8 {
            self.into_flags()
                .into_iter()
                .fold(2, |acc, f| acc | f.into_u8())
        }
    }

    #[test]
    fn iter() {
        use TestFlag::*;
        assert_eq!(TestFlag::iter().collect::<Vec<_>>(), vec![Ovf, Zero]);
        let fs = 0u8.set(Ovf).set(Zero).set(Ovf);
        assert_eq!(fs.scrambled(), 11);
        let fs = fs.reset(Zero);
        assert_eq!(fs.scrambled(), 10);
    }
}
