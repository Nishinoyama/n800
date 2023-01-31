pub trait DataRegisterCode {}
pub trait AddressingRegisterCode {}

pub trait ProcDataRegisters<C: DataRegisterCode> {
    fn data_reg_read(&mut self, code: &C);
    fn data_reg_load(&mut self, code: &C);
}

pub trait ProcAddressingRegisters<C: AddressingRegisterCode> {
    fn addressing_reg_read(&mut self, code: &C);
}
/// Processor that can fetch data from its memory and make it store data.
pub trait ProcMemory {
    fn store(&mut self);
    fn fetch(&mut self);
}

pub trait ProcInstructionFetch {
    fn fetch_instruction(&mut self);
}

pub trait ProcImmediateValue: ProcInstructionFetch {
    fn fetch_immediate_value(&mut self) {
        self.fetch_instruction();
    }
}

pub mod i8080;
