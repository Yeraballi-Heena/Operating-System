//! Integrate OS — Scheduler & Multitasking Subsystem
//!
//! Provides:
//! - Multi-level priority run queues (`RunQueue`)
//! - Monotonic tick sleep queue and timed task wakeup (`SleepQueue`)
//! - Preemption control and timeslice management
//! - Cooperative rescheduling (`yield_now`, `sleep_current`)
//! - Preemptive rescheduling driven by 100 Hz PIT timer interrupt
//! - Kernel thread termination handling (`kernel_thread_exit_stub`)

pub mod priority;
pub mod queue;
pub mod sleep;
pub mod tests;

use crate::klog_ok;
use crate::process::manager::PROCESS_MANAGER;
use crate::process::pid::ProcessId;
use crate::process::state::ProcessState;
use crate::scheduler::priority::Priority;
use crate::scheduler::queue::RunQueue;
use crate::scheduler::sleep::SleepQueue;
use crate::sync::IrqSpinlock;

/// Central system scheduler singleton.
pub static SCHEDULER: IrqSpinlock<Scheduler> = IrqSpinlock::new(Scheduler::new());

/// Global kernel thread exit stub.
///
/// Invoked automatically when a kernel thread entry function returns normally.
#[no_mangle]
pub extern "C" fn kernel_thread_exit_stub() -> ! {
    exit_current(0);
}

/// Central Preemptive Priority-Based Scheduler.
pub struct Scheduler {
    run_queue: RunQueue,
    sleep_queue: SleepQueue,
    current_pid: ProcessId,
    idle_pid: ProcessId,
    remaining_ticks: u32,
    preempt_disable_depth: u32,
    total_context_switches: u64,
    total_preemptions: u64,
    total_yields: u64,
    active: bool,
}

impl Scheduler {
    /// Creates a dormant scheduler instance.
    pub const fn new() -> Self {
        Self {
            run_queue: RunQueue::new(),
            sleep_queue: SleepQueue::new(),
            current_pid: ProcessId::KERNEL,
            idle_pid: ProcessId::KERNEL,
            remaining_ticks: 5,
            preempt_disable_depth: 0,
            total_context_switches: 0,
            total_preemptions: 0,
            total_yields: 0,
            active: false,
        }
    }

    /// Prepares a context switch from `self.current_pid` to the next runnable process.
    ///
    /// Returns `Some((prev_rsp_ptr, next_rsp, next_cr3))` if a switch is necessary.
    pub fn prepare_reschedule(&mut self) -> Option<(*mut u64, u64, u64)> {
        if !self.active {
            return None;
        }

        let prev_pid = self.current_pid;

        // 1. Pick the next runnable task from the run queue
        let next_pid = match self.run_queue.dequeue() {
            Some(pid) => pid,
            None => {
                // If queue is empty, check if current process can keep running
                let pm = PROCESS_MANAGER.lock();
                let can_keep_running = pm.get(prev_pid).map(|p| p.is_runnable()).unwrap_or(false);
                drop(pm);
                if can_keep_running {
                    self.remaining_ticks = 5;
                    return None;
                }
                self.idle_pid
            }
        };

        // If next task is the same as current task, reset timeslice and return
        if next_pid == prev_pid {
            let pm = PROCESS_MANAGER.lock();
            let is_running = pm.get(prev_pid).map(|p| p.state == ProcessState::Running).unwrap_or(false);
            drop(pm);
            if is_running {
                self.remaining_ticks = 5;
                return None;
            }
        }

        // 2. Access PROCESS_MANAGER to update states and extract context pointers
        let mut pm = PROCESS_MANAGER.lock();

        // Update previous process state
        if let Some(prev_pcb) = pm.get_mut(prev_pid) {
            if prev_pcb.is_runnable() {
                prev_pcb.set_state(ProcessState::Ready);
                let prio = Priority::from_u8(prev_pcb.priority);
                self.run_queue.enqueue(prev_pid, prio);
            }
        }

        // Extract previous PCB's saved_rsp address
        let prev_rsp_ptr = match pm.get_mut(prev_pid) {
            Some(pcb) => core::ptr::addr_of_mut!(pcb.saved_rsp),
            None => return None,
        };

        // Update incoming process state
        let (next_rsp, next_cr3, next_prio) = match pm.get_mut(next_pid) {
            Some(next_pcb) => {
                next_pcb.set_state(ProcessState::Running);
                (
                    next_pcb.saved_rsp,
                    next_pcb.cr3,
                    Priority::from_u8(next_pcb.priority),
                )
            }
            None => return None,
        };

        pm.set_current_pid(next_pid);
        drop(pm);

        // 3. Update scheduler metadata
        self.current_pid = next_pid;
        self.remaining_ticks = next_prio.default_timeslice();
        self.total_context_switches += 1;

        Some((prev_rsp_ptr, next_rsp, next_cr3))
    }

    /// Handles periodic timer ticks for sleeping task wakeups and timeslice decrement.
    ///
    /// Returns `true` if current process timeslice expired and another task is ready.
    pub fn on_timer_tick(&mut self) -> bool {
        let current_tick = crate::arch::x86_64::pit::get_ticks();

        // 0. Account CPU tick to currently running process
        {
            let mut pm = PROCESS_MANAGER.lock();
            if let Some(pcb) = pm.get_mut(self.current_pid) {
                pcb.ticks_used = pcb.ticks_used.saturating_add(1);
            }
        }

        // 1. Wake expired sleeping processes
        let woken = self.sleep_queue.wake_expired(current_tick);
        if !woken.is_empty() {
            let mut pm = PROCESS_MANAGER.lock();
            for pid in woken {
                if let Some(pcb) = pm.get_mut(pid) {
                    pcb.set_state(ProcessState::Ready);
                    let prio = Priority::from_u8(pcb.priority);
                    self.run_queue.enqueue(pid, prio);
                }
            }
        }

        // 2. Check if preemption is permitted
        if self.preempt_disable_depth > 0 {
            return false;
        }

        if self.remaining_ticks > 0 {
            self.remaining_ticks -= 1;
        }

        if self.remaining_ticks == 0 {
            if !self.run_queue.is_empty() {
                return true;
            } else {
                self.remaining_ticks = 5;
            }
        }

        false
    }

    /// Returns the currently running process ID.
    pub fn current_pid(&self) -> ProcessId {
        self.current_pid
    }
}

/// Initializes the central scheduler subsystem.
pub fn init() {
    let mut sched = SCHEDULER.lock();
    sched.run_queue.clear();
    sched.sleep_queue.clear();
    sched.idle_pid = ProcessId::KERNEL;
    sched.current_pid = ProcessId::KERNEL;
    sched.remaining_ticks = 5;
    sched.preempt_disable_depth = 0;
    sched.active = true;
    drop(sched);

    klog_ok!("Scheduler Subsystem Initialized: Preemptive priority queue active (PIT 100 Hz)");
}

/// Registers a ready process with the scheduler.
pub fn register_task(pid: ProcessId, priority: Priority) {
    let mut sched = SCHEDULER.lock();
    sched.run_queue.enqueue(pid, priority);
    let mut pm = PROCESS_MANAGER.lock();
    if let Some(pcb) = pm.get_mut(pid) {
        pcb.set_state(ProcessState::Ready);
    }
}

/// Voluntarily yields the remainder of the current process's timeslice.
pub fn yield_now() {
    let switch_info = {
        let mut sched = SCHEDULER.lock();
        if !sched.active {
            return;
        }
        sched.total_yields += 1;
        sched.prepare_reschedule()
    };

    if let Some((prev_rsp_ptr, next_rsp, next_cr3)) = switch_info {
        // SAFETY: Pointers obtained from valid PCBs in PROCESS_MANAGER.
        unsafe {
            crate::process::switch::switch_context_asm(prev_rsp_ptr, next_rsp, next_cr3);
        }
    }
}

/// Wakes a blocked or waiting process, transitioning it to Ready and adding to the run queue.
pub fn wake_task(pid: ProcessId) {
    let mut pm = PROCESS_MANAGER.lock();
    if let Some(pcb) = pm.get_mut(pid) {
        let prio = Priority::from_u8(pcb.priority);
        pcb.set_state(ProcessState::Ready);
        drop(pm);
        let mut sched = SCHEDULER.lock();
        sched.run_queue.enqueue(pid, prio);
    }
}

/// Blocks the current process (sets state to Blocked) and yields the CPU.
pub fn block_current() {
    let switch_info = {
        let mut sched = SCHEDULER.lock();
        if !sched.active {
            return;
        }
        let curr = sched.current_pid;
        let mut pm = PROCESS_MANAGER.lock();
        if let Some(pcb) = pm.get_mut(curr) {
            pcb.set_state(ProcessState::Blocked);
        }
        drop(pm);
        sched.prepare_reschedule()
    };

    if let Some((prev_rsp_ptr, next_rsp, next_cr3)) = switch_info {
        // SAFETY: Pointers obtained from valid PCBs in PROCESS_MANAGER.
        unsafe {
            crate::process::switch::switch_context_asm(prev_rsp_ptr, next_rsp, next_cr3);
        }
    }
}

/// Suspends the current process for at least `ticks` timer ticks (100 Hz).
pub fn sleep_current(ticks: u64) {
    if ticks == 0 {
        yield_now();
        return;
    }

    let wake_tick = crate::arch::x86_64::pit::get_ticks() + ticks;
    let switch_info = {
        let mut sched = SCHEDULER.lock();
        if !sched.active {
            drop(sched);
            while crate::arch::x86_64::pit::get_ticks() < wake_tick {
                core::hint::spin_loop();
            }
            return;
        }
        let curr = sched.current_pid;
        sched.sleep_queue.add(curr, wake_tick);

        let mut pm = PROCESS_MANAGER.lock();
        if let Some(pcb) = pm.get_mut(curr) {
            pcb.set_state(ProcessState::Sleeping(wake_tick));
        }
        drop(pm);

        sched.prepare_reschedule()
    };

    if let Some((prev_rsp_ptr, next_rsp, next_cr3)) = switch_info {
        // SAFETY: Pointers obtained from valid PCBs in PROCESS_MANAGER.
        unsafe {
            crate::process::switch::switch_context_asm(prev_rsp_ptr, next_rsp, next_cr3);
        }
    } else {
        // If no other task is ready to run, wait until target tick
        while crate::arch::x86_64::pit::get_ticks() < wake_tick {
            core::hint::spin_loop();
        }
    }
}

/// Terminates the current process and yields CPU to the next ready task.
pub fn exit_current(exit_code: i32) -> ! {
    let switch_info = {
        let mut sched = SCHEDULER.lock();
        let curr = sched.current_pid;
        let mut pm = PROCESS_MANAGER.lock();
        pm.terminate(curr, exit_code);
        drop(pm);

        sched.prepare_reschedule()
    };

    if let Some((prev_rsp_ptr, next_rsp, next_cr3)) = switch_info {
        // SAFETY: Context switch away from terminated task.
        unsafe {
            crate::process::switch::switch_context_asm(prev_rsp_ptr, next_rsp, next_cr3);
        }
    }

    // Fallback parking if no other runnable tasks exist
    crate::halt_loop();
}

/// Invoked from the PIT timer interrupt handler to drive preemption.
pub fn check_timer_tick() {
    let need_reschedule = {
        let mut sched = SCHEDULER.lock();
        if !sched.active {
            return;
        }
        sched.on_timer_tick()
    };

    if need_reschedule {
        let switch_info = {
            let mut sched = SCHEDULER.lock();
            sched.total_preemptions += 1;
            sched.prepare_reschedule()
        };

        if let Some((prev_rsp_ptr, next_rsp, next_cr3)) = switch_info {
            // SAFETY: Context switch triggered by timeslice expiration.
            unsafe {
                crate::process::switch::switch_context_asm(prev_rsp_ptr, next_rsp, next_cr3);
            }
        }
    }
}

/// Disables scheduler preemption for the current execution thread.
pub fn preempt_disable() {
    SCHEDULER.lock().preempt_disable_depth += 1;
}

/// Re-enables scheduler preemption.
pub fn preempt_enable() {
    let mut sched = SCHEDULER.lock();
    if sched.preempt_disable_depth > 0 {
        sched.preempt_disable_depth -= 1;
    }
}

/// Returns the current scheduler statistics: (context_switches, preemptions, yields).
pub fn stats() -> (u64, u64, u64) {
    let sched = SCHEDULER.lock();
    (
        sched.total_context_switches,
        sched.total_preemptions,
        sched.total_yields,
    )
}

/// Returns the currently running process ID.
pub fn current_pid() -> ProcessId {
    SCHEDULER.lock().current_pid()
}

/// Sets the CPU affinity mask for the specified process.
pub fn sched_setaffinity(pid: ProcessId, mask: u64) -> Result<(), &'static str> {
    if mask == 0 {
        return Err("Invalid affinity mask: at least one CPU must be specified");
    }
    let mut pm = PROCESS_MANAGER.lock();
    let pcb = pm.get_mut(pid).ok_or("Process not found")?;
    pcb.set_cpu_affinity(mask);
    Ok(())
}

/// Retrieves the CPU affinity mask for the specified process.
pub fn sched_getaffinity(pid: ProcessId) -> Result<u64, &'static str> {
    let pm = PROCESS_MANAGER.lock();
    let pcb = pm.get(pid).ok_or("Process not found")?;
    Ok(pcb.get_cpu_affinity())
}
