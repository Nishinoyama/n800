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
    fn dis_scrambled(data: D) -> Self;
}

pub trait ALUFlag: Sized + Copy {}
