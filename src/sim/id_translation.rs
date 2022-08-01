use rand_distr::num_traits::real::Real;

use crate::settings::{RealRowMapping, RowMapping};

pub type ChannelID = usize;
pub type ChipID = (ChannelID, usize);

pub type BankID = (ChipID, usize);
pub type PeID = (BankID, usize);

pub fn channel_id_from_chip_id(chip_id: &ChipID) -> &ChannelID {
    &chip_id.0
}

pub fn chip_id_from_bank_id(bank_id: &BankID) -> &ChipID {
    &bank_id.0
}

pub fn channel_id_from_bank_id(bank_id: &BankID) -> &ChannelID {
    channel_id_from_chip_id(chip_id_from_bank_id(bank_id))
}

pub fn bank_id_from_pe_id(pe_id: &PeID) -> &BankID {
    &pe_id.0
}

pub fn chip_id_from_pe_id(pe_id: &PeID) -> &ChipID {
    chip_id_from_bank_id(bank_id_from_pe_id(pe_id))
}

pub fn channel_id_from_pe_id(pe_id: &PeID) -> &ChannelID {
    channel_id_from_chip_id(chip_id_from_pe_id(pe_id))
}

/// todo! use bit operations!

// pub fn get_bank_id_from_row_id(
//     row_id: usize,
//     channels: usize,
//     chips: usize,
//     banks: usize,
//     num_rows: usize,
//     row_mapping: &RealRowMapping,
// ) -> BankID {
//     let flat_bank_id =
//         crate::pim::get_bank_id_from_row_id(row_id, channels, chips, banks, num_rows, row_mapping);
//     let banks_per_channel = chips * banks;
//     let channel_id = flat_bank_id / banks_per_channel;
//     let flat_bankd_id_in_chip = flat_bank_id - channel_id * banks_per_channel;
//     let banks_per_chip = banks;
//     let chip_id = flat_bankd_id_in_chip / banks_per_chip;
//     let bank_id = flat_bankd_id_in_chip - chip_id * banks_per_chip;
//     ((channel_id, chip_id), bank_id)
// }

#[cfg(test)]
mod test {
    use crate::settings::{RealRowMapping, RowMapping};

    // #[test]
    // fn test_id_trans() {
    //     let total_rows = 32;
    //     let ret = (0..total_rows)
    //         .map(|x| get_bank_id_from_row_id(x, 2, 4, 4, total_rows, &RealRowMapping::Chunk))
    //         .collect::<Vec<_>>();

    //     let correct = vec![
    //         ((0, 0), 0),
    //         ((0, 0), 1),
    //         ((0, 0), 2),
    //         ((0, 0), 3),
    //         ((0, 1), 0),
    //         ((0, 1), 1),
    //         ((0, 1), 2),
    //         ((0, 1), 3),
    //         ((0, 2), 0),
    //         ((0, 2), 1),
    //         ((0, 2), 2),
    //         ((0, 2), 3),
    //         ((0, 3), 0),
    //         ((0, 3), 1),
    //         ((0, 3), 2),
    //         ((0, 3), 3),
    //         ((1, 0), 0),
    //         ((1, 0), 1),
    //         ((1, 0), 2),
    //         ((1, 0), 3),
    //         ((1, 1), 0),
    //         ((1, 1), 1),
    //         ((1, 1), 2),
    //         ((1, 1), 3),
    //         ((1, 2), 0),
    //         ((1, 2), 1),
    //         ((1, 2), 2),
    //         ((1, 2), 3),
    //         ((1, 3), 0),
    //         ((1, 3), 1),
    //         ((1, 3), 2),
    //         ((1, 3), 3),
    //     ];
    //     assert_eq!(ret, correct);
    // }

    // #[test]
    // fn test_id_trans2() {
    //     let total_rows = 64;
    //     let ret = (0..total_rows)
    //         .map(|x| get_bank_id_from_row_id(x, 2, 4, 4, total_rows, &RealRowMapping::Chunk))
    //         .collect::<Vec<_>>();

    //     let correct = vec![
    //         ((0, 0), 0),
    //         ((0, 0), 0),
    //         ((0, 0), 1),
    //         ((0, 0), 1),
    //         ((0, 0), 2),
    //         ((0, 0), 2),
    //         ((0, 0), 3),
    //         ((0, 0), 3),
    //         ((0, 1), 0),
    //         ((0, 1), 0),
    //         ((0, 1), 1),
    //         ((0, 1), 1),
    //         ((0, 1), 2),
    //         ((0, 1), 2),
    //         ((0, 1), 3),
    //         ((0, 1), 3),
    //         ((0, 2), 0),
    //         ((0, 2), 0),
    //         ((0, 2), 1),
    //         ((0, 2), 1),
    //         ((0, 2), 2),
    //         ((0, 2), 2),
    //         ((0, 2), 3),
    //         ((0, 2), 3),
    //         ((0, 3), 0),
    //         ((0, 3), 0),
    //         ((0, 3), 1),
    //         ((0, 3), 1),
    //         ((0, 3), 2),
    //         ((0, 3), 2),
    //         ((0, 3), 3),
    //         ((0, 3), 3),
    //         ((1, 0), 0),
    //         ((1, 0), 0),
    //         ((1, 0), 1),
    //         ((1, 0), 1),
    //         ((1, 0), 2),
    //         ((1, 0), 2),
    //         ((1, 0), 3),
    //         ((1, 0), 3),
    //         ((1, 1), 0),
    //         ((1, 1), 0),
    //         ((1, 1), 1),
    //         ((1, 1), 1),
    //         ((1, 1), 2),
    //         ((1, 1), 2),
    //         ((1, 1), 3),
    //         ((1, 1), 3),
    //         ((1, 2), 0),
    //         ((1, 2), 0),
    //         ((1, 2), 1),
    //         ((1, 2), 1),
    //         ((1, 2), 2),
    //         ((1, 2), 2),
    //         ((1, 2), 3),
    //         ((1, 2), 3),
    //         ((1, 3), 0),
    //         ((1, 3), 0),
    //         ((1, 3), 1),
    //         ((1, 3), 1),
    //         ((1, 3), 2),
    //         ((1, 3), 2),
    //         ((1, 3), 3),
    //         ((1, 3), 3),
    //     ];
    //     assert_eq!(ret, correct);
}
