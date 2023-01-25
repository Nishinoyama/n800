use crate::bits::BitsSignal;

pub trait Register: Sized {
    type Size: BitsSignal;
    fn read(&self) -> Self::Size;
    fn load(self, bits: Self::Size) -> Self;
    fn masked(self, mask: Self::Size) -> MaskedRegister<Self> {
        MaskedRegister { reg: self, mask }
    }
}

pub struct MaskedRegister<R: Register> {
    reg: R,
    mask: <R as Register>::Size,
}

impl<R: Register> MaskedRegister<R> {
    pub fn unmasked(self) -> R {
        self.reg
    }
}

impl<R: Register> Register for MaskedRegister<R> {
    type Size = <R as Register>::Size;

    fn read(&self) -> Self::Size {
        self.reg.read() & self.mask
    }

    fn load(self, bits: Self::Size) -> Self {
        let bits = (bits & self.mask) | (self.reg.read() & !self.mask);
        Self {
            reg: self.reg.load(bits),
            ..self
        }
    }
}

pub mod bit8 {
    use crate::register::Register;

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
        pub fn increment(self) -> Self {
            let hl = u16::from_be_bytes([self.h.read(), self.l.read()]);
            let [h, l] = hl.wrapping_add(1).to_be_bytes();
            Self {
                h: Register8::new(h),
                l: Register8::new(l),
            }
        }
        pub fn decrement(self) -> Self {
            let hl = u16::from_be_bytes([self.h.read(), self.l.read()]);
            let [h, l] = hl.wrapping_sub(1).to_be_bytes();
            Self {
                h: Register8::new(h),
                l: Register8::new(l),
            }
        }
    }

    impl Register for Register8 {
        type Size = u8;

        fn read(&self) -> Self::Size {
            self.bits
        }
        fn load(mut self, bits: Self::Size) -> Self {
            self.bits = bits;
            self
        }
    }

    impl Register for Register8Pair {
        type Size = u16;

        fn read(&self) -> Self::Size {
            u16::from_be_bytes([self.h.read(), self.l.read()])
        }
        fn load(self, bits: Self::Size) -> Self {
            let [h, l] = bits.to_be_bytes();
            Self {
                h: Register8::new(h),
                l: Register8::new(l),
            }
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
            let reg16 = Register8Pair::from_tuple(reg16.load(3141).split());
            assert_eq!(reg16.read(), 3141);
            let reg16 = reg16.increment();
            assert_eq!(reg16.read(), 3142);
            let reg16 = reg16.decrement();
            assert_eq!(reg16.read(), 3141);
            let (h, l) = reg16.split();
            assert_eq!(h.read(), 12);
            assert_eq!(l.read(), 69);
        }

        #[test]
        fn reg8_flag_reg() {
            let reg = Register8::default().masked(0x33).load(0x55).unmasked();
            assert_eq!(reg.read(), 0x11);
            let reg = reg.masked(0x66).load(0x44).unmasked();
            assert_eq!(reg.read(), 0x55);
        }
    }
}
