use ::jni::{
    errors::Result,
    objects::{JMethodID, JObject},
    signature::JavaType,
    JNIEnv,
};
use std::task::Waker;

/// Wraps the given waker in a `io.github.gedgygedgy.rust.task.Waker` object.
///
/// Calling this function is generally not necessary, since
/// [`JFuture`](crate::future::JFuture) and [`JStream`](crate::stream::JStream)
/// take care of it for you.
///
/// # Arguments
///
/// * `env` - Java environment in which to create the object.
/// * `waker` - Waker to wrap in a Java object.
pub fn waker<'a: 'b, 'b>(env: &'b JNIEnv<'a>, waker: Waker) -> Result<JObject<'a>> {
    let class = env.find_class("io/github/gedgygedgy/rust/task/Waker")?;

    let obj = env.new_object(class, "()V", &[])?;
    env.set_rust_field(obj, "data", waker)?;
    Ok(obj)
}

/// Wrapper for [`JObject`]s that implement
/// `io.github.gedgygedgy.rust.task.PollResult`. Provides method to get the
/// poll result.
///
/// Looks up the class and method IDs on creation rather than for every method
/// call.
pub struct JPollResult<'a: 'b, 'b> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JPollResult<'a, 'b> {
    /// Create a [`JPollResult`] from the environment and an object. This looks
    /// up the necessary class and method IDs to call all of the methods on it
    /// so that extra work doesn't need to be done on every method call.
    ///
    /// # Arguments
    ///
    /// * `env` - Java environment to use.
    /// * `obj` - Object to wrap.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("io/github/gedgygedgy/rust/task/PollResult")?);

        let get = env.get_method_id(&class, "get", "()Ljava/lang/Object;")?;
        Ok(Self {
            internal: obj,
            get,
            env,
        })
    }

    /// Gets the object associated with the [`JPollResult`] by calling
    /// `io.github.gedgygedgy.rust.task.PollResult.get()`. Can throw an
    /// exception.
    pub fn get(&self) -> Result<JObject<'a>> {
        self.env
            .call_method_unchecked(
                self.internal,
                self.get,
                JavaType::Object("java/lang/Object".into()),
                &[],
            )?
            .l()
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JPollResult<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JPollResult<'a, 'b>> for JObject<'a> {
    fn from(other: JPollResult<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

pub(crate) mod jni {
    use jni::{errors::Result, objects::JObject, JNIEnv};
    use std::{ffi::c_void, sync::MutexGuard, task::Waker};

    fn waker_wake_impl(env: JNIEnv, obj: JObject) -> Result<()> {
        use jni::errors::Error;

        let result: Result<MutexGuard<Waker>> = env.get_rust_field(obj, "data");
        match result {
            Ok(waker) => waker.wake_by_ref(),
            Err(Error::NullPtr(_)) => env.throw_new(
                "java/lang/NullPointerException",
                "This Waker has already been finalized",
            )?,
            Err(_) => (),
        }
        Ok(())
    }

    extern "C" fn waker_wake(env: JNIEnv, obj: JObject) {
        let _ = waker_wake_impl(env, obj);
    }

    extern "C" fn waker_finalize(env: JNIEnv, obj: JObject) {
        let _: Result<Waker> = env.take_rust_field(obj, "data");
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.find_class("io/github/gedgygedgy/rust/task/Waker")?;
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
        use jni::{errors::Error, objects::JString};

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

        if let Error::JavaException = env.call_method(jwaker, "wake", "()V", &[]).unwrap_err() {
        } else {
            panic!("Second wake() should have thrown an exception")
        }
        let ex = env.exception_occurred().unwrap();
        env.exception_clear().unwrap();

        let class = env.get_object_class(ex).unwrap();
        let null_ptr_ex = env.find_class("java/lang/NullPointerException").unwrap();
        assert!(env.is_same_object(class, null_ptr_ex).unwrap());

        let message: JString = env
            .call_method(ex, "getMessage", "()Ljava/lang/String;", &[])
            .unwrap()
            .l()
            .unwrap()
            .into();
        let message_str = env.get_string(message).unwrap();
        assert_eq!(
            message_str.to_str().unwrap(),
            "This Waker has already been finalized"
        );

        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), true);
    }
}
