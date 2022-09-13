use std::path::Path;

use eyre::{Context, Result};
use serde::Serialize;
#[derive(Serialize, Debug)]
pub struct Results<'a> {
    pub all: Vec<SingleResult<'a>>,
}
#[derive(Serialize, Debug)]
pub struct SingleResult<'a> {
    pub file: &'a Path,
    pub c: usize,
    pub r: usize,
    pub block_size: usize,
    pub origin_nnz: usize,
    pub new_nnz: usize,
    pub new_element: usize,
    pub need_speed_up: f32,
    pub row_read: Vec<(usize, usize)>,
    pub bank_merge: Vec<usize>,
    pub bank_add: Vec<usize>,
    pub chip_merge: Vec<usize>,
    pub chip_add: Vec<usize>,
    pub channel_merge: Vec<usize>,
    pub channel_add: Vec<usize>,
    pub dimm_merge: usize,
    pub dimm_add: usize,
}

impl<'a> Results<'a> {
    pub fn save_to_file(&self, filename: &Path) -> Result<()> {
        // create dir first
        if let Some(parent) = filename.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(filename.with_extension("json"))
            .wrap_err("fail to create json file")?;
        serde_json::to_writer_pretty(&mut file, self).wrap_err("fail to write json file")?;

        // let mut file = std::fs::File::create(filename.with_extension("toml"))
        //     .wrap_err("file to create toml file")?;
        // file.write_all(
        //     toml::ser::to_string_pretty(self)
        //         .wrap_err("fail to serialize toml")?
        //         .as_bytes(),
        // )
        // .wrap_err("fail to write toml file")?;
        Ok(())
    }
}
pub fn save_result_list<T: AsRef<Path> + Serialize>(
    ok_list: &[T],
    err_list: &[T],
    filename: &Path,
) -> Result<()> {
    let mut file = std::fs::File::create(filename.with_extension("ok.json"))?;
    serde_json::to_writer_pretty(&mut file, ok_list)?;
    let mut file = std::fs::File::create(filename.with_extension("err.json"))?;
    serde_json::to_writer_pretty(&mut file, err_list)?;

    // // write toml
    // let mut file = std::fs::File::create(filename.with_extension("ok.toml"))?;
    // file.write_all(toml::ser::to_string_pretty(&ok_list).unwrap().as_bytes())?;
    // let mut file = std::fs::File::create(filename.with_extension("err.toml"))?;
    // file.write_all(toml::ser::to_string_pretty(&err_list).unwrap().as_bytes())?;

    Ok(())
}
