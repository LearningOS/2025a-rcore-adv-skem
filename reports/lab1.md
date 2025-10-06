///user/src 用户调用接口
pub fn count_syscall(id: usize) -> isize {
    trace(TraceRequest::Syscall, id, 0)
}

///user/src 用户层库函数
pub fn trace(request: TraceRequest, id: usize, data: usize) -> isize {
    sys_trace(request as usize, id, data)
}

///os/src/syscall 内核层函数
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {

}

///user/src 用户层库函数
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    syscall(SYSCALL_TRACE, [trace_request, id, data])
}



///os/src/syscall 
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}


///user/src
pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}


### 关注对称性
用户程序：
    count_syscall(id)
        ↓
    trace(Syscall, id, 0)
        ↓
    sys_trace(2, id, 0)     # 用户态
        ↓
    syscall(SYSCALL_TRACE, [2, id, 0])
        ↓  (ecall 陷入)
──────────── 内核分界 ────────────
trap_handler() 捕获 ecall
    ↓
syscall(syscall_id=410, args=[2, id, 0])   # 内核分发器
    ↓
sys_trace(trace_request=2, id, 0)          # 内核实现
    ↓
返回调用次数
──────────── 返回用户态 ────────────
count_syscall() 获得结果
