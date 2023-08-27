// use alloc::{collections::VecDeque, sync::Arc};
// use core::ops::Deref;

// use crate::BaseScheduler;

// /// A task wrapper for the [`SimpleScheduler`].
// pub struct SimpleTask<T> {
//     inner: T,
// }

// impl<T> SimpleTask<T> {
//     /// Creates a new [`SimpleTask`] from the inner task struct.
//     pub const fn new(inner: T) -> Self {
//         Self {
//             inner,
//         }
//     }

//     /// Returns a reference to the inner task struct.
//     pub const fn inner(&self) -> &T {
//         &self.inner
//     }
// }

// impl<T> Deref for SimpleTask<T> {
//     type Target = T;
//     #[inline]
//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

// /// A simple scheduler.
// ///
// /// When a task is added to the scheduler, it's placed at the end of the ready
// /// queue. When picking the next task to run, the head of the ready queue is
// /// taken.
// ///
// /// As it's a cooperative scheduler, it does nothing when the timer tick occurs.
// ///
// pub struct SimpleScheduler<T> {
//     ready_queue: VecDeque<Arc<SimpleTask<T>>>,
// }

// impl<T> SimpleScheduler<T> {
//     /// Creates a new empty [`SimpleScheduler`].
//     pub const fn new() -> Self {
//         Self {
//             ready_queue: VecDeque::new(),
//         }
//     }
//     /// get the name of scheduler
//     pub fn scheduler_name() -> &'static str {
//         "Simple"
//     }
// }

// impl<T> BaseScheduler for SimpleScheduler<T> {
//     type SchedItem = Arc<SimpleTask<T>>;

//     fn init(&mut self) {}

//     fn add_task(&mut self, task: Self::SchedItem) {
//         trace!("######### add_task");
//         self.ready_queue.push_back(task);
//     }

//     fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
//         trace!("######### remove_task");
//         self.ready_queue
//             .iter()
//             .position(|t| Arc::ptr_eq(t, task))
//             .and_then(|idx| self.ready_queue.remove(idx))
//     }

//     fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
//         self.ready_queue.pop_front()
//     }

//     fn put_prev_task(&mut self, prev: Self::SchedItem, _preempt: bool) {
//         self.ready_queue.push_back(prev);
//     }

//     fn task_tick(&mut self, _current: &Self::SchedItem) -> bool {
//         false // no reschedule
//     }

//     fn set_priority(&mut self, _task: &Self::SchedItem, _prio: isize) -> bool {
//         false
//     }
// }



/********************************************************************** */


use alloc::{collections::BTreeMap, sync::Arc};
use core::ops::Deref;
use core::sync::atomic::{AtomicIsize, Ordering};

use crate::BaseScheduler;

/// task for CFS
pub struct SimpleTask<T> {
    inner: T,
    init_vruntime: AtomicIsize,
    delta: AtomicIsize,
    nice: AtomicIsize,
    id: AtomicIsize,
}



const NICE_RANGE_POS: usize = 19; // MAX_NICE in Linux
const NICE_RANGE_NEG: usize = 20; // -MIN_NICE in Linux, the range of nice is [MIN_NICE, MAX_NICE]


const NICE2WEIGHT_POS: [isize; NICE_RANGE_POS + 1] = [
    1024, 820, 655, 526, 423, 335, 272, 215, 172, 137, 110, 87, 70, 56, 45, 36, 29, 23, 18, 15,
];
const NICE2WEIGHT_NEG: [isize; NICE_RANGE_NEG + 1] = [
    1024, 1277, 1586, 1991, 2501, 3121, 3906, 4904, 6100, 7620, 9548, 11916, 14949, 18705, 23254,
    29154, 36291, 46273, 56483, 71755, 88761,
];

impl<T> SimpleTask<T> {
    /// new with default values
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            init_vruntime: AtomicIsize::new(0_isize),
            delta: AtomicIsize::new(0_isize),
            nice: AtomicIsize::new(0_isize),
            id: AtomicIsize::new(0_isize),
        }
    }

    fn get_weight(&self) -> isize {
        let nice = self.nice.load(Ordering::Acquire);
        if nice >= 0 {
            NICE2WEIGHT_POS[nice as usize]
        } else {
            NICE2WEIGHT_NEG[(-nice) as usize]
        }
    }

    fn get_id(&self) -> isize {
        self.id.load(Ordering::Acquire)
    }

    fn get_vruntime(&self) -> isize {
        if self.nice.load(Ordering::Acquire) == 0 {
            self.init_vruntime.load(Ordering::Acquire) + self.delta.load(Ordering::Acquire)
        } else {
            self.init_vruntime.load(Ordering::Acquire)
                + self.delta.load(Ordering::Acquire) * 1024 / self.get_weight()
        }
    }

    fn set_vruntime(&self, v: isize) {
        self.init_vruntime.store(v, Ordering::Release);
    }

    // Simple Implementation: no change in vruntime.
    // Only modifying priority of current process is supported currently.
    fn set_priority(&self, nice: isize) {
        let current_init_vruntime = self.get_vruntime();
        self.init_vruntime
            .store(current_init_vruntime, Ordering::Release);
        self.delta.store(0, Ordering::Release);
        self.nice.store(nice, Ordering::Release);
    }

    fn set_id(&self, id: isize) {
        self.id.store(id, Ordering::Release);
    }

    fn task_tick(&self) {
        self.delta.fetch_add(1, Ordering::Release);
    }

    /// Returns a reference to the inner task struct.
    pub const fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T> Deref for SimpleTask<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}


pub struct SimpleScheduler<T> {
    ready_queue: BTreeMap<(isize, isize), Arc<SimpleTask<T>>>, // (vruntime, taskid)
    min_vruntime: Option<AtomicIsize>,
    id_pool: AtomicIsize,
}

impl<T> SimpleScheduler<T> {
    /// Creates a new empty [`CFScheduler`].
    pub const fn new() -> Self {
        Self {
            ready_queue: BTreeMap::new(),
            min_vruntime: None,
            id_pool: AtomicIsize::new(0_isize),
        }
    }
    /// get the name of scheduler
    pub fn scheduler_name() -> &'static str {
        "Completely Fair"
    }
}

impl<T> BaseScheduler for SimpleScheduler<T> {
    type SchedItem = Arc<SimpleTask<T>>;

    fn init(&mut self) {}

    fn add_task(&mut self, task: Self::SchedItem) {
        if self.min_vruntime.is_none() {
            self.min_vruntime = Some(AtomicIsize::new(0_isize));
        }
        let vruntime = self.min_vruntime.as_mut().unwrap().load(Ordering::Acquire);
        let taskid = self.id_pool.fetch_add(1, Ordering::Release);
        task.set_vruntime(vruntime);
        task.set_id(taskid);
        self.ready_queue.insert((vruntime, taskid), task);
        if let Some(((min_vruntime, _), _)) = self.ready_queue.first_key_value() {
            self.min_vruntime = Some(AtomicIsize::new(*min_vruntime));
        } else {
            self.min_vruntime = None;
        }
    }

    fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        if let Some((_, tmp)) = self
            .ready_queue
            .remove_entry(&(task.clone().get_vruntime(), task.clone().get_id()))
        {
            if let Some(((min_vruntime, _), _)) = self.ready_queue.first_key_value() {
                self.min_vruntime = Some(AtomicIsize::new(*min_vruntime));
            } else {
                self.min_vruntime = None;
            }
            Some(tmp)
        } else {
            None
        }
    }

    fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
        if let Some((_, v)) = self.ready_queue.pop_first() {
            Some(v)
        } else {
            None
        }
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, _preempt: bool) {
        let taskid = self.id_pool.fetch_add(1, Ordering::Release);
        prev.set_id(taskid);
        self.ready_queue
            .insert((prev.clone().get_vruntime(), taskid), prev);
    }

    fn task_tick(&mut self, current: &Self::SchedItem) -> bool {
        current.task_tick();
        self.min_vruntime.is_none()
            || current.get_vruntime() > self.min_vruntime.as_mut().unwrap().load(Ordering::Acquire)
    }

    fn set_priority(&mut self, task: &Self::SchedItem, prio: isize) -> bool {
        if (-20..=19).contains(&prio) {
            task.set_priority(prio);
            true
        } else {
            false
        }
    }
}