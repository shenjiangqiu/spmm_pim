//! the non-pim module contains the NonPim trait

///the struct that run with out pim
pub trait NonPim {
    /// return the traffic read,real data read, and the cycle to read
    fn mem_read_cycle(&self) -> (usize, usize, u64);
    /// return the cycle to precess
    fn process_cycle(&self) -> u64;
}
