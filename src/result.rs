use std::path::Path;

use serde::Serialize;

#[derive(Serialize,Debug)]
pub struct Results<'a> {
    pub all: Vec<SingleResult<'a>>,
}
#[derive(Serialize,Debug)]
pub struct SingleResult<'a> {
    pub file: &'a Path,
    pub c: usize,
    pub origin_nnz: usize,
    pub new_nnz: usize,
    pub new_element: usize,
    pub need_speed_up: f32,
}
