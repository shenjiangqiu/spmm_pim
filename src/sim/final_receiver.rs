use std::{cell::RefCell, rc::Rc};

use genawaiter::rc::{Co, Gen};
use log::debug;
use qsim::ResourceId;

use crate::{
    csv_nodata::CsVecNodata,
    sim::{PartialResultTaskType, StateWithSharedStatus},
    two_matrix::TwoMatrix,
};

use super::{component::Component, SpmmContex, SpmmStatus, SpmmStatusEnum};
#[derive(Debug)]
pub struct FinalReceiver {
    pub receiver: ResourceId,
    pub collect_result: bool,
    pub result_matrix: Vec<CsVecNodata<usize>>,
    pub all_received: Rc<RefCell<Vec<usize>>>,
}

impl FinalReceiver {
    pub fn new(
        receiver: ResourceId,
        collect_result: bool,
        _tow_matrix: &TwoMatrix<i32, i32>,
        all_received: Rc<RefCell<Vec<usize>>>,
    ) -> Self {
        // there is a bug here, maybe resolve later!
        // let a = &tow_matrix.a;
        // let b = &tow_matrix.b;
        // let c = a * b;
        // let result_matrix = c
        //     .outer_iterator()
        //     .map(|i| i.to_owned().into())
        //     .collect::<Vec<_>>();
        Self {
            receiver,
            collect_result,
            result_matrix: vec![],
            all_received,
        }
    }
}

impl Component for FinalReceiver {
    fn run(self, original_status: SpmmStatus) -> Box<super::SpmmGenerator> {
        let function = |co: Co<SpmmStatus, SpmmContex>| async move {
            let mut all_rows_collected = vec![];
            loop {
                let ret: SpmmContex = co
                    .yield_(original_status.clone_with_state(SpmmStatusEnum::Pop(self.receiver)))
                    .await;
                debug!("FINIAL_RECIEVER: received final result: {:?}", ret);
                let (_time, pop_status) = ret.into_inner();
                let StateWithSharedStatus {
                    status,
                    shared_status: _,
                } = pop_status.into_inner();
                let (_resouce_id, (target_row, sender_id, result)): (usize, PartialResultTaskType) =
                    status.into_push_partial_task().unwrap();
                debug!("FINIAL_RECIEVER: {}:{}:{:?}", target_row, sender_id, result);
                // there is a bug here, maybe resolve later!
                // assert_eq!(result.indices, self.result_matrix[target_row].indices);
                self.all_received.borrow_mut().push(target_row);
                if self.collect_result {
                    all_rows_collected.push(target_row);
                    debug!(
                        "FINIAL_RECIEVER: all_rows_collected: {:?}",
                        all_rows_collected
                    );
                }
            }
        };

        Box::new(Gen::new(function))
    }
}
