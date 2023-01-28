use crate::bits::BitsSignal;

pub trait Register: Sized {
    type Size: BitsSignal;
    fn read(&self) -> Self::Size;
    fn load(&mut self, bits: Self::Size);
    fn masked(&mut self, mask: Self::Size) -> MaskedRegister<Self> {
        MaskedRegister { reg: self, mask }
    }
}

pub struct MaskedRegister<'a, R: Register> {
    reg: &'a mut R,
    mask: <R as Register>::Size,
}

impl<'a, R: Register> MaskedRegister<'a, R> {
    pub fn unmasked(self) -> &'a mut R {
        self.reg
    }
}

impl<'a, R: Register> Register for MaskedRegister<'a, R> {
    type Size = <R as Register>::Size;

    fn read(&self) -> Self::Size {
        self.reg.read() & self.mask
    }

    fn load(&mut self, bits: Self::Size) {
        let bits = (bits & self.mask) | (self.reg.read() & !self.mask);
        self.reg.load(bits)
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
        pub fn increment(&mut self) {
            let hl = self.as_u16();
            let [h, l] = hl.wrapping_add(1).to_be_bytes();
            self.h.load(h);
            self.l.load(l);
        }
        pub fn decrement(&mut self) {
            let hl = self.as_u16();
            let [h, l] = hl.wrapping_sub(1).to_be_bytes();
            self.h.load(h);
            self.l.load(l);
        }
        pub fn as_u16(&self) -> u16 {
            u16::from_be_bytes([self.h.read(), self.l.read()])
        }
    }

    impl Register for Register8 {
        type Size = u8;

        #[must_use]
        fn read(&self) -> Self::Size {
            self.bits
        }
        fn load(&mut self, bits: Self::Size) {
            self.bits = bits
        }
    }

    impl Register for Register8Pair {
        type Size = u16;

        fn read(&self) -> Self::Size {
            u16::from_be_bytes([self.h.read(), self.l.read()])
        }
        fn load(&mut self, bits: Self::Size) {
            let [h, l] = bits.to_be_bytes();
            self.h.load(h);
            self.l.load(l);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reg8_16() {
            let mut reg = Register8::new(3);
            reg.load(13);
            assert_eq!(reg.read(), 13);
            let mut reg16 = Register8Pair::new(Register8::new(10), Register8::new(32));
            assert_eq!(reg16.read(), 2592);
            reg16.load(3141);
            let mut reg16 = Register8Pair::from_tuple(reg16.split());
            assert_eq!(reg16.read(), 3141);
            reg16.increment();
            assert_eq!(reg16.read(), 3142);
            reg16.decrement();
            assert_eq!(reg16.read(), 3141);
            let (h, l) = reg16.split();
            assert_eq!(h.read(), 12);
            assert_eq!(l.read(), 69);
        }

        #[test]
        fn reg8_flag_reg() {
            let mut reg = Register8::default();
            let mut reg = reg.masked(0x33);
            reg.load(0x55);
            let reg = reg.unmasked();
            assert_eq!(reg.read(), 0x11);
            let mut reg = reg.masked(0x66);
            reg.load(0x44);
            let reg = reg.unmasked();
            assert_eq!(reg.read(), 0x55);
        }
    }
}
