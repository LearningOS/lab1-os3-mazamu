use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Copy,Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub task_start_time: usize,
    pub task_syscall_times: [u32; MAX_SYSCALL_NUM],
}