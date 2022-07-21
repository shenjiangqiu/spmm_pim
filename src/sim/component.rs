//! this module contains the definition of the component trait
//! - a component is a struct that can spawn a generator throw run() method

use super::{SpmmGenerator, SpmmStatus};

/// trait Component is the trait that every component should implement
/// - example:
/// ```ignore
/// use qsim::ResourceId;
/// use super::{component::Component, SpmmContex, SpmmGenerator, SpmmStatusEnum};
/// pub struct TaskReordererSetting {}
/// pub struct TaskReorderer {
///     pub task_in: ResourceId,
///     pub task_out: ResourceId,
///     pub task_reorderer_config: TaskReordererSetting,
/// }
/// impl TaskReorderer {
///     pub fn new(
///         task_in: ResourceId,
///         task_out: ResourceId,
///         task_reorderer_config: TaskReordererSetting,
///     ) -> Self {
///         Self {
///             task_in,
///             task_out,
///             task_reorderer_config,
///         }
///     }
/// }
/// impl Component for TaskReorderer {
///     fn run(self) -> Box<SpmmGenerator> {
///         Box::new(move |context: SpmmContex| {
///             let (time, status) = context.into_inner();
///             loop {
///                 let task: SpmmContex =
///                     yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
///                 let (_time, state) = task.into_inner();
///                 let (_, state, _) = state.into_inner();
///                 let task = state.into_push_bank_task().unwrap().1;
///                 // do some reorder work
///                 // finished, push the task
///                 yield status.clone_with_states(SpmmStatusEnum::PushBankTask(self.task_out, task));
///             }
///         })
///     }
/// }
/// ```
pub trait Component {
    /// generate a generator
    /// - should like:
    /// ```ignore
    ///  impl Component for TaskReorderer {
    ///     fn run(self) -> Box<SpmmGenerator> {
    ///         Box::new(move |context: SpmmContex| {
    ///             let (time, status) = context.into_inner();
    ///             loop {
    ///                 let task: SpmmContex =
    ///                     yield status.clone_with_state(SpmmStatusEnum::Pop(self.task_in));
    ///                 let (_time, state) = task.into_inner();
    ///                 let (_, state, _) = state.into_inner();
    ///                 let task = state.into_push_bank_task().unwrap().1;
    ///                 // do some reorder work
    ///                 // finished, push the task
    ///                 yield status.clone_with_states(SpmmStatusEnum::PushBankTask(self.task_out, task));
    ///             }
    ///         })
    ///     }
    /// }
    /// ```
    ///
    ///
    fn run(self, original_status: SpmmStatus) -> Box<SpmmGenerator>;
}
