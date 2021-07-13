use ::jni::{errors::Result, objects::JObject, JNIEnv};

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

    let class = env.find_class("io/github/gedgygedgy/rust/ops/FnOnceRunnable")?;

    let obj = env.new_object(class, "()V", &[])?;
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

pub(crate) mod jni {
    use super::FnOnceWrapper;
    use jni::{errors::Result, objects::JObject, JNIEnv};
    use std::ffi::c_void;

    extern "C" fn fn_once_run(env: JNIEnv, obj: JObject) {
        let _ = (|| {
            if let Ok(f) = env.take_rust_field::<_, _, FnOnceWrapper>(obj, "data") {
                f.0(&env);
            }
        })();
    }

    extern "C" fn fn_once_close(env: JNIEnv, obj: JObject) {
        let _ = env.take_rust_field::<_, _, FnOnceWrapper>(obj, "data");
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.find_class("io/github/gedgygedgy/rust/ops/FnOnceRunnable")?;
        env.register_native_methods(
            class,
            &[
                crate::jni::native("run", "()V", fn_once_run as *mut c_void),
                crate::jni::native("close", "()V", fn_once_close as *mut c_void),
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
        Box<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>) + Send + 'static>,
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
}
