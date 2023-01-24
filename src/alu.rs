use enumset::EnumSetType;

pub trait ALU {
    type Data;
    /// flag scrambler is containing!
    fn op(self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, Self::Data);
}

pub trait ALUFlag: Sized + Copy + EnumSetType {}

#[cfg(test)]
mod tests {
    use crate::alu::{ALUFlag, ALU};
    use enumset::{EnumSet, EnumSetType};
    use std::ops::Sub;

    #[derive(Debug, EnumSetType)]
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

    fn scramble(flags: EnumSet<TestFlag>) -> u8 {
        flags.iter().fold(2, |acc, f| acc | f.into_u8())
    }

    #[test]
    fn iter() {
        use TestFlag::*;
        assert_eq!(
            EnumSet::<TestFlag>::all().into_iter().collect::<Vec<_>>(),
            vec![Ovf, Zero]
        );
        let fs = Ovf | Zero | Ovf;
        assert_eq!(scramble(fs), 11);
        let ft = fs.sub(Zero);
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
