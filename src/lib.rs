pub mod bits;

pub mod register;

pub mod alu;

pub mod memory;

pub mod processor;

pub mod instruction {
    pub trait Instruction<C> {
        fn execute(&self, cpu: &mut C);
    }
}

pub mod interact {
    use std::collections::HashMap;

    #[derive(Default)]
    struct C {
        d: u8,
        a: u8,
        b: u8,
        c: u8,
    }
    #[derive(Default)]
    struct M {
        data: HashMap<u8, u8>,
    }
}
