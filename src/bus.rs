pub trait DataBus {
    type Data;
    fn set_data(&mut self, data: Self::Data);
    fn get_data(&self) -> Self::Data;
}

pub trait AddressBus {
    type Address;
    fn set_address(&mut self, data: Self::Address);
    fn get_address(&self) -> Self::Address;
}

/// Be connected to data bus and can load data from it.
pub trait DataBusLoad {
    type DataBus: DataBus;
    fn load_from_data(&mut self);
}

/// Be connected to data bus and can write data to it.
pub trait DataBusRead {
    type DataBus: DataBus;
    fn read_to_data(&self);
}

/// Be connected to address bus and can load address from it.
pub trait AddressBusLoad {
    type AddressBus: AddressBus;
    fn load_address(&mut self);
}

/// Be connected to address bus and can write address to it.
pub trait AddressBusRead {
    type AddressBus: AddressBus;
    fn read_address(&self);
}
