//! Phase 31 — Kernel Synchronization Verification Suite
//!
//! Validates:
//! 1. Atomic primitives & Spinlocks (CAS, fetch_add, `Spinlock`, `IrqSpinlock`, `try_lock`)
//! 2. Reader-Writer Lock (`RwLock`) concurrency and mutual exclusion
//! 3. Kernel Sleeping Mutex (`KernelMutex`) & cooperative yielding
//! 4. Counting Semaphore resource tracking & permit bounded capacity
//! 5. Task Wait Queue, Condition Variable, OnceCell, and Barrier synchronization

use core::sync::atomic::{AtomicU64, Ordering};

use crate::klog_info;
use crate::process::pid::ProcessId;
use crate::sync::barrier::Barrier;
use crate::sync::condvar::Condvar;
use crate::sync::mutex::KernelMutex;
use crate::sync::once::OnceCell;
use crate::sync::rwlock::RwLock;
use crate::sync::semaphore::CountingSemaphore;
use crate::sync::spinlock::{push_and_disable_interrupts, restore_interrupts, IrqSpinlock, Spinlock};
use crate::sync::wait_queue::WaitQueue;

/// Executes the full Phase 31 kernel synchronization verification suite.
pub fn run_sync_tests() {
    klog_info!("=== Phase 31: Kernel Synchronization Tests ===");

    test_atomics_and_spinlocks();
    test_rwlock_concurrency();
    test_kernel_mutex();
    test_counting_semaphore();
    test_wait_queue_condvar_once();

    klog_info!("=== Phase 31: All Kernel Synchronization Tests Passed (5/5) ===");
}

/// Test 1: Atomic primitives & Spinlocks (CAS, fetch_add, `Spinlock`, `IrqSpinlock`, `try_lock`).
fn test_atomics_and_spinlocks() {
    klog_info!("Sync Test 1: Atomic Primitives & Spinlocks...");

    // 1. Atomic CAS and arithmetic operations
    let counter = AtomicU64::new(100);
    assert_eq!(counter.fetch_add(50, Ordering::SeqCst), 100);
    assert_eq!(counter.load(Ordering::SeqCst), 150);

    let cas_res = counter.compare_exchange(150, 200, Ordering::AcqRel, Ordering::Relaxed);
    assert_eq!(cas_res, Ok(150));
    assert_eq!(counter.load(Ordering::SeqCst), 200);

    let cas_fail = counter.compare_exchange(999, 300, Ordering::AcqRel, Ordering::Relaxed);
    assert_eq!(cas_fail, Err(200));

    // 2. Standard Spinlock
    let spin = Spinlock::new(42);
    {
        let mut guard = spin.lock();
        assert_eq!(*guard, 42);
        *guard += 10;
        assert!(spin.is_locked());
    }
    assert!(!spin.is_locked());
    assert_eq!(*spin.lock(), 52);

    // 3. Spinlock try_lock
    {
        let guard = spin.lock();
        assert!(spin.try_lock().is_none());
        drop(guard);
    }
    let try_guard = spin.try_lock();
    assert!(try_guard.is_some());
    drop(try_guard);

    // 4. IrqSpinlock (interrupt disabling verification)
    let irq_spin = IrqSpinlock::new(1000);
    {
        let guard = irq_spin.lock();
        assert_eq!(*guard, 1000);
        assert!(irq_spin.is_locked());

        // In critical section, interrupts must be disabled
        let was_enabled = push_and_disable_interrupts();
        restore_interrupts(was_enabled);
        assert!(!was_enabled, "Interrupts must be disabled inside IrqSpinlock critical section");

        assert!(irq_spin.try_lock().is_none());
    }
    assert!(!irq_spin.is_locked());

    klog_info!("Sync Test 1: PASS — Atomic CAS, Spinlock & IrqSpinlock mutual exclusion verified");
}

/// Test 2: Reader-Writer Lock (`RwLock`) concurrency and mutual exclusion.
fn test_rwlock_concurrency() {
    klog_info!("Sync Test 2: Reader-Writer Lock (RwLock) Concurrency...");

    let rw = RwLock::new(777);

    // 1. Multiple concurrent readers
    let r1 = rw.try_read();
    let r2 = rw.try_read();
    let r3 = rw.try_read();
    let r4 = rw.try_read();

    assert!(r1.is_some() && r2.is_some() && r3.is_some() && r4.is_some());
    assert_eq!(rw.reader_count(), 4);
    assert_eq!(*r1.unwrap(), 777);

    // 2. Writer cannot acquire while readers exist
    assert!(rw.try_write().is_none(), "Writer must be rejected when readers are active");
    assert!(!rw.is_write_locked());

    // 3. Drop all readers
    drop(r2);
    drop(r3);
    drop(r4);
    assert_eq!(rw.reader_count(), 0);

    // 4. Exclusive writer acquisition
    {
        let mut w = rw.write();
        assert!(rw.is_write_locked());
        assert_eq!(*w, 777);
        *w = 888;

        // No readers or other writers can acquire while write lock is held
        assert!(rw.try_read().is_none());
        assert!(rw.try_write().is_none());
    }

    // 5. After writer drops, readers can read updated value
    assert!(!rw.is_write_locked());
    let reader_after = rw.read();
    assert_eq!(*reader_after, 888);
    drop(reader_after);

    klog_info!("Sync Test 2: PASS — Multi-reader concurrency & exclusive writer isolation verified");
}

/// Test 3: Kernel Sleeping Mutex (`KernelMutex`) & cooperative yielding.
fn test_kernel_mutex() {
    klog_info!("Sync Test 3: Kernel Sleeping Mutex (KernelMutex)...");

    let mutex = KernelMutex::new(500);

    // 1. Basic lock & mutate
    {
        let mut guard = mutex.lock();
        assert!(mutex.is_locked());
        assert_eq!(*guard, 500);
        *guard += 250;
    }
    assert!(!mutex.is_locked());

    // 2. Verify updated value
    {
        let guard = mutex.lock();
        assert_eq!(*guard, 750);
        // try_lock must fail while locked
        assert!(mutex.try_lock().is_none());
    }

    // 3. try_lock succeeds when unlocked
    let try_guard = mutex.try_lock();
    assert!(try_guard.is_some());
    drop(try_guard);
    assert!(!mutex.is_locked());

    klog_info!("Sync Test 3: PASS — KernelMutex mutual exclusion & RAII lifecycle verified");
}

/// Test 4: Counting Semaphore resource tracking & permit bounded capacity.
fn test_counting_semaphore() {
    klog_info!("Sync Test 4: Counting Semaphore Resource Tracking...");

    let sem = CountingSemaphore::new(3, 3);
    assert_eq!(sem.available_permits(), 3);
    assert_eq!(sem.max_permits(), 3);

    // 1. Acquire permits until exhausted
    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 2);

    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 1);

    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 0);

    // 2. Acquisition fails when 0 permits remain
    assert!(!sem.try_acquire());

    // 3. Release 1 permit and re-acquire
    assert!(sem.release());
    assert_eq!(sem.available_permits(), 1);
    assert!(sem.try_acquire());
    assert_eq!(sem.available_permits(), 0);

    // 4. Release all permits back to max capacity
    assert!(sem.release());
    assert!(sem.release());
    assert!(sem.release());
    assert_eq!(sem.available_permits(), 3);

    // 5. Exceeding max capacity is rejected
    assert!(!sem.release());
    assert_eq!(sem.available_permits(), 3);

    klog_info!("Sync Test 4: PASS — CountingSemaphore permits, exhaustion & bounded capacity verified");
}

/// Test 5: Task Wait Queue, Condition Variable, OnceCell, and Barrier synchronization.
fn test_wait_queue_condvar_once() {
    klog_info!("Sync Test 5: Wait Queue, Condvar, OnceCell & Barrier...");

    // 1. WaitQueue FIFO operations
    let wq = WaitQueue::new();
    assert!(wq.is_empty());
    assert_eq!(wq.len(), 0);

    let pid1 = ProcessId(8001);
    let pid2 = ProcessId(8002);
    let pid3 = ProcessId(8003);

    wq.wait_pid(pid1);
    wq.wait_pid(pid2);
    wq.wait_pid(pid3);
    assert_eq!(wq.len(), 3);

    // Wake one
    let woken1 = wq.wake_one();
    assert_eq!(woken1, Some(pid1));
    assert_eq!(wq.len(), 2);

    // Wake remaining all
    let woken_all = wq.wake_all();
    assert_eq!(woken_all, 2);
    assert!(wq.is_empty());

    // 2. Condition Variable notification
    let cv = Condvar::new();
    assert_eq!(cv.waiter_count(), 0);
    assert!(!cv.notify_one()); // No waiters
    assert_eq!(cv.notify_all(), 0);

    // 3. OnceCell lazy initialization
    let cell: OnceCell<&str> = OnceCell::new();
    assert!(cell.get().is_none());

    let val = cell.get_or_init(|| "IntegrateOS_Kernel_Sync_Verified");
    assert_eq!(*val, "IntegrateOS_Kernel_Sync_Verified");
    assert_eq!(cell.get(), Some(&"IntegrateOS_Kernel_Sync_Verified"));

    // Setting an already initialized cell is rejected
    let second_set = cell.set("NewValue");
    assert_eq!(second_set, Err("NewValue"));
    assert_eq!(cell.get(), Some(&"IntegrateOS_Kernel_Sync_Verified"));

    // 4. Barrier rendezvous & automatic reset
    let barrier = Barrier::new(1);
    let b1 = barrier.wait();
    assert!(b1.is_leader(), "First participant in barrier of 1 must be the leader");

    // Barrier auto-resets generation and count for subsequent cycles
    let b2 = barrier.wait();
    assert!(b2.is_leader(), "Second cycle participant must also be the leader");

    klog_info!("Sync Test 5: PASS — WaitQueue, Condvar, OnceCell & Barrier verified");
}
