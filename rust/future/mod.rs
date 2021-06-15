use ::jni::{
    errors::Result,
    objects::{JMethodID, JObject},
    signature::JavaType,
    JNIEnv,
};
use std::{
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
            "(Lgedgygedgy/rust/task/Waker;)Lgedgygedgy/rust/task/Poll;",
        )?;
        Ok(Self {
            internal: obj,
            poll,
            env,
        })
    }

    pub fn j_poll(&self, waker: JObject<'a>) -> Result<Option<JObject<'a>>> {
        let result = self
            .env
            .call_method_unchecked(
                self.internal,
                self.poll,
                JavaType::Object("gedgygedgy/rust/task/Poll".into()),
                &[waker.into()],
            )?
            .l()?;

        Ok(if self.env.is_same_object(result, JObject::null())? {
            None
        } else {
            let poll = JPoll::from_env(self.env, result)?;
            Some(poll.get()?)
        })
    }

    // Switch the Result and Poll return value to make this easier to implement using ?.
    fn poll_internal(&self, context: &mut Context<'_>) -> Result<Poll<JObject<'a>>> {
        use crate::task::waker;

        Ok(
            if let Some(obj) = self.j_poll(waker(self.env, context.waker().clone())?)? {
                Poll::Ready(obj)
            } else {
                Poll::Pending
            },
        )
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
    type Output = Result<JObject<'a>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match (*self).poll_internal(context) {
            Ok(Poll::Ready(result)) => Poll::Ready(Ok(result)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

pub struct JPoll<'a: 'b, 'b> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JPoll<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/task/Poll")?);

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

#[cfg(test)]
mod test {
    use super::JFuture;
    use crate::test_utils;
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    #[test]
    fn test_future() {
        use std::sync::{Arc, Mutex};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let data = Arc::new(Mutex::new(false));
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), false);

        let waker = test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        let mut future = JFuture::from_env(
            env,
            env.new_object("gedgygedgy/rust/future/Future", "()V", &[])
                .unwrap(),
        )
        .unwrap();

        assert!(Future::poll(Pin::new(&mut future), &mut Context::from_waker(&waker)).is_pending());
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), false);

        let obj = env.new_object("java/lang/Object", "()V", &[]).unwrap();
        env.call_method(*future, "wake", "(Ljava/lang/Object;)V", &[obj.into()])
            .unwrap();
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), true);

        let poll = Future::poll(Pin::new(&mut future), &mut Context::from_waker(&waker));
        if let Poll::Ready(result) = poll {
            assert!(env.is_same_object(result.unwrap(), obj).unwrap());
        } else {
            panic!("Poll result should be ready");
        }
    }

    #[test]
    fn test_future_await() {
        use futures::{executor::block_on, join};
        use jni::{objects::JObject, JNIEnv};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let future = JFuture::from_env(
            env,
            env.new_object("gedgygedgy/rust/future/Future", "()V", &[])
                .unwrap(),
        )
        .unwrap();
        let obj = env.new_object("java/lang/Object", "()V", &[]).unwrap();

        async fn future_wake<'a: 'b, 'b>(
            env: &'b JNIEnv<'a>,
            future_obj: JObject<'a>,
            obj: JObject<'a>,
        ) {
            env.call_method(future_obj, "wake", "(Ljava/lang/Object;)V", &[obj.into()])
                .unwrap();
        }

        async fn future_get<'a: 'b, 'b>(
            env: &'b JNIEnv<'a>,
            future: JFuture<'a, 'b>,
            obj: JObject<'a>,
        ) {
            assert!(env.is_same_object(future.await.unwrap(), obj).unwrap());
        }

        async fn future_join<'a: 'b, 'b>(
            env: &'b JNIEnv<'a>,
            future: JFuture<'a, 'b>,
            obj: JObject<'a>,
        ) {
            join!(future_wake(env, *future, obj), future_get(env, future, obj));
        }

        block_on(future_join(env, future, obj));
    }
}
