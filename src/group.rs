use std::time::Duration;

use libc::c_void;

use super::object::DispatchObject;
use super::queue::Queue;
use super::utils::function_wrapper;
use super::{ffi::*, WaitError};

#[derive(Debug, Clone)]
pub struct Group {
    dispatch_object: DispatchObject<dispatch_group_s>,
}

#[derive(Debug)]
pub struct GroupGuard(Group, bool);

impl Group {
    pub fn new() -> Option<Self> {
        let object = unsafe { dispatch_group_create() };

        if object.is_null() {
            return None;
        }

        let dispatch_object = unsafe {
            // Safety: object cannot be null.
            DispatchObject::new_owned(object as *mut _)
        };

        Some(Group { dispatch_object })
    }

    pub fn exec_async<F>(&self, queue: &Queue, work: F)
    where
        F: Send + FnOnce(),
    {
        let work_boxed = Box::leak(Box::new(work)) as *mut _ as *mut c_void;

        unsafe {
            // Safety: All parameters cannot be null.
            dispatch_group_async_f(
                self.as_raw(),
                queue.as_raw(),
                work_boxed,
                function_wrapper::<F>,
            );
        }
    }

    pub fn wait(&self, timeout: Option<Duration>) -> Result<(), WaitError> {
        let timeout = if let Some(timeout) = timeout {
            dispatch_time_t::try_from(timeout).map_err(|_| WaitError::TimeOverflow)?
        } else {
            DISPATCH_TIME_FOREVER
        };

        let result = unsafe { dispatch_group_wait(self.as_raw(), timeout) };

        match result {
            0 => Ok(()),
            _ => Err(WaitError::Timeout),
        }
    }

    pub fn notify<F>(&self, queue: &Queue, work: F)
    where
        F: Send + FnOnce(),
    {
        let work_boxed = Box::leak(Box::new(work)) as *mut _ as *mut c_void;

        unsafe {
            // Safety: All parameters cannot be null.
            dispatch_group_notify_f(
                self.as_raw(),
                queue.as_raw(),
                work_boxed,
                function_wrapper::<F>,
            );
        }
    }

    pub fn enter(&self) -> GroupGuard {
        unsafe {
            dispatch_group_enter(self.as_raw());
        }

        GroupGuard(self.clone(), false)
    }

    pub fn set_finalizer<F>(&mut self, destructor: F)
    where
        F: Send + FnOnce(),
    {
        self.dispatch_object.set_finalizer(destructor);
    }

    pub const fn as_raw(&self) -> dispatch_group_t {
        self.dispatch_object.as_raw()
    }
}

impl GroupGuard {
    pub fn leave(mut self) {
        unsafe {
            dispatch_group_leave(self.0.as_raw());
        }

        self.1 = true;
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        if !self.1 {
            unsafe {
                dispatch_group_leave(self.0.as_raw());
            }

            self.1 = true;
        }
    }
}
