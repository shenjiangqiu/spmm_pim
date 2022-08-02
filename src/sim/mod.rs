pub mod bank;
pub mod buffer_status;
pub mod channel_merger;
pub mod chip_merger;
pub mod comp_collector;
pub mod component;
pub mod dimm_merger;
pub mod final_receiver;
pub mod full_result_merger_worker;
pub mod id_translation;
pub mod merger_status;
pub mod merger_task_dispather;
pub mod merger_task_sender;
pub mod partial_sum_collector;
pub mod partial_sum_sender;
pub mod partial_sum_sender_bank;
pub mod partial_sum_sender_dimm;
pub mod partial_sum_signal_collector;
pub mod queue_tracker;
pub mod sim_time;
mod task_balance;
pub mod task_reorderer;
pub mod task_router;
pub mod task_sender;
pub mod types;

use id_translation::*;
use itertools::Itertools;
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use qsim::{prelude::*, resources::Store};

use self::{
    bank::{BankPe, BankTaskReorder},
    buffer_status::SharedBufferStatus,
    channel_merger::ChannelMerger,
    chip_merger::ChipMerger,
    dimm_merger::DimmMerger,
    final_receiver::FinalReceiver,
    full_result_merger_worker::FullResultMergerWorker,
    merger_task_dispather::MergerWorkerDispatcher,
    partial_sum_collector::PartialSumCollector,
    partial_sum_sender::PartialSumSender,
    partial_sum_sender_bank::PartialSumSenderBank,
    partial_sum_sender_dimm::PartialSumSenderDimm,
    partial_sum_signal_collector::PartialSumSignalCollector,
    queue_tracker::QueueTrackerId,
    sim_time::{
        DetailedTimeStats, LevelTime, LevelTimeId, SharedNamedTime, SharedSimTime, TimeStats,
    },
    task_sender::TaskSender,
    types::SpmmStatus,
};
use crate::{
    settings::MemSettings,
    sim::{
        comp_collector::ProcessInfoCollector,
        merger_status::SharedMergerStatus,
        queue_tracker::QueueTracker,
        sim_time::SharedEndTime,
        task_balance::{DefaultTaskScheduler, RandomTaskScheduler},
        types::{SharedStatus, SpmmStatusEnum},
    },
    two_matrix::TwoMatrix,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

fn build_dimm(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    task_send_store: usize,
    status: SpmmStatus,
    final_data_receiver: usize,
    _dimm_level_id: LevelTimeId,
    channel_level_id: LevelTimeId,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    p_collector: &mut ProcessInfoCollector,
    sender_id_to_name_mapping: &mut BTreeMap<usize, String>,
    queue_tracker_id_recv: QueueTrackerId,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 2. add the Dimm
    let num_channels = mem_settings.channels;
    // task send from dimm to channel
    let channel_stores = (0..num_channels)
        .map(|_i| {
            sim.create_resource(
                Box::new(Store::new(mem_settings.sender_store_size)),
                "dimm_to_channel",
            )
        })
        .collect_vec();
    for (index, id) in channel_stores.iter().enumerate() {
        sender_id_to_name_mapping.insert(*id, format!("channel_{}", index));
    }
    let merger_status_id = shared_status
        .shared_merger_status
        .add_component(mem_settings.dimm_merger_count);

    let sim_time_id = shared_status
        .shared_named_time
        .add_component_with_name("DIMMSENDER_GETID", vec!["dimm"]);
    let buffer_status_id = shared_status
        .shared_buffer_status
        .add_component(mem_settings.dimm_buffer_lines);

    let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_dimm");
    let ready_id_queue = sim.create_resource(Box::new(Store::new(128)), "ready_dimm");
    let named_sim_time = shared_status.shared_named_time.add_component_with_name(
        "dimm_signal_collector",
        vec!["signal_collector", "dimm_signal_collector"],
    );
    let dimm_signal_collector = PartialSumSignalCollector {
        queue_id_signal_in: signal_in,
        queue_id_ready_out: ready_id_queue,
        buffer_status_id,
        level_id: LevelId::Dimm,
        named_sim_time,
    };
    p_collector.create_process_and_schedule(sim, dimm_signal_collector, &status);
    let collector_to_dispatcher =
        sim.create_resource(Box::new(Store::new(1)), "collector_to_dispatcher_dimm");
    let named_sim_time = shared_status.shared_named_time.add_component_with_name(
        "DIMM_DATA_COLLECTOR",
        vec!["dimm_data_collector", "data_collector"],
    );

    let dimm_partial_sum_data_collector = PartialSumCollector {
        queue_id_ready_in: ready_id_queue,
        queue_id_full_result_out: collector_to_dispatcher,
        queue_id_pop_signal_out: signal_in,
        level_id: LevelId::Dimm,
        buffer_status_id,
        named_sim_time,
        is_bind: mem_settings.buffer_mode.is_bind_merger(),
    };

    p_collector.create_process_and_schedule(sim, dimm_partial_sum_data_collector, &status);
    let queue_tracker_id_send = (0..num_channels)
        .map(|i| {
            shared_status
                .queue_tracker
                .add_component_with_name(format!("dimm_sender-{i}"))
        })
        .collect_vec();

    let dimm = DimmMerger::new(
        LevelId::Dimm,
        task_send_store,
        channel_stores.clone(),
        merger_status_id,
        sim_time_id,
        buffer_status_id,
        queue_tracker_id_recv,
        queue_tracker_id_send.clone(),
    );

    p_collector.create_process_and_schedule(sim, dimm, &status);
    // create the merger_task_worker
    let mut task_receiver = vec![];
    for i in 0..mem_settings.dimm_merger_count {
        let merger_to_sender_queue =
            sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_dimm");
        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "dimm_sum_sender",
            vec!["partial_sum_sender", "dimm_sum_sender"],
        );
        let dimm_signal_sender = PartialSumSenderDimm {
            queue_id_partial_sum_in: merger_to_sender_queue,
            queue_id_partial_sum_out: final_data_receiver,
            level_id: LevelId::Dimm,
            named_sim_time,
            id: i,
            merger_status_id,
            is_binding: mem_settings.buffer_mode.is_bind_merger(),
            queue_id_finished_signal_out: collector_to_dispatcher,
        };
        p_collector.create_process_and_schedule(sim, dimm_signal_sender, &status);

        let full_partial_sum_in =
            sim.create_resource(Box::new(Store::new(1)), "dispatcher_to_merger_dimm");
        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "DIMM_MERGER_TASK_WORKER",
            vec!["dimm_merger_task_worker", "merger_task_worker"],
        );
        let merger_task_worker = FullResultMergerWorker {
            buffer_status_id,
            level_id: LevelId::Dimm,
            id: i,
            queue_id_partial_sum_sender: merger_to_sender_queue,
            queue_id_partial_sum_in: full_partial_sum_in,
            self_sender_id: task_send_store,
            merger_status_id,
            merger_width: mem_settings.dimm_merger_size,
            named_sim_time,
            is_bind: mem_settings.buffer_mode.is_bind_merger(),
            queue_id_finished_signal_out: collector_to_dispatcher,
        };
        p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
        task_receiver.push(full_partial_sum_in);
    }

    // create the dimm merger_task_dispatcher
    let merger_task_dispatcher = MergerWorkerDispatcher {
        level_id: LevelId::Dimm,
        merger_status_id,
        merger_task_sender: task_receiver,
        full_sum_in: collector_to_dispatcher,
        is_binding: mem_settings.buffer_mode.is_bind_merger(),
    };

    p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);
    build_channel(
        mem_settings,
        sim,
        status,
        channel_level_id,
        chip_level_id,
        bank_level_id,
        channel_stores,
        signal_in,
        p_collector,
        sender_id_to_name_mapping,
        queue_tracker_id_send,
    )?;
    Ok(())
}

fn build_channel(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    channel_level_id: LevelTimeId,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    channel_task_senders: Vec<usize>,
    dimm_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
    sender_id_to_name_mapping: &mut BTreeMap<usize, String>,
    queue_tracker_id_recv: Vec<QueueTrackerId>,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();

    // 3. add the Channel
    for (channel_id, (dimm_to_channel_task_sender, queue_tracker_id_recv)) in channel_task_senders
        .into_iter()
        .zip(queue_tracker_id_recv)
        .enumerate()
    {
        // create the channel!
        let num_chips = mem_settings.chips;

        // the channel that send task to the chip from this channel
        let chip_stores = (0..num_chips)
            .map(|_i| {
                sim.create_resource(
                    Box::new(Store::new(mem_settings.sender_store_size)),
                    "channel_to_chip",
                )
            })
            .collect_vec();
        for (index, id) in chip_stores.iter().enumerate() {
            sender_id_to_name_mapping.insert(*id, format!("chip_{}.{}", channel_id, index));
        }
        let merger_status_id = shared_status
            .shared_merger_status
            .add_component(mem_settings.channel_merger_count);

        let sim_time = shared_status
            .shared_named_time
            .add_component_with_name("channel_sender", vec!["channel_task_sender", "task_sender"]);
        let buffer_status_id = shared_status
            .shared_buffer_status
            .add_component(mem_settings.channel_buffer_lines);

        let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_channel");
        let ready_queueid = sim.create_resource(Box::new(Store::new(128)), "ready_channel");
        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "channel_signal_collector",
            vec!["signal_collector", "channel_signal_collector"],
        );
        let channel_signal_collector = PartialSumSignalCollector {
            queue_id_signal_in: signal_in,
            queue_id_ready_out: ready_queueid,
            buffer_status_id,
            level_id: LevelId::Channel(channel_id),
            named_sim_time,
        };

        p_collector.create_process_and_schedule(sim, channel_signal_collector, &status);

        let collector_to_dispatcher =
            sim.create_resource(Box::new(Store::new(1)), "collector_to_dispatcher_channel");

        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "CHANNEL_DATA_COLLECTOR",
            vec!["channel_data_collector", "data_collector"],
        );
        let channel_partial_sum_data_collector = PartialSumCollector {
            queue_id_ready_in: ready_queueid,
            queue_id_full_result_out: collector_to_dispatcher,
            queue_id_pop_signal_out: signal_in,
            level_id: LevelId::Channel(channel_id),
            buffer_status_id,
            named_sim_time,
            is_bind: mem_settings.buffer_mode.is_bind_merger(),
        };

        p_collector.create_process_and_schedule(sim, channel_partial_sum_data_collector, &status);
        let queue_tracker_id_send = (0..num_chips)
            .map(|i| {
                shared_status
                    .queue_tracker
                    .add_component_with_name(format!("channel_sender-{i}"))
            })
            .collect_vec();
        let channel = ChannelMerger::new(
            LevelId::Channel(channel_id),
            dimm_to_channel_task_sender,
            chip_stores.clone(),
            merger_status_id,
            channel_level_id,
            sim_time,
            buffer_status_id,
            queue_tracker_id_recv,
            queue_tracker_id_send.clone(),
        );

        // create the process
        p_collector.create_process_and_schedule(sim, channel, &status);
        // create the merger_task_worker
        let mut task_receiver = vec![];
        for i in 0..mem_settings.channel_merger_count {
            let merger_to_sender_queue =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_channel");
            let named_sim_time = shared_status.shared_named_time.add_component_with_name(
                "channel_sum_sender",
                vec!["partial_sum_sender", "channel_sum_sender"],
            );
            let channel_signal_sender = PartialSumSender {
                queue_id_partial_sum_in: merger_to_sender_queue,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(0)), "data_provider_of_sender_channel"),
                queue_id_signal_out: dimm_signal_in,
                level_id: LevelId::Channel(channel_id),
                named_sim_time,
                id: i,
                merger_status_id,
                is_binding: mem_settings.buffer_mode.is_bind_merger(),
                queue_id_finished_signal_out: collector_to_dispatcher,
            };
            p_collector.create_process_and_schedule(sim, channel_signal_sender, &status);

            let resouce =
                sim.create_resource(Box::new(Store::new(1)), "dispatcher_to_merger_channel");
            let named_sim_time = shared_status.shared_named_time.add_component_with_name(
                "channel_merger_task_worker",
                vec!["merger_task_worker", "channel_merger_task_worker"],
            );

            let merger_task_worker = FullResultMergerWorker {
                buffer_status_id,
                level_id: LevelId::Channel(channel_id),
                id: i,
                queue_id_partial_sum_sender: merger_to_sender_queue,
                queue_id_partial_sum_in: resouce,
                self_sender_id: dimm_to_channel_task_sender,
                merger_status_id,
                merger_width: mem_settings.channel_merger_size,
                named_sim_time,
                is_bind: mem_settings.buffer_mode.is_bind_merger(),
                queue_id_finished_signal_out: collector_to_dispatcher,
            };
            p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
            task_receiver.push(resouce);
        }
        // create the channel merger_task_dispatcher
        let merger_task_dispatcher = MergerWorkerDispatcher {
            level_id: LevelId::Channel(channel_id),
            merger_status_id,
            merger_task_sender: task_receiver,
            full_sum_in: collector_to_dispatcher,
            is_binding: mem_settings.buffer_mode.is_bind_merger(),
        };
        p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);
        build_chip(
            mem_settings,
            sim,
            status.clone(),
            chip_level_id,
            bank_level_id,
            chip_stores,
            channel_id,
            signal_in,
            p_collector,
            sender_id_to_name_mapping,
            queue_tracker_id_send,
        )?;
    }
    Ok(())
}
fn build_chip(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    chip_level_id: LevelTimeId,
    bank_level_id: LevelTimeId,
    chip_stores: Vec<usize>,
    channel_id: ChannelID,
    channel_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
    sender_id_to_name_mapping: &mut BTreeMap<usize, String>,
    queue_tracker_id_recv: Vec<QueueTrackerId>,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 4. add the chip
    for (chip_id, (store_id, queue_tracker_id_recv)) in chip_stores
        .into_iter()
        .zip(queue_tracker_id_recv)
        .enumerate()
    {
        // create the chip!
        let chip_id = (channel_id, chip_id);
        let num_banks = mem_settings.banks;
        let bank_stores = (0..num_banks)
            .map(|_i| {
                sim.create_resource(
                    Box::new(Store::new(mem_settings.sender_store_size)),
                    "chip_to_bank",
                )
            })
            .collect_vec();
        for (index, id) in bank_stores.iter().enumerate() {
            sender_id_to_name_mapping.insert(*id, format!("bank_{:?}.{}", &chip_id, index));
        }
        let merger_status_id = shared_status
            .shared_merger_status
            .add_component(mem_settings.chip_merger_count);
        let sim_time_id = shared_status
            .shared_named_time
            .add_component_with_name("chip_sender", vec!["chip_task_sender", "task_sender"]);
        let buffer_status_id = shared_status
            .shared_buffer_status
            .add_component(mem_settings.chip_buffer_lines);

        let signal_in = sim.create_resource(Box::new(Store::new(128)), "signal_chip");
        let ready_queueid = sim.create_resource(Box::new(Store::new(128)), "ready_chip");
        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "chip_singal_collector",
            vec!["chip_singal_collector", "signal_collector"],
        );
        let chip_signal_collector = PartialSumSignalCollector {
            queue_id_signal_in: signal_in,
            queue_id_ready_out: ready_queueid,
            buffer_status_id,
            level_id: LevelId::Chip(chip_id),
            named_sim_time,
        };
        p_collector.create_process_and_schedule(sim, chip_signal_collector, &status);
        let collector_to_dispatcher =
            sim.create_resource(Box::new(Store::new(1)), "collector_to_dispatcher_chip");

        let named_sim_time = shared_status.shared_named_time.add_component_with_name(
            "CHIP_DATA_COLLECTOR",
            vec!["chip_data_collector", "data_collector"],
        );

        let chip_partial_sum_data_collector = PartialSumCollector {
            queue_id_ready_in: ready_queueid,
            queue_id_full_result_out: collector_to_dispatcher,
            queue_id_pop_signal_out: signal_in,
            level_id: LevelId::Chip(chip_id),
            buffer_status_id,
            named_sim_time,
            is_bind: mem_settings.buffer_mode.is_bind_merger(),
        };
        p_collector.create_process_and_schedule(sim, chip_partial_sum_data_collector, &status);
        let queue_tracker_id_send = (0..num_banks)
            .map(|i| {
                shared_status
                    .queue_tracker
                    .add_component_with_name(format!("chip_sender-{i}"))
            })
            .collect_vec();
        let chip = ChipMerger::new(
            LevelId::Chip(chip_id),
            store_id,
            bank_stores.clone(),
            merger_status_id,
            chip_level_id,
            sim_time_id,
            buffer_status_id,
            queue_tracker_id_recv,
            queue_tracker_id_send.clone(),
        );

        // create the process
        p_collector.create_process_and_schedule(sim, chip, &status);
        // create the merger_task_worker
        let mut task_receiver = vec![];
        for i in 0..mem_settings.chip_merger_count {
            // build partial sum sender, signal collector and data collector
            let merger_to_sender_queue =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_chip");

            let named_sim_time = shared_status.shared_named_time.add_component_with_name(
                "chip_partial_sum_sender",
                vec!["chip_partial_sum_sender", "partial_sum_sender"],
            );
            let chip_signal_sender = PartialSumSender {
                queue_id_partial_sum_in: merger_to_sender_queue,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(0)), "data_provider_of_sender_chip"),
                queue_id_signal_out: channel_signal_in,
                level_id: LevelId::Chip(chip_id),
                named_sim_time,
                merger_status_id,
                is_binding: mem_settings.buffer_mode.is_bind_merger(),
                id: i,
                queue_id_finished_signal_out: collector_to_dispatcher,
            };
            p_collector.create_process_and_schedule(sim, chip_signal_sender, &status);

            let resouce = sim.create_resource(Box::new(Store::new(1)), "dispatcher_to_merger_chip");
            let named_sim_time = shared_status.shared_named_time.add_component_with_name(
                "chip_merger_task_worker",
                vec!["merger_task_worker", "chip_merger_task_worker"],
            );

            let merger_task_worker = FullResultMergerWorker {
                buffer_status_id,
                level_id: LevelId::Chip(chip_id),
                id: i,
                queue_id_partial_sum_sender: merger_to_sender_queue,
                queue_id_partial_sum_in: resouce,
                self_sender_id: store_id,
                merger_status_id,
                merger_width: mem_settings.chip_merger_size,
                named_sim_time,
                is_bind: mem_settings.buffer_mode.is_bind_merger(),
                queue_id_finished_signal_out: collector_to_dispatcher,
            };
            p_collector.create_process_and_schedule(sim, merger_task_worker, &status);
            task_receiver.push(resouce);
        }
        // create the chip merger_task_dispatcher
        let merger_task_dispatcher = MergerWorkerDispatcher {
            level_id: LevelId::Chip(chip_id),
            merger_status_id,
            merger_task_sender: task_receiver,
            full_sum_in: collector_to_dispatcher,
            is_binding: mem_settings.buffer_mode.is_bind_merger(),
        };
        p_collector.create_process_and_schedule(sim, merger_task_dispatcher, &status);

        build_bank(
            mem_settings,
            sim,
            status.clone(),
            bank_level_id,
            bank_stores,
            chip_id,
            signal_in,
            p_collector,
            sender_id_to_name_mapping,
            queue_tracker_id_send,
        )?;
    }
    // start

    Ok(())
    // what we should to output?
}

fn build_bank(
    mem_settings: &MemSettings,
    sim: &mut Simulation<SpmmStatus>,
    status: SpmmStatus,
    _bank_level_id: LevelTimeId,
    bank_stores: Vec<usize>,
    chip_id: ChipID,
    chip_signal_in: usize,
    p_collector: &mut ProcessInfoCollector,
    _sender_id_to_name_mapping: &mut BTreeMap<usize, String>,
    queue_tracker_id_recv: Vec<QueueTrackerId>,
) -> eyre::Result<()> {
    let shared_status = status.shared_status.clone();
    // 5. add the bank
    for (bank_id, (store_id, queue_tracker_id_recv)) in bank_stores
        .into_iter()
        .zip(queue_tracker_id_recv)
        .enumerate()
    {
        // create the bank!
        let bank_id = (chip_id, bank_id);

        let bank_pe_stores = (0..mem_settings.bank_merger_count)
            .map(|_i| {
                sim.create_resource(
                    Box::new(Store::new(mem_settings.sender_store_size)),
                    "bank_to_pe",
                )
            })
            .collect_vec();

        let comp_id = shared_status.shared_named_time.add_component_with_name(
            &format!("bank_reorder-{bank_id:?}"),
            vec!["bank_bank_reorder", "bank_reorder"],
        );
        let end_time_id = shared_status
            .shared_end_time
            .add_component_with_name(format!("bank_reorder-{bank_id:?}"));
        let bank = BankTaskReorder::new(
            LevelId::Bank(bank_id),
            store_id,
            bank_pe_stores.clone(),
            mem_settings.reorder_count,
            bank_id,
            mem_settings.row_change_latency as f64,
            comp_id,
            end_time_id,
            queue_tracker_id_recv,
        );

        // create the process
        p_collector.create_process_and_schedule(sim, bank, &status);

        for (bank_pe_id, bank_pe_store_id) in bank_pe_stores.into_iter().enumerate() {
            // create the partial sum sender, signal collector and data collector
            let merger_to_sender =
                sim.create_resource(Box::new(Store::new(1)), "merger_to_sender_bank");

            let named_sim_time = shared_status.shared_named_time.add_component_with_name(
                "bank_partial_sum_sender",
                vec!["bank_partial_sum_sender", "partial_sum_sender"],
            );
            let bank_signal_sender = PartialSumSenderBank {
                queue_id_partial_sum_in: merger_to_sender,
                queue_id_partial_sum_out: sim
                    .create_resource(Box::new(Store::new(0)), "data_provider_of_sender_bank"),
                queue_id_signal_out: chip_signal_in,
                level_id: LevelId::Bank(bank_id),
                named_sim_time,
            };
            p_collector.create_process_and_schedule(sim, bank_signal_sender, &status);

            let comp_id = shared_status.shared_named_time.add_component_with_name(
                format!("bank_pe_{bank_id:?}_{bank_pe_id}"),
                vec!["bank_pe"],
            );
            let end_time_id = shared_status
                .shared_end_time
                .add_component_with_name(format!("bank_pe-{bank_id:?}-{bank_pe_id}"));
            let bank_pe = BankPe::new(
                LevelId::Bank(bank_id),
                bank_pe_id,
                bank_pe_store_id,
                merger_to_sender,
                mem_settings.bank_merger_size,
                mem_settings.bank_adder_size,
                store_id,
                comp_id,
                end_time_id,
            );
            p_collector.create_process_and_schedule(sim, bank_pe, &status);
        }
    }

    Ok(())
}

pub struct Simulator {}
impl Simulator {
    /// run the simulator
    pub fn run(
        mem_settings: &MemSettings,
        input_matrix: TwoMatrix<i32, i32>,
    ) -> Result<(f64, TimeStats, DetailedTimeStats, Vec<(String, f64)>), eyre::Report> {
        let mut sender_id_to_name_mapping = BTreeMap::<usize, String>::new();

        let total_rows = input_matrix.a.rows();
        // now we need a stucture to map the sim_time id to the real component time

        // the statistics
        let mut p_collector = ProcessInfoCollector::new(true);
        // 1.---- the basic data
        debug!("start to run");
        let mut sim = Simulation::new();
        // let merger_status = Rc::new(RefCell::new(FullMergerStatus::new()));
        let bankpe_status = Rc::new(RefCell::new(BTreeMap::new()));
        let shared_level_time = Rc::new(LevelTime::new());

        let dimm_level_id = shared_level_time.add_level();
        let channel_level_id = shared_level_time.add_level();
        let chip_level_id = shared_level_time.add_level();
        let bank_level_id = shared_level_time.add_level();

        let shared_named_time = Rc::new(SharedNamedTime::new());
        let shared_buffer_status = Rc::new(SharedBufferStatus::default());
        let sim_time = Rc::new(SharedSimTime::new());
        let shared_merger_status = Rc::new(SharedMergerStatus::new(
            mem_settings.buffer_mode.is_bind_merger(),
        ));
        let shared_end_time = Rc::new(SharedEndTime::new());
        let queue_tracker = Rc::new(QueueTracker::new());
        let shared_status = SharedStatus {
            shared_bankpe_status: bankpe_status,
            shared_sim_time: sim_time,
            shared_level_time,
            shared_named_time,
            shared_buffer_status,
            shared_merger_status,
            shared_end_time,
            queue_tracker,
        };

        let status = SpmmStatus::new(SpmmStatusEnum::Continue, shared_status.clone());

        let final_receiver_resouce = sim.create_resource(Box::new(Store::new(1)), "final_receiver");
        let all_received = Rc::new(RefCell::new(Vec::new()));
        let final_rev = FinalReceiver::new(
            final_receiver_resouce,
            true,
            &input_matrix,
            all_received.clone(),
        );

        p_collector.create_process_and_schedule(&mut sim, final_rev, &status);
        // this store connect the task sender and the Dimm
        let task_send_store = sim.create_resource(
            Box::new(Store::new(mem_settings.sender_store_size)),
            "task_send_store",
        );
        sender_id_to_name_mapping.insert(task_send_store, "dimm".to_string());
        let queue_tracker_id_send = shared_status
            .queue_tracker
            .add_component_with_name("channel_sender");
        let real_row_mapping = mem_settings
            .row_mapping
            .to_real_row_mapping(mem_settings.interleaved_chunk);
        match mem_settings.task_scheduler_mode {
            crate::settings::TaskSchedulerMode::Sequence => {
                let task_sender = TaskSender::<DefaultTaskScheduler>::new(
                    input_matrix.a,
                    input_matrix.b,
                    task_send_store,
                    mem_settings.channels,
                    mem_settings.chips,
                    mem_settings.banks,
                    real_row_mapping,
                    queue_tracker_id_send,
                );
                p_collector.create_process_and_schedule(&mut sim, task_sender, &status);
            }
            crate::settings::TaskSchedulerMode::Shuffle => {
                let task_sender = TaskSender::<RandomTaskScheduler>::new(
                    input_matrix.a,
                    input_matrix.b,
                    task_send_store,
                    mem_settings.channels,
                    mem_settings.chips,
                    mem_settings.banks,
                    real_row_mapping,
                    queue_tracker_id_send,
                );
                p_collector.create_process_and_schedule(&mut sim, task_sender, &status);
            }
        }

        build_dimm(
            mem_settings,
            &mut sim,
            task_send_store,
            status.clone(),
            final_receiver_resouce,
            dimm_level_id,
            channel_level_id,
            chip_level_id,
            bank_level_id,
            &mut p_collector,
            &mut sender_id_to_name_mapping,
            queue_tracker_id_send,
        )?;
        // p_collector.show_data();

        let sim = sim.run(EndCondition::NoEvents);
        // validate the result

        sim.print_resources();
        let time = sim.time();
        status.shared_status.shared_named_time.show_data(time);
        let time_stats = status.shared_status.shared_named_time.get_stats(time);
        info!(
            "all_received: count: {:?}, min: {:?}, max: {:?}",
            all_received.borrow().len(),
            all_received.borrow().iter().min(),
            all_received.borrow().iter().max(),
        );
        info!("original_matrix: {}", total_rows);
        if all_received.borrow().len() != total_rows {
            error!(
                "the received data is not correct,received: {},should be:{}",
                all_received.borrow().len(),
                total_rows
            );
        }
        // output the mapping of sender id:
        info!(
            "sender_id_to_name_mapping:\n{}",
            serde_json::to_string_pretty(&sender_id_to_name_mapping).unwrap()
        );
        let detailed_time_stats = status
            .shared_status
            .shared_named_time
            .get_detailed_stats(time);
        let end_time_stats = status.shared_status.shared_end_time.get_stats(time);
        Ok((time, time_stats, detailed_time_stats, end_time_stats))
    }
}

#[cfg(test)]
mod test {

    use sprs::CsMat;

    use crate::{
        settings::{BufferMode, RowMapping, TaskSchedulerMode},
        sim::task_balance::TaskScheduler,
    };

    use super::*;

    #[test]
    fn sim_test() {
        // ---- first create neccessary status structures
        let config_str = include_str!("../../log_config.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or(());

        debug!("start");
        let csr: CsMat<i32> = sprs::io::read_matrix_market("mtx/bfwa62.mtx")
            .unwrap()
            .to_csr();
        let trans_pose = csr.transpose_view().to_csr();
        let two_matrix = TwoMatrix::new(csr, trans_pose);
        let mem_settings = MemSettings {
            row_size: 512,
            banks: 2,
            chips: 2,
            channels: 2,
            row_mapping: RowMapping::Chunk,
            interleaved_chunk: 10,
            bank_merger_size: 2,
            chip_merger_size: 2,
            channel_merger_size: 2,
            dimm_merger_size: 2,
            simd_width: 128,
            parallel_count: 8,
            reorder_count: 8,
            bank_merger_count: 2,
            chip_merger_count: 2,
            channel_merger_count: 2,
            dimm_merger_count: 2,
            row_change_latency: 8,
            bank_adder_size: 8,
            sender_store_size: 4,
            dimm_buffer_lines: 2,
            channel_buffer_lines: 2,
            chip_buffer_lines: 2,
            buffer_mode: BufferMode::Standalone,
            task_scheduler_mode: TaskSchedulerMode::Shuffle,
        };
        Simulator::run(&mem_settings, two_matrix).unwrap();
    }
}
