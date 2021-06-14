use ::jni::{errors::Result, objects::JObject, JNIEnv};
use std::task::Waker;

pub fn waker<'a: 'b, 'b>(env: &'b JNIEnv<'a>, waker: Waker) -> Result<JObject<'a>> {
    let class = env.auto_local(env.find_class("gedgygedgy/rust/task/Waker")?);

    let obj = env.new_object(class.as_obj(), "()V", &[])?;
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

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.find_class("gedgygedgy/rust/task/Waker")?;
        env.register_native_methods(
            class,
            &[crate::jni::native("wake", "()V", waker_wake as *mut c_void)],
        )?;
        Ok(())
    }
}
