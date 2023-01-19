pub trait RegisterLoad: RegisterRead {
    type Register;
    fn load(self, bits: Self::Size) -> Self::Register;
}

pub trait RegisterRead {
    type Size;
    fn read(&self) -> Self::Size;
}

pub mod bit8 {
    use crate::register::{RegisterLoad, RegisterRead};

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
    pub struct Register8 {
        bits: u8,
    }

    impl Register8 {
        pub fn new(bits: u8) -> Self {
            Self { bits }
        }
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
    pub struct Register8Pair {
        h: Register8,
        l: Register8,
    }

    impl Register8Pair {
        pub fn new(h: Register8, l: Register8) -> Self {
            Self { h, l }
        }
        pub fn from_tuple((h, l): (Register8, Register8)) -> Self {
            Self { h, l }
        }
        pub fn split(self) -> (Register8, Register8) {
            (self.h, self.l)
        }
    }

    impl RegisterRead for Register8 {
        type Size = u8;

        fn read(&self) -> Self::Size {
            self.bits
        }
    }

    impl RegisterLoad for Register8 {
        type Register = Self;

        fn load(mut self, bits: Self::Size) -> Self::Register {
            self.bits = bits;
            self
        }
    }

    impl RegisterRead for Register8Pair {
        type Size = u16;

        fn read(&self) -> Self::Size {
            u16::from_be_bytes([self.h.read(), self.l.read()])
        }
    }

    impl RegisterLoad for Register8Pair {
        type Register = (Register8, Register8);

        fn load(self, bits: Self::Size) -> Self::Register {
            let [h, l] = bits.to_be_bytes();
            (self.h.load(h), self.l.load(l))
        }
    }

    pub struct MaskedRegister8 {
        reg: Register8,
        mask: u8,
    }

    impl MaskedRegister8 {
        pub fn new(reg: Register8, mask: u8) -> Self {
            Self { reg, mask }
        }
    }

    impl RegisterRead for MaskedRegister8 {
        type Size = u8;

        fn read(&self) -> Self::Size {
            self.reg.read() & self.mask
        }
    }

    impl RegisterLoad for MaskedRegister8 {
        type Register = Register8;

        fn load(self, bits: Self::Size) -> Self::Register {
            let bits = (bits & self.mask) | (self.reg.read() & !self.mask);
            self.reg.load(bits)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reg8_16() {
            let reg = Register8::new(3);
            let reg = reg.load(13);
            assert_eq!(reg.read(), 13);
            let reg16 = Register8Pair::new(Register8::new(10), Register8::new(32));
            assert_eq!(reg16.read(), 2592);
            let reg16 = Register8Pair::from_tuple(reg16.load(3141));
            assert_eq!(reg16.read(), 3141);
            let (h, l) = reg16.split();
            assert_eq!(h.read(), 12);
            assert_eq!(l.read(), 69);
        }

        #[test]
        fn reg8_flag_reg() {
            let reg = Register8::default();
            let reg = MaskedRegister8::new(reg, 0x33).load(0x55);
            assert_eq!(reg.read(), 0x11);
            let reg = MaskedRegister8::new(reg, 0x66).load(0x44);
            assert_eq!(reg.read(), 0x55);
        }
    }
}
