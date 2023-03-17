
mod switch;
mod context;
#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;

pub use task::{TaskControlBlock, TaskStatus};
use switch::__switch;
use context::TaskContext;

use lazy_static::*;




pub struct TaskManager{
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner{
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

pub struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize
}

lazy_static!{
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [
            TaskControlBlock{
                task_cx: TaskContext::zero_init(),
                task_status: TaskStatus::UnInit,
                task_start_time: 0,
                task_syscall_times: [0;MAX_SYSCALL_NUM],
            };
            MAX_APP_NUM
        ];
        for i in 0..num_app{
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager{
            num_app,
            inner: unsafe{
                UPSafeCell::new(TaskManagerInner{
                    tasks,
                    current_task: 0,
            })},
        }
    };
}

pub fn suspend_current_and_run_next(){
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next(){
    mark_current_exited();
    run_next_task();
}

pub fn run_first_task(){
    TASK_MANAGER.run_first_task();
}

pub fn increase_syscall_time(syscall_id: usize){
    TASK_MANAGER.increase_syscall_time(syscall_id);
}

pub fn get_task_info(ti: *mut TaskInfo)->isize{
    if TASK_MANAGER.get_task_info(ti) == 0{
        return 0
    }
    return -1

}




fn mark_current_suspended(){
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited(){
    TASK_MANAGER.mark_current_exited();
}

fn run_next_task(){
    TASK_MANAGER.run_next_task();
}

impl TaskManager{
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self){
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn run_next_task(&self){
        if let Some(next) = self.find_next_task(){
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            if inner.tasks[next].task_start_time==0 {
                inner.tasks[next].task_start_time = get_time_us();
            }
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            
            unsafe{
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            };
        }else {
            panic!("All applications completed!");
        }
    }

    fn run_first_task(&self) -> !{
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        task0.task_start_time = get_time_us();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);

        let mut _unused = TaskContext::zero_init();
        unsafe{
            __switch(
                &mut _unused as *mut TaskContext,
                next_task_cx_ptr,
            );
        }
        panic!("unreachable is run_first_task!");
    }

    fn find_next_task(&self) -> Option<usize>{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current +1..current + self.num_app+1)
            .map(|id| id % self.num_app)
            .find(|id|{
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    fn increase_syscall_time(&self, syscall_id: usize){
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_syscall_times[syscall_id] +=1;
        drop(inner);
    }

    fn get_task_info(&self, ti:*mut TaskInfo) -> isize{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let task_start_time = inner.tasks[current].task_start_time;
        let syscall_times = inner.tasks[current].task_syscall_times;
        drop(inner);
        unsafe{
            *ti = TaskInfo{
                status: TaskStatus::Running,
                syscall_times,
                time: (get_time_us() - task_start_time)/1000,
            }
        };
        
        return 0
    }
}