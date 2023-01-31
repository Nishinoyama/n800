use crate::processor::{
    AddressingRegisterCode, DataRegisterCode, ProcAddressingRegisters, ProcDataRegisters,
    ProcMemory,
};

pub trait Instruction<P> {
    fn execute(&self, proc: &mut P);
}

#[derive(Debug, Copy, Clone)]
pub struct DataRegisterInstruction<C> {
    code: C,
    inst: DataRegisterInstructionType,
}

#[derive(Debug, Copy, Clone)]
pub enum DataRegisterInstructionType {
    Load,
    Read,
}

impl<P, C> Instruction<P> for DataRegisterInstruction<C>
where
    P: ProcDataRegisters<C>,
    C: DataRegisterCode,
{
    fn execute(&self, proc: &mut P) {
        match self.inst {
            DataRegisterInstructionType::Load => proc.data_reg_load(&self.code),
            DataRegisterInstructionType::Read => proc.data_reg_read(&self.code),
        }
    }
}

impl<C> DataRegisterInstruction<C> {
    pub fn load(code: C) -> Self {
        Self {
            code,
            inst: DataRegisterInstructionType::Load,
        }
    }
    pub fn read(code: C) -> Self {
        Self {
            code,
            inst: DataRegisterInstructionType::Read,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AddressingRegisterInstruction<C>(C);

impl<P, C> Instruction<P> for AddressingRegisterInstruction<C>
where
    P: ProcAddressingRegisters<C>,
    C: AddressingRegisterCode,
{
    fn execute(&self, proc: &mut P) {
        proc.addressing_reg_read(&self.0)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MemoryInstruction {
    Store,
    Fetch,
}

impl<P: ProcMemory> Instruction<P> for MemoryInstruction {
    fn execute(&self, proc: &mut P) {
        match self {
            MemoryInstruction::Store => proc.store(),
            MemoryInstruction::Fetch => proc.fetch(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::instruction::{
        AddressingRegisterInstruction, DataRegisterInstruction, Instruction, MemoryInstruction,
    };
    use crate::processor::i8080::I8080Console;

    #[test]
    fn i8080test() {
        use crate::processor::i8080::I8080RegisterCode::*;
        use crate::processor::i8080::I8080RegisterCode16::*;
        let mut p = I8080Console::default();
        let p = &mut p;
        p.flash(&[1, 2, 3, 4]);
        MemoryInstruction::Fetch.execute(p);
        DataRegisterInstruction::load(L).execute(p);
        AddressingRegisterInstruction(HL).execute(p);
        MemoryInstruction::Fetch.execute(p);
        DataRegisterInstruction::load(Acc).execute(p);
        println!("{:?}", p.code_reg_mut(Acc));
    }
}
