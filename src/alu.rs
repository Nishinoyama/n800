use enumset::{EnumSet, EnumSetType};

pub trait ALU {
    type Flag: StatusFlag;
    type Data;
    fn op(self, lhs: Self::Data, rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>);
}

pub trait StatusFlag: Sized + Copy + EnumSetType {}

#[cfg(test)]
mod tests {
    use crate::alu::{StatusFlag, ALU};
    use enumset::{EnumSet, EnumSetType};

    #[derive(Debug, EnumSetType)]
    enum TestFlag {
        Ovf,
        Zero,
    }

    impl StatusFlag for TestFlag {}

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
        type Flag = TestFlag;
        type Data = u8;

        fn op(self, lhs: Self::Data, mut rhs: Self::Data) -> (Self::Data, EnumSet<Self::Flag>) {
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
            let mut status = EnumSet::empty();
            if self.neg ^ ovf {
                status |= TestFlag::Ovf;
            };
            if res == 0 {
                status |= TestFlag::Zero;
            };
            (res, status)
        }
    }

    #[test]
    fn alu() {
        use TestFlag::*;
        assert_eq!(TestAdder::adder().op(10, 3), (13, EnumSet::empty()));
        assert_eq!(TestAdder::adder().op(103, 191), (38, Ovf.into()));
        assert_eq!(TestAdder::adder().op(1, 255), (0, Ovf | Zero));
        assert_eq!(TestAdder::carried_adder().op(0, 255), (0, Ovf | Zero));
        assert_eq!(TestAdder::carried_adder().op(6, 9), (16, EnumSet::empty()));
        assert_eq!(TestAdder::subber().op(10, 3), (7, EnumSet::empty()));
        assert_eq!(TestAdder::subber().op(10, 13), (253, Ovf.into()));
        assert_eq!(TestAdder::subber().op(13, 13), (0, Zero.into()));
        assert_eq!(TestAdder::borrowed_subber().op(10, 3), (6, EnumSet::empty()));
    }
}
