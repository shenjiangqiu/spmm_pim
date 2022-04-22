use log::debug;
use sprs::SpIndex;

pub struct BsrRowbuilder<N, I, const C: usize, const R: usize> {
    data: Vec<[[N; C]; R]>,
    index: Vec<I>,
    current_working_window: [[N; C]; R],
    current_c: usize,
    current_nnz: usize,
}

impl<N, I, const C: usize, const R: usize> Default for BsrRowbuilder<N, I, C, R>
where
    N: Default + Copy + Clone,
    I: SpIndex,
{
    fn default() -> Self {
        BsrRowbuilder {
            data: vec![[[N::default(); C]; R]; R],
            index: vec![],
            current_working_window: [[N::default(); C]; R],
            current_c: 0,
            current_nnz: 0,
        }
    }
}

impl<N, I, const C: usize, const R: usize> BsrRowbuilder<N, I, C, R>
where
    N: Default + Copy + Clone,
    I: SpIndex,
{
    pub fn new() -> Self {
        Self::default()
    }
    ///  gradually push index and data into the builder, the index ***must*** from small to large
    /// It can handle the the case the the column size is not multiple of C
    /// because the stream manner do not really know the real size of the column size;
    pub fn push_element(&mut self, index: I, value: N, row: usize) {
        if row >= R {
            panic!("row index out of bounds");
        }
        // test if
        if index.index() >= self.current_c + C {
            // push to result and make a new one
            if self.current_nnz != 0 {
                debug!("push to result and make a new one");

                self.data.push(self.current_working_window);
                self.index.push(I::from(self.current_c / C).unwrap());

                self.current_working_window = [[N::default(); C]; R];
                self.current_nnz = 0;
            }
            self.current_c = index.index() / C * C;
        }

        self.current_working_window[row][index.index() - self.current_c] = value;
        self.current_nnz += 1;
    }
    pub fn into_row(self) -> (Vec<I>, Vec<[[N; C]; R]>) {
        let mut me = self;
        if me.current_nnz != 0 {
            me.data.push(me.current_working_window);
            me.index.push(I::from(me.current_c / C).unwrap());
        }

        (me.index, me.data)
    }
}
