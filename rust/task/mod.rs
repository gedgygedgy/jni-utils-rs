use ::jni::{errors::Result, objects::JObject, JNIEnv};
use std::task::Waker;

pub fn waker<'a: 'b, 'b>(env: &'b JNIEnv<'a>, waker: Waker) -> Result<JObject<'a>> {
    let class = env.find_class("gedgygedgy/rust/task/Waker")?;

    let obj = env.new_object(class, "()V", &[])?;
    env.set_rust_field(obj, "data", waker)?;
    Ok(obj)
}

pub(crate) mod jni {
    use jni::{errors::Result, objects::JObject, JNIEnv};
    use std::{ffi::c_void, sync::MutexGuard, task::Waker};

    fn waker_wake_impl(env: JNIEnv, obj: JObject) -> Result<()> {
        let waker: MutexGuard<Waker> = env.get_rust_field(obj, "data")?;
        waker.wake_by_ref();
        Ok(())
    }

    extern "C" fn waker_wake(env: JNIEnv, obj: JObject) {
        let _ = waker_wake_impl(env, obj);
    }

    extern "C" fn waker_finalize(env: JNIEnv, obj: JObject) {
        let _: Result<Waker> = env.take_rust_field(obj, "data");
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.find_class("gedgygedgy/rust/task/Waker")?;
        env.register_native_methods(
            class,
            &[
                crate::jni::native("wake", "()V", waker_wake as *mut c_void),
                crate::jni::native("finalize", "()V", waker_finalize as *mut c_void),
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_waker() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let data = Arc::new(Mutex::new(false));
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), false);

        let waker = crate::test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        let jwaker = super::waker(env, waker).unwrap();
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        env.call_method(jwaker, "wake", "()V", &[]).unwrap();
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), true);

        env.call_method(jwaker, "finalize", "()V", &[]).unwrap();
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), true);
    }
}
