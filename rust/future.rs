use crate::task::JPollResult;
use ::jni::{
    errors::{Error, Result},
    objects::{GlobalRef, JMethodID, JObject},
    signature::JavaType,
    JNIEnv, JavaVM,
};
use static_assertions::assert_impl_all;
use std::{
    convert::TryFrom,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct JFuture<'a: 'b, 'b> {
    internal: JObject<'a>,
    poll: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JFuture<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/future/Future")?);

        let poll = env.get_method_id(
            &class,
            "poll",
            "(Lgedgygedgy/rust/task/Waker;)Lgedgygedgy/rust/task/PollResult;",
        )?;
        Ok(Self {
            internal: obj,
            poll,
            env,
        })
    }

    pub fn j_poll(&self, waker: JObject<'a>) -> Result<Poll<JPollResult<'a, 'b>>> {
        let result = self
            .env
            .call_method_unchecked(
                self.internal,
                self.poll,
                JavaType::Object("gedgygedgy/rust/task/PollResult".into()),
                &[waker.into()],
            )?
            .l()?;
        Ok(if self.env.is_same_object(result, JObject::null())? {
            Poll::Pending
        } else {
            let poll = JPollResult::from_env(self.env, result)?;
            Poll::Ready(poll)
        })
    }

    // Switch the Result and Poll return value to make this easier to implement using ?.
    fn poll_internal(&self, context: &mut Context<'_>) -> Result<Poll<JPollResult<'a, 'b>>> {
        use crate::task::waker;
        self.j_poll(waker(self.env, context.waker().clone())?)
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JFuture<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JFuture<'a, 'b>> for JObject<'a> {
    fn from(other: JFuture<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> Future for JFuture<'a, 'b> {
    type Output = Result<JPollResult<'a, 'b>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_internal(context) {
            Ok(Poll::Ready(result)) => Poll::Ready(Ok(result)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

pub struct JavaFuture {
    internal: GlobalRef,
    vm: JavaVM,
}

impl<'a: 'b, 'b> TryFrom<JFuture<'a, 'b>> for JavaFuture {
    type Error = Error;

    fn try_from(future: JFuture<'a, 'b>) -> Result<Self> {
        Ok(JavaFuture {
            internal: future.env.new_global_ref(future.internal)?,
            vm: future.env.get_java_vm()?,
        })
    }
}

impl ::std::ops::Deref for JavaFuture {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl JavaFuture {
    fn poll_internal(&self, context: &mut Context<'_>) -> Result<Poll<Result<GlobalRef>>> {
        let env = self.vm.get_env()?;
        let jfuture = JFuture::from_env(&env, self.internal.as_obj())?;
        jfuture
            .poll_internal(context)
            .map(|result| result.map(|result| Ok(env.new_global_ref(result)?)))
    }
}

impl Future for JavaFuture {
    type Output = Result<GlobalRef>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_internal(context) {
            Ok(result) => result,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

assert_impl_all!(JavaFuture: Send);

#[cfg(test)]
mod test {
    use super::{JFuture, JavaFuture};
    use crate::{task::JPollResult, test_utils};
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    #[test]
    fn test_jfuture() {
        use std::sync::{Arc, Mutex};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let data = Arc::new(Mutex::new(false));
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), false);

        let waker = test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        let future_obj = env
            .new_object("gedgygedgy/rust/future/SimpleFuture", "()V", &[])
            .unwrap();
        let mut future = JFuture::from_env(env, future_obj).unwrap();

        assert!(Future::poll(Pin::new(&mut future), &mut Context::from_waker(&waker)).is_pending());
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), false);

        let obj = env.new_object("java/lang/Object", "()V", &[]).unwrap();
        env.call_method(future_obj, "wake", "(Ljava/lang/Object;)V", &[obj.into()])
            .unwrap();
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), true);

        let poll = Future::poll(Pin::new(&mut future), &mut Context::from_waker(&waker));
        if let Poll::Ready(result) = poll {
            assert!(env
                .is_same_object(result.unwrap().get().unwrap(), obj)
                .unwrap());
        } else {
            panic!("Poll result should be ready");
        }
    }

    #[test]
    fn test_jfuture_await() {
        use futures::{executor::block_on, join};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let future_obj = env
            .new_object("gedgygedgy/rust/future/SimpleFuture", "()V", &[])
            .unwrap();
        let future = JFuture::from_env(env, future_obj).unwrap();
        let obj = env.new_object("java/lang/Object", "()V", &[]).unwrap();

        block_on(async {
            join!(
                async {
                    env.call_method(future_obj, "wake", "(Ljava/lang/Object;)V", &[obj.into()])
                        .unwrap();
                },
                async {
                    assert!(env
                        .is_same_object(future.await.unwrap().get().unwrap(), obj)
                        .unwrap());
                }
            );
        });
    }

    #[test]
    fn test_jfuture_await_throw() {
        use futures::{executor::block_on, join};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let future_obj = env
            .new_object("gedgygedgy/rust/future/SimpleFuture", "()V", &[])
            .unwrap();
        let future = JFuture::from_env(env, future_obj).unwrap();
        let ex = env.new_object("java/lang/Exception", "()V", &[]).unwrap();

        block_on(async {
            join!(
                async {
                    env.call_method(
                        future_obj,
                        "wakeWithThrowable",
                        "(Ljava/lang/Throwable;)V",
                        &[ex.into()],
                    )
                    .unwrap();
                },
                async {
                    future.await.unwrap().get().unwrap_err();
                    let future_ex = env.exception_occurred().unwrap();
                    env.exception_clear().unwrap();
                    let actual_ex = env
                        .call_method(future_ex, "getCause", "()Ljava/lang/Throwable;", &[])
                        .unwrap()
                        .l()
                        .unwrap();
                    assert!(env.is_same_object(actual_ex, ex).unwrap());
                }
            );
        });
    }

    #[test]
    fn test_java_future_await() {
        use futures::{executor::block_on, join};
        use std::convert::TryInto;

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let future_obj = env
            .new_object("gedgygedgy/rust/future/SimpleFuture", "()V", &[])
            .unwrap();
        let future = JFuture::from_env(env, future_obj).unwrap();
        let future: JavaFuture = future.try_into().unwrap();
        let obj = env.new_object("java/lang/Object", "()V", &[]).unwrap();

        block_on(async {
            join!(
                async {
                    env.call_method(future_obj, "wake", "(Ljava/lang/Object;)V", &[obj.into()])
                        .unwrap();
                },
                async {
                    let global_ref = future.await.unwrap();
                    let jpoll = JPollResult::from_env(env, global_ref.as_obj()).unwrap();
                    assert!(env.is_same_object(jpoll.get().unwrap(), obj).unwrap());
                }
            );
        });
    }
}
