//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next, TASK_MANAGER},
    timer::get_time_us,
};
// use crate::syscall::SYSCALL_TRACE;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
/// sys_trace 系统调用实现
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    match trace_request {
        // case 0: 从用户地址读取一个字节
        0 => {
            let ptr = id as *const u8;
            let value = unsafe { *ptr };
            value as isize
        }

        // case 1: 向用户地址写入一个字节
        1 => {
            let ptr = id as *mut u8;
            let byte = (data & 0xFF) as u8;
            unsafe { *ptr = byte; }
            0
        }

        // case 2: 查询当前任务的 syscall[id] 调用次数
        2 => {
             let current = TASK_MANAGER.get_current_task_id();
            // ⚠️ 不再自增，外层 syscall() 已经统计过一次
            let count = TASK_MANAGER.get_syscall_count(current, id);
            count as isize
        }

        _ => -1,
    }
}