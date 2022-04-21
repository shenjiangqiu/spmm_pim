use std::{io::Write, path::Path};

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
    pub origin_nnz: usize,
    pub new_nnz: usize,
    pub new_element: usize,
    pub need_speed_up: f32,
}

impl<'a> Results<'a> {
    pub fn save_to_file(&self, filename: &Path) -> Result<()> {
        let mut file = std::fs::File::create(filename.with_extension("json"))
            .wrap_err("file to create json file")?;
        serde_json::to_writer_pretty(&mut file, self).wrap_err("fail to write json file")?;

        let mut file = std::fs::File::create(filename.with_extension("toml"))
            .wrap_err("file to create toml file")?;
        file.write_all(
            toml::ser::to_string_pretty(self)
                .wrap_err("fail to serialize toml")?
                .as_bytes(),
        )
        .wrap_err("fail to write toml file")?;
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

    // write toml
    let mut file = std::fs::File::create(filename.with_extension("ok.toml"))?;
    file.write_all(toml::ser::to_string_pretty(&ok_list).unwrap().as_bytes())?;
    let mut file = std::fs::File::create(filename.with_extension("err.toml"))?;
    file.write_all(toml::ser::to_string_pretty(&err_list).unwrap().as_bytes())?;

    Ok(())
}
