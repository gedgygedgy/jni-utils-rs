use ::jni::{errors::Result, objects::JObject, JNIEnv};
use std::sync::{Arc, Mutex};

struct SendSyncWrapper<T>(T);

unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

fn fn_once_runnable_internal<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnOnce(&'d JNIEnv<'c>, JObject<'c>) + 'static,
    local: bool,
) -> Result<JObject<'a>> {
    let mutex = Mutex::new(Some(f));
    fn_runnable_internal(
        env,
        move |env, obj| {
            let f = {
                let mut guard = mutex.lock().unwrap();
                if let Some(f) = guard.take() {
                    f
                } else {
                    return;
                }
            };
            f(env, obj)
        },
        local,
    )
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given
/// [`FnOnce`] without checking if it is [`Send`]. Attempting to call `run()`
/// or `close()` on the resulting object from a thread other than its origin
/// thread will result in an
/// `io.github.gedgygedgy.rust.thread.LocalThreadException` being thrown.
pub fn fn_once_runnable_local<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnOnce(&'d JNIEnv<'c>, JObject<'c>) + 'static,
) -> Result<JObject<'a>> {
    fn_once_runnable_internal(env, f, true)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given
/// [`FnOnce`]. The function can later be called by calling the object's
/// `run()` method. The function can be freed without calling it by calling
/// the object's `close()` method.
///
/// If the closure panics, the unwind will be caught and thrown as an
/// `io.github.gedgygedgy.rust.panic.PanicException`.
///
/// It is safe to call the object's `run()` method recursively, but the second
/// call will be a no-op.
pub fn fn_once_runnable<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnOnce(&'d JNIEnv<'c>, JObject<'c>) + Send + 'static,
) -> Result<JObject<'a>> {
    fn_once_runnable_internal(env, f, false)
}

fn fn_mut_runnable_internal<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnMut(&'d JNIEnv<'c>, JObject<'c>) + 'static,
    local: bool,
) -> Result<JObject<'a>> {
    let mutex = Mutex::new(f);
    fn_runnable_internal(
        env,
        move |env, obj| {
            let mut guard = mutex.lock().unwrap();
            guard(env, obj)
        },
        local,
    )
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given
/// [`FnMut`] without checking if it is [`Send`]. Attempting to call `run()`
/// or `close()` on the resulting object from a thread other than its origin
/// thread will result in an
/// `io.github.gedgygedgy.rust.thread.LocalThreadException` being thrown.
pub fn fn_mut_runnable_local<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnMut(&'d JNIEnv<'c>, JObject<'c>) + 'static,
) -> Result<JObject<'a>> {
    fn_mut_runnable_internal(env, f, true)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given
/// [`FnMut`]. The function can later be called by calling the object's
/// `run()` method. The function can be freed without calling it by calling
/// the object's `close()` method.
///
/// If the closure panics, the unwind will be caught and thrown as an
/// `io.github.gedgygedgy.rust.panic.PanicException`.
///
/// Unlike [`fn_runnable`] and [`fn_once_runnable`], it is not safe to call the
/// resulting object's `run()` method recursively. The [`FnMut`] is managed
/// with an internal [`Mutex`], so calling `run()` recursively will result in a
/// deadlock.
pub fn fn_mut_runnable<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> FnMut(&'d JNIEnv<'c>, JObject<'c>) + Send + 'static,
) -> Result<JObject<'a>> {
    fn_mut_runnable_internal(env, f, false)
}

type FnWrapper = SendSyncWrapper<Arc<dyn for<'a, 'b> Fn(&'b JNIEnv<'a>, JObject<'a>) + 'static>>;

fn fn_runnable_internal<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) + 'static,
    local: bool,
) -> Result<JObject<'a>> {
    let arc: Arc<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>)> = Arc::from(f);

    let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnRunnableImpl")?);

    let obj = env.new_object(&class, "(Z)V", &[local.into()])?;
    env.set_rust_field::<_, _, FnWrapper>(obj, "data", SendSyncWrapper(arc))?;
    Ok(obj)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given [`Fn`]
/// without checking if it is [`Send`] or [`Sync`]. Attempting to call `run()`
/// or `close()` on the resulting object from a thread other than its origin
/// thread will result in an
/// `io.github.gedgygedgy.rust.thread.LocalThreadException` being thrown.
pub fn fn_runnable_local<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) + 'static,
) -> Result<JObject<'a>> {
    fn_runnable_internal(env, f, true)
}

/// Create an `io.github.gedgygedgy.rust.ops.FnRunnable` from a given [`Fn`].
/// The function can later be called by calling the object's `run()` method.
/// The function can be freed without calling it by calling the object's
/// `close()` method.
///
/// If the closure panics, the unwind will be caught and thrown as an
/// `io.github.gedgygedgy.rust.panic.PanicException`.
///
/// It is safe to call the object's `run()` method recursively.
pub fn fn_runnable<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    f: impl for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) + Send + Sync + 'static,
) -> Result<JObject<'a>> {
    fn_runnable_internal(env, f, false)
}

pub(crate) mod jni {
    use super::FnWrapper;
    use jni::{errors::Result, objects::JObject, JNIEnv, NativeMethod};

    extern "C" fn fn_run_internal(env: JNIEnv, obj: JObject) {
        use std::panic::AssertUnwindSafe;

        let arc = if let Ok(f) = env.get_rust_field::<_, _, FnWrapper>(obj, "data") {
            AssertUnwindSafe(f.0.clone())
        } else {
            return;
        };
        let _ = crate::exceptions::throw_unwind(&env, || arc(&env, obj));
    }

    extern "C" fn fn_close_internal(env: JNIEnv, obj: JObject) {
        let _ = env.take_rust_field::<_, _, FnWrapper>(obj, "data");
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        use std::ffi::c_void;

        let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/ops/FnRunnableImpl")?);
        env.register_native_methods(
            &class,
            &[
                NativeMethod {
                    name: "runInternal".into(),
                    sig: "()V".into(),
                    fn_ptr: fn_run_internal as *mut c_void,
                },
                NativeMethod {
                    name: "closeInternal".into(),
                    sig: "()V".into(),
                    fn_ptr: fn_close_internal as *mut c_void,
                },
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils;
    use jni::{objects::JObject, JNIEnv};
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    fn create_test_fn<'a: 'b, 'b>() -> (
        Arc<Mutex<u32>>,
        Box<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) + Send + Sync + 'static>,
    ) {
        let arc = Arc::new(Mutex::new(0));
        let arc2 = arc.clone();
        (
            arc,
            Box::new(move |_e, _o| {
                let mut guard = arc2.lock().unwrap();
                *&mut *guard += 1;
            }),
        )
    }

    fn create_test_fn_local<'a: 'b, 'b>() -> (
        Rc<RefCell<u32>>,
        Box<dyn for<'c, 'd> Fn(&'d JNIEnv<'c>, JObject<'c>) + 'static>,
    ) {
        let rc = Rc::new(RefCell::new(0));
        let rc2 = rc.clone();
        (
            rc,
            Box::new(move |_e, _o| {
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

    fn test_data_local(data: &Rc<RefCell<u32>>, expected: u32, expected_refcount: usize) {
        assert_eq!(Rc::strong_count(data), expected_refcount);
        let guard = data.try_borrow().unwrap();
        assert_eq!(*guard, expected);
    }

    #[test]
    fn test_fn_once_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_once_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 1);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_once_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_once_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_once_run_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_once_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 1);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_once_close_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_once_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_once_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_once_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_once_object() {
        test_utils::JVM_ENV.with(|env| {
            let obj_ref = Arc::new(Mutex::new(env.new_global_ref(JObject::null()).unwrap()));
            let obj_ref_2 = obj_ref.clone();
            let runnable = super::fn_once_runnable(env, move |e, o| {
                let guard = obj_ref_2.lock().unwrap();
                assert!(e.is_same_object(guard.as_obj(), o).unwrap());
            })
            .unwrap();

            {
                let mut guard = obj_ref.lock().unwrap();
                *guard = env.new_global_ref(runnable).unwrap();
            }

            env.call_method(runnable, "run", "()V", &[]).unwrap();
        });
    }

    #[test]
    fn test_fn_once_recursive() {
        test_utils::JVM_ENV.with(|env| {
            let arc = Arc::new(Mutex::new(false));
            let arc2 = arc.clone();

            let runnable = super::fn_once_runnable(env, move |env, obj| {
                let value = {
                    let mut guard = arc2.lock().unwrap();
                    let old = *guard;
                    *guard = true;
                    old
                };
                if !value {
                    env.call_method(obj, "run", "()V", &[]).unwrap();
                }
            })
            .unwrap();

            env.call_method(runnable, "run", "()V", &[]).unwrap();

            let guard = arc.lock().unwrap();
            assert!(*guard);
        })
    }

    #[test]
    fn test_fn_once_panic() {
        test_utils::JVM_ENV.with(|env| {
            let runnable =
                super::fn_once_runnable(env, |_e, _o| panic!("This is a panic")).unwrap();
            if let jni::errors::Error::JavaException =
                env.call_method(runnable, "run", "()V", &[]).unwrap_err()
            {
            } else {
                panic!("JavaException not found");
            }

            assert!(env.exception_check().unwrap());
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env
                .is_instance_of(ex, "io/github/gedgygedgy/rust/panic/PanicException")
                .unwrap());

            let ex = crate::exceptions::JPanicException::from_env(env, ex).unwrap();
            let any = ex.take().unwrap();
            let str = any.downcast::<&str>().unwrap();
            assert_eq!(*str, "This is a panic");
        });
    }

    #[test]
    fn test_fn_once_local_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_once_runnable_local(env, f).unwrap();
            test_data_local(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 1, 1);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_once_local_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_once_runnable_local(env, f).unwrap();
            let runnable = env.new_global_ref(runnable).unwrap();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_runnable(env, move |env, _obj| {
                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "run", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);

                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "close", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);
            })
            .unwrap();

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data_local(&data, 0, 2);
        });
    }

    #[test]
    fn test_fn_mut_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_mut_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 2, 2);
        });
    }

    #[test]
    fn test_fn_mut_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_mut_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_mut_run_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_mut_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_mut_close_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_mut_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_mut_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_mut_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data(&data, 1, 2);
        });
    }

    #[test]
    fn test_fn_mut_object() {
        test_utils::JVM_ENV.with(|env| {
            let obj_ref = Arc::new(Mutex::new(env.new_global_ref(JObject::null()).unwrap()));
            let obj_ref_2 = obj_ref.clone();
            let runnable = super::fn_mut_runnable(env, move |e, o| {
                let guard = obj_ref_2.lock().unwrap();
                assert!(e.is_same_object(guard.as_obj(), o).unwrap());
            })
            .unwrap();

            {
                let mut guard = obj_ref.lock().unwrap();
                *guard = env.new_global_ref(runnable).unwrap();
            }

            env.call_method(runnable, "run", "()V", &[]).unwrap();
        });
    }

    #[test]
    fn test_fn_mut_panic() {
        test_utils::JVM_ENV.with(|env| {
            let runnable = super::fn_mut_runnable(env, |_e, _o| panic!("This is a panic")).unwrap();
            if let jni::errors::Error::JavaException =
                env.call_method(runnable, "run", "()V", &[]).unwrap_err()
            {
            } else {
                panic!("JavaException not found");
            }

            assert!(env.exception_check().unwrap());
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env
                .is_instance_of(ex, "io/github/gedgygedgy/rust/panic/PanicException")
                .unwrap());

            let ex = crate::exceptions::JPanicException::from_env(env, ex).unwrap();
            let any = ex.take().unwrap();
            let str = any.downcast::<&str>().unwrap();
            assert_eq!(*str, "This is a panic");
        });
    }

    #[test]
    fn test_fn_mut_local_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_mut_runnable_local(env, f).unwrap();
            test_data_local(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 1, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 2, 2);
        });
    }

    #[test]
    fn test_fn_mut_local_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_mut_runnable_local(env, f).unwrap();
            let runnable = env.new_global_ref(runnable).unwrap();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_runnable(env, move |env, _obj| {
                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "run", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);

                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "close", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);
            })
            .unwrap();

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data_local(&data, 0, 2);
        });
    }

    #[test]
    fn test_fn_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 2, 2);
        });
    }

    #[test]
    fn test_fn_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_run_close() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 1, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 1, 1);
        });
    }

    #[test]
    fn test_fn_close_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            env.call_method(runnable, "close", "()V", &[]).unwrap();
            test_data(&data, 0, 1);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data(&data, 0, 1);
        });
    }

    #[test]
    fn test_fn_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn();
            test_data(&data, 0, 2);

            let runnable = super::fn_runnable(env, f).unwrap();
            test_data(&data, 0, 2);

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data(&data, 1, 2);
        });
    }

    #[test]
    fn test_fn_object() {
        test_utils::JVM_ENV.with(|env| {
            let obj_ref = Arc::new(Mutex::new(env.new_global_ref(JObject::null()).unwrap()));
            let obj_ref_2 = obj_ref.clone();
            let runnable = super::fn_runnable(env, move |e, o| {
                let guard = obj_ref_2.lock().unwrap();
                assert!(e.is_same_object(guard.as_obj(), o).unwrap());
            })
            .unwrap();

            {
                let mut guard = obj_ref.lock().unwrap();
                *guard = env.new_global_ref(runnable).unwrap();
            }

            env.call_method(runnable, "run", "()V", &[]).unwrap();
        });
    }

    #[test]
    fn test_fn_recursive() {
        test_utils::JVM_ENV.with(|env| {
            let arc = Arc::new(Mutex::new(false));
            let arc2 = arc.clone();

            let calling = Mutex::new(false);

            let runnable = super::fn_runnable(env, move |env, obj| {
                let calling_value = {
                    let mut guard = calling.lock().unwrap();
                    let old = *guard;
                    *guard = true;
                    old
                };
                if !calling_value {
                    env.call_method(obj, "run", "()V", &[]).unwrap();
                    let mut guard = calling.lock().unwrap();
                    *guard = false;
                } else {
                    let mut guard = arc2.lock().unwrap();
                    *guard = true;
                }
            })
            .unwrap();

            env.call_method(runnable, "run", "()V", &[]).unwrap();

            let guard = arc.lock().unwrap();
            assert!(*guard);
        })
    }

    #[test]
    fn test_fn_panic() {
        test_utils::JVM_ENV.with(|env| {
            let runnable = super::fn_runnable(env, |_e, _o| panic!("This is a panic")).unwrap();
            if let jni::errors::Error::JavaException =
                env.call_method(runnable, "run", "()V", &[]).unwrap_err()
            {
            } else {
                panic!("JavaException not found");
            }

            assert!(env.exception_check().unwrap());
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env
                .is_instance_of(ex, "io/github/gedgygedgy/rust/panic/PanicException")
                .unwrap());

            let ex = crate::exceptions::JPanicException::from_env(env, ex).unwrap();
            let any = ex.take().unwrap();
            let str = any.downcast::<&str>().unwrap();
            assert_eq!(*str, "This is a panic");
        });
    }

    #[test]
    fn test_fn_local_run() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_runnable_local(env, f).unwrap();
            test_data_local(&data, 0, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 1, 2);

            env.call_method(runnable, "run", "()V", &[]).unwrap();
            test_data_local(&data, 2, 2);
        });
    }

    #[test]
    fn test_fn_local_thread() {
        test_utils::JVM_ENV.with(|env| {
            let (data, f) = create_test_fn_local();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_runnable_local(env, f).unwrap();
            let runnable = env.new_global_ref(runnable).unwrap();
            test_data_local(&data, 0, 2);

            let runnable = super::fn_runnable(env, move |env, _obj| {
                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "run", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);

                let value = crate::exceptions::try_block(env, || {
                    env.call_method(runnable.as_obj(), "close", "()V", &[])?;
                    Ok(false)
                })
                .catch(
                    "io/github/gedgygedgy/rust/thread/LocalThreadException",
                    |_ex| Ok(true),
                )
                .result()
                .unwrap();
                assert!(value);
            })
            .unwrap();

            let thread = env
                .new_object(
                    "java/lang/Thread",
                    "(Ljava/lang/Runnable;)V",
                    &[runnable.into()],
                )
                .unwrap();
            env.call_method(thread, "start", "()V", &[]).unwrap();
            env.call_method(thread, "join", "()V", &[]).unwrap();
            test_data_local(&data, 0, 2);
        });
    }
}
