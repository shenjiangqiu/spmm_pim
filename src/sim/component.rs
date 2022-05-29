use super::SpmmGenerator;

pub trait Component {
    fn run(self) -> Box<SpmmGenerator>;
}
