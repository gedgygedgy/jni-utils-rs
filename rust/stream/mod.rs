use crate::task::JPoll;
use ::jni::{
    errors::Result,
    objects::{JMethodID, JObject},
    signature::JavaType,
    JNIEnv,
};
use futures::stream::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct JStream<'a: 'b, 'b> {
    internal: JObject<'a>,
    poll_next: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JStream<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/stream/Stream")?);

        let poll_next = env.get_method_id(
            &class,
            "pollNext",
            "(Lgedgygedgy/rust/task/Waker;)Lgedgygedgy/rust/task/Poll;",
        )?;
        Ok(Self {
            internal: obj,
            poll_next,
            env,
        })
    }

    pub fn j_poll_next(&self, waker: JObject<'a>) -> Result<Poll<Option<JObject<'a>>>> {
        let result = self
            .env
            .call_method_unchecked(
                self.internal,
                self.poll_next,
                JavaType::Object("gedgygedgy/rust/task/Poll".into()),
                &[waker.into()],
            )?
            .l()?;
        Ok(if self.env.is_same_object(result, JObject::null())? {
            Poll::Pending
        } else {
            Poll::Ready({
                let poll = JPoll::from_env(self.env, result)?;
                let stream_poll_obj = poll.get()?;
                if self.env.is_same_object(stream_poll_obj, JObject::null())? {
                    None
                } else {
                    let stream_poll = JStreamPoll::from_env(self.env, stream_poll_obj)?;
                    Some(stream_poll.get()?)
                }
            })
        })
    }

    // Switch the Result and Poll return value to make this easier to implement using ?.
    fn poll_next_internal(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Result<Poll<Option<JObject<'a>>>> {
        use crate::task::waker;
        self.j_poll_next(waker(self.env, context.waker().clone())?)
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JStream<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JStream<'a, 'b>> for JObject<'a> {
    fn from(other: JStream<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> Stream for JStream<'a, 'b> {
    type Item = Result<JObject<'a>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        match self.poll_next_internal(context) {
            Ok(Poll::Ready(result)) => Poll::Ready(result.map(|o| Ok(o))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }
}

struct JStreamPoll<'a: 'b, 'b> {
    internal: JObject<'a>,
    get: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JStreamPoll<'a, 'b> {
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.auto_local(env.find_class("gedgygedgy/rust/stream/StreamPoll")?);

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
    use super::JStream;
    use crate::test_utils;
    use futures::stream::Stream;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    #[test]
    fn test_jstream() {
        use std::sync::{Arc, Mutex};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let data = Arc::new(Mutex::new(false));
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), false);

        let waker = test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        let waker_obj = env
            .call_static_method(
                "gedgygedgy/rust/stream/Stream",
                "create",
                "()Lgedgygedgy/rust/stream/Stream$Waker;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();
        let mut stream = JStream::from_env(
            env,
            env.call_method(
                waker_obj,
                "getStream",
                "()Lgedgygedgy/rust/stream/Stream;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap(),
        )
        .unwrap();

        assert!(Pin::new(&mut stream)
            .poll_next(&mut Context::from_waker(&waker))
            .is_pending());
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), false);

        let obj1 = env.new_object("java/lang/Object", "()V", &[]).unwrap();
        env.call_method(waker_obj, "add", "(Ljava/lang/Object;)V", &[obj1.into()])
            .unwrap();
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), true);
        *data.lock().unwrap() = false;

        let obj2 = env.new_object("java/lang/Object", "()V", &[]).unwrap();
        env.call_method(waker_obj, "add", "(Ljava/lang/Object;)V", &[obj2.into()])
            .unwrap();
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), true);
        *data.lock().unwrap() = false;

        let poll = Pin::new(&mut stream).poll_next(&mut Context::from_waker(&waker));
        if let Poll::Ready(Some(Ok(actual_obj1))) = poll {
            assert!(env.is_same_object(actual_obj1, obj1).unwrap());
        } else {
            panic!("Poll result should be ready");
        }
        assert_eq!(Arc::strong_count(&data), 4);
        assert_eq!(*data.lock().unwrap(), false);

        let poll = Pin::new(&mut stream).poll_next(&mut Context::from_waker(&waker));
        if let Poll::Ready(Some(Ok(actual_obj2))) = poll {
            assert!(env.is_same_object(actual_obj2, obj2).unwrap());
        } else {
            panic!("Poll result should be ready");
        }
        assert_eq!(Arc::strong_count(&data), 5);
        assert_eq!(*data.lock().unwrap(), false);

        assert!(Pin::new(&mut stream)
            .poll_next(&mut Context::from_waker(&waker))
            .is_pending());
        assert_eq!(Arc::strong_count(&data), 6);
        assert_eq!(*data.lock().unwrap(), false);

        env.call_method(waker_obj, "finish", "()V", &[]).unwrap();
        assert_eq!(Arc::strong_count(&data), 6);
        assert_eq!(*data.lock().unwrap(), true);
        *data.lock().unwrap() = false;

        let poll = Pin::new(&mut stream).poll_next(&mut Context::from_waker(&waker));
        if let Poll::Ready(None) = poll {
        } else {
            panic!("Poll result should be ready");
        }
        assert_eq!(Arc::strong_count(&data), 7);
        assert_eq!(*data.lock().unwrap(), false);
    }

    #[test]
    fn test_jstream_await() {
        use futures::{executor::block_on, join};

        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let waker_obj = env
            .call_static_method(
                "gedgygedgy/rust/stream/Stream",
                "create",
                "()Lgedgygedgy/rust/stream/Stream$Waker;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();
        let mut stream = JStream::from_env(
            env,
            env.call_method(
                waker_obj,
                "getStream",
                "()Lgedgygedgy/rust/stream/Stream;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap(),
        )
        .unwrap();
        let obj1 = env.new_object("java/lang/Object", "()V", &[]).unwrap();
        let obj2 = env.new_object("java/lang/Object", "()V", &[]).unwrap();

        block_on(async {
            join!(
                async {
                    env.call_method(waker_obj, "add", "(Ljava/lang/Object;)V", &[obj1.into()])
                        .unwrap();
                    env.call_method(waker_obj, "add", "(Ljava/lang/Object;)V", &[obj2.into()])
                        .unwrap();
                    env.call_method(waker_obj, "finish", "()V", &[]).unwrap();
                },
                async {
                    use futures::StreamExt;
                    assert!(env
                        .is_same_object(stream.next().await.unwrap().unwrap(), obj1)
                        .unwrap());
                    assert!(env
                        .is_same_object(stream.next().await.unwrap().unwrap(), obj2)
                        .unwrap());
                    assert!(stream.next().await.is_none());
                }
            );
        });
    }
}
