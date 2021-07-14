use ::jni::{errors::Result, objects::JObject, JNIEnv};
use std::sync::Arc;

struct SendSyncWrapper<T>(T);

unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

type FnOnceWrapper = SendSyncWrapper<Box<dyn for<'a, 'b> FnOnce(&'b JNIEnv<'a>) + 'static>>;

/// Create an `io.github.gedgygedgy.rust.ops.FnOnceRunnable` from a given
/// [`FnOnce`] without checking if it is [`Send`].
///
/// # Safety
///
/// This is unsafe because it could allow non-[`Send`] functions to be sent to
/// another thread. Calling code is responsible for making sure that the
/// resulting object does not have its `run()` or `close()` methods called from
/// any thread except the current thread.
pub unsafe fn fn_once_runnable_unchecked<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnOnce(&'d JNIEnv<'c>) + 'static,
) -> Result<JObject<'a>> {
    let boxed: Box<dyn for<'c, 'd> FnOnce(&'d JNIEnv<'c>)> = Box::from(f);

    let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnOnceRunnable")?);

    let obj = env.new_object(&class, "()V", &[])?;
    env.set_rust_field::<_, _, FnOnceWrapper>(obj, "data", SendSyncWrapper(boxed))?;
    Ok(obj)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnOnceRunnable` from a given
/// [`FnOnce`]. The function can later be called by calling the object's
/// `run()` method. The function can be freed without calling it by calling
/// the object's `close()` method.
pub fn fn_once_runnable<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnOnce(&'d JNIEnv<'c>) + Send + 'static,
) -> Result<JObject<'a>> {
    unsafe { fn_once_runnable_unchecked(env, f) }
}

type FnWrapper = SendSyncWrapper<Arc<dyn for<'a, 'b> Fn(&'b JNIEnv<'a>) + 'static>>;

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given [`Fn`]
/// without checking if it is [`Send`] or [`Sync`].
///
/// # Safety
///
/// This is unsafe because it could allow non-[`Send`] or non-[`Sync`]
/// functions to be sent to another thread. Calling code is responsible for
/// making sure that the resulting object does not have its `run()` or
/// `close()` methods called from any thread except the current thread.
pub unsafe fn fn_runnable_unchecked<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> Fn(&'d JNIEnv<'c>) + 'static,
) -> Result<JObject<'a>> {
    let arc: Arc<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>)> = Arc::from(f);

    let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnRunnable")?);

    let obj = env.new_object(&class, "()V", &[])?;
    env.set_rust_field::<_, _, FnWrapper>(obj, "data", SendSyncWrapper(arc))?;
    Ok(obj)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given [`Fn`].
/// The function can later be called by calling the object's `run()` method.
/// The function can be freed without calling it by calling the object's
/// `close()` method.
pub fn fn_runnable<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> Fn(&'d JNIEnv<'c>) + Send + Sync + 'static,
) -> Result<JObject<'a>> {
    unsafe { fn_runnable_unchecked(env, f) }
}

pub(crate) mod jni {
    use super::{FnOnceWrapper, FnWrapper};
    use jni::{errors::Result, objects::JObject, JNIEnv};
    use std::ffi::c_void;

    extern "C" fn fn_once_run(env: JNIEnv, obj: JObject) {
        if let Ok(f) = env.take_rust_field::<_, _, FnOnceWrapper>(obj, "data") {
            f.0(&env);
        }
    }

    extern "C" fn fn_once_close(env: JNIEnv, obj: JObject) {
        let _ = env.take_rust_field::<_, _, FnOnceWrapper>(obj, "data");
    }

    extern "C" fn fn_run(env: JNIEnv, obj: JObject) {
        let arc = if let Ok(f) = env.get_rust_field::<_, _, FnWrapper>(obj, "data") {
            f.0.clone()
        } else {
            return;
        };
        arc(&env);
    }

    extern "C" fn fn_close(env: JNIEnv, obj: JObject) {
        let _ = env.take_rust_field::<_, _, FnWrapper>(obj, "data");
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnOnceRunnable")?);
        env.register_native_methods(
            &class,
            &[
                crate::jni::native("run", "()V", fn_once_run as *mut c_void),
                crate::jni::native("close", "()V", fn_once_close as *mut c_void),
            ],
        )?;

        let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnRunnable")?);
        env.register_native_methods(
            &class,
            &[
                crate::jni::native("run", "()V", fn_run as *mut c_void),
                crate::jni::native("close", "()V", fn_close as *mut c_void),
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils;
    use jni::JNIEnv;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    fn create_test_fn<'a: 'b, 'b>() -> (
        Arc<Mutex<u32>>,
        Box<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>) + Send + Sync + 'static>,
    ) {
        let arc = Arc::new(Mutex::new(0));
        let arc2 = arc.clone();
        (
            arc,
            Box::new(move |_e| {
                let mut guard = arc2.lock().unwrap();
                *&mut *guard += 1;
            }),
        )
    }

    fn create_test_fn_unchecked<'a: 'b, 'b>() -> (
        Rc<RefCell<u32>>,
        Box<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>) + 'static>,
    ) {
        let rc = Rc::new(RefCell::new(0));
        let rc2 = rc.clone();
        (
            rc,
            Box::new(move |_e| {
                let mut guard = rc2.try_borrow_mut().unwrap();
                *&mut *guard += 1;
            }),
        )
    }

    fn test_data(data: &Arc<Mutex<u32>>, expected: u32, expected_refcount: usize) {
        assert_eq!(Arc::strong_count(data), expected_refcount);
        let guard = data.lock().unwrap();
        assert_eq!(*guard, expected);
    }

    fn test_data_unchecked(data: &Rc<RefCell<u32>>, expected: u32, expected_refcount: usize) {
        assert_eq!(Rc::strong_count(data), expected_refcount);
        let guard = data.try_borrow().unwrap();
        assert_eq!(*guard, expected);
    }

    #[test]
    fn test_fn_once_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_once_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 1, 1);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 1, 1);
    }

    #[test]
    fn test_fn_once_close() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_once_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);
    }

    #[test]
    fn test_fn_once_run_close() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_once_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 1, 1);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 1, 1);
    }

    #[test]
    fn test_fn_once_close_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_once_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 0, 1);
    }

    #[test]
    fn test_fn_once_unchecked_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn_unchecked();
        test_data_unchecked(&data, 0, 2);

        let runnable = unsafe { super::fn_once_runnable_unchecked(env, f) }.unwrap();
        test_data_unchecked(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data_unchecked(&data, 1, 1);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data_unchecked(&data, 1, 1);
    }

    #[test]
    fn test_fn_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 1, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 2, 2);
    }

    #[test]
    fn test_fn_close() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);
    }

    #[test]
    fn test_fn_run_close() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 1, 2);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 1, 1);
    }

    #[test]
    fn test_fn_close_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn();
        test_data(&data, 0, 2);

        let runnable = super::fn_runnable(env, f).unwrap();
        test_data(&data, 0, 2);

        env.call_method(runnable, "close", "()V", &[]).unwrap();
        test_data(&data, 0, 1);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data(&data, 0, 1);
    }

    #[test]
    fn test_fn_unchecked_run() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let (data, f) = create_test_fn_unchecked();
        test_data_unchecked(&data, 0, 2);

        let runnable = unsafe { super::fn_runnable_unchecked(env, f) }.unwrap();
        test_data_unchecked(&data, 0, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data_unchecked(&data, 1, 2);

        env.call_method(runnable, "run", "()V", &[]).unwrap();
        test_data_unchecked(&data, 2, 2);
    }
}
