use ::jni::{
    errors::Result,
    objects::{JMethodID, JObject},
    signature::JavaType,
    JNIEnv,
};
use std::{
    future::Future,
    pin::Pin,
    sync::MutexGuard,
    task::{Context, Poll, Waker},
};

pub struct JavaObjectFuture<'a: 'b, 'b> {
    internal: JObject<'a>,
    poll: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JavaObjectFuture<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/future/JavaObjectFuture")?);

        let poll = env.get_method_id(&class, "poll", "()Lgedgygedgy/rust/future/PollResult;")?;
        Ok(Self {
            internal: obj,
            poll,
            env,
        })
    }

    pub fn j_poll(&self) -> Result<Option<JObject<'a>>> {
        let result = self
            .env
            .call_method_unchecked(
                self.internal,
                self.poll,
                JavaType::Object("gedgygedgy/rust/future/PollResult".into()),
                &[],
            )?
            .l()?;

        Ok(if self.env.is_same_object(result, JObject::null())? {
            None
        } else {
            let poll_result = JPollResult::from_env(self.env, result)?;
            Some(poll_result.get()?)
        })
    }

    // Switch the Result and Poll return value to make this easier to implement using ?.
    fn poll_internal(&self, context: &mut Context<'_>) -> Result<Poll<JObject<'a>>> {
        Ok(if let Some(obj) = self.j_poll()? {
            Poll::Ready(obj)
        } else {
            let mut waker: MutexGuard<Option<Waker>> =
                self.env.get_rust_field(self.internal, "waker")?;
            *waker = Some(context.waker().clone());
            Poll::Pending
        })
    }
}

impl<'a: 'b, 'b> Future for JavaObjectFuture<'a, 'b> {
    type Output = Result<JObject<'a>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match (*self).poll_internal(context) {
            Ok(Poll::Ready(result)) => Poll::Ready(Ok(result)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

pub struct JPollResult<'a: 'b, 'b> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JPollResult<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/future/PollResult")?);

        let get = env.get_method_id(&class, "get", "()Ljava/lang/Object;")?;
        Ok(Self {
            internal: obj,
            get,
            env,
        })
    }

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

pub(crate) mod jni {
    use jni::{errors::Result, objects::JObject, JNIEnv, NativeMethod};
    use std::{ffi::c_void, sync::MutexGuard, task::Waker};

    fn native(name: &str, sig: &str, fn_ptr: *mut c_void) -> NativeMethod {
        NativeMethod {
            name: name.into(),
            sig: sig.into(),
            fn_ptr,
        }
    }

    extern "C" fn java_object_future_init_waker(env: JNIEnv, obj: JObject) {
        let field: Option<Waker> = None;
        let _ = env.set_rust_field(obj, "waker", field);
    }

    fn java_object_future_wake_impl(env: JNIEnv, obj: JObject) -> Result<()> {
        let waker_field: MutexGuard<Option<Waker>> = env.get_rust_field(obj, "waker")?;
        match &*waker_field {
            Some(waker) => waker.wake_by_ref(),
            None => (),
        }
        Ok(())
    }

    extern "C" fn java_object_future_wake_internal(env: JNIEnv, obj: JObject) {
        let _ = java_object_future_wake_impl(env, obj);
    }

    pub fn init(env: &JNIEnv) -> Result<()> {
        let class = env.find_class("gedgygedgy/rust/future/JavaObjectFuture")?;
        env.register_native_methods(
            class,
            &[
                native(
                    "initWaker",
                    "()V",
                    java_object_future_init_waker as *mut c_void,
                ),
                native(
                    "wakeInternal",
                    "()V",
                    java_object_future_wake_internal as *mut c_void,
                ),
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use jni::{InitArgsBuilder, JavaVM};
    use std::{env, path::PathBuf};

    fn make_vm() -> JavaVM {
        let mut jni_utils_jar = PathBuf::from(env::current_exe().unwrap());
        jni_utils_jar.pop();
        jni_utils_jar.pop();
        jni_utils_jar.push("java");
        jni_utils_jar.push("libs");
        jni_utils_jar.push("jni-utils.jar");

        let args = InitArgsBuilder::new()
            .option(&format!(
                "-Djava.class.path={}",
                jni_utils_jar.to_str().unwrap()
            ))
            .build()
            .unwrap();
        let vm = JavaVM::new(args).unwrap();

        let env = vm.attach_current_thread_permanently().unwrap();
        crate::init(&env).unwrap();

        vm
    }

    #[test]
    fn test_init() {
        let vm = make_vm();
    }
}
