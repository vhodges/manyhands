//! Fault/race boundaries for the source-included, isolated SSH fixture runner.
#![allow(dead_code)]
use super::RemoteOperationSafePoint;
use std::cell::RefCell;
type Hook = Box<dyn FnMut(RemoteOperationSafePoint)>;
thread_local! { static HOOK: RefCell<Option<Hook>> = RefCell::new(None); }
pub(crate) fn checkpoint(point: RemoteOperationSafePoint) {
    HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(point);
        }
    });
}
pub(crate) struct HookGuard;
impl Drop for HookGuard {
    fn drop(&mut self) {
        HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}
pub(crate) fn install_hook(hook: impl FnMut(RemoteOperationSafePoint) + 'static) -> HookGuard {
    HOOK.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    HookGuard
}
