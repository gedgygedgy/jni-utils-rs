use jni::{
    descriptors::Desc,
    errors::Error,
    objects::{JClass, JThrowable},
    JNIEnv,
};

/// Result from [`try_block`]. This object can be chained into
/// [`catch`](TryCatchResult::catch) calls to catch exceptions. When finished
/// with the try/catch sequence, the result can be obtained from
/// [`result`](TryCatchResult::result).
pub struct TryCatchResult<'a: 'b, 'b, T> {
    env: &'b JNIEnv<'a>,
    try_result: Result<Result<T, Error>, Error>,
    catch_result: Option<Result<T, Error>>,
}

/// Attempt to execute a block of JNI code. If the code causes an exception
/// to be thrown, it will be stored in the resulting [`TryCatchResult`] for
/// matching with [`catch`](TryCatchResult::catch). If an exception was already
/// being thrown before [`try_block`] is called, the given block will not be
/// executed, nor will any of the [`catch`](TryCatchResult::catch) blocks.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `block` - Block of JNI code to run.
pub fn try_block<'a: 'b, 'b, T>(
    env: &'b JNIEnv<'a>,
    block: impl FnOnce() -> Result<T, Error>,
) -> TryCatchResult<'a, 'b, T> {
    TryCatchResult {
        env,
        try_result: (|| {
            if env.exception_check()? {
                Err(Error::JavaException)
            } else {
                Ok(block())
            }
        })(),
        catch_result: None,
    }
}

impl<'a: 'b, 'b, T> TryCatchResult<'a, 'b, T> {
    /// Attempt to catch an exception thrown by [`try_block`]. If the thrown
    /// exception matches the given class, the block is executed. If no
    /// exception was thrown by [`try_block`], or if the exception does not
    /// match the given class, the block is not executed.
    ///
    /// # Arguments
    ///
    /// * `class` - Exception class to match.
    /// * `block` - Block of JNI code to run.
    pub fn catch(
        self,
        class: impl Desc<'a, JClass<'a>>,
        block: impl FnOnce(JThrowable<'a>) -> Result<T, Error>,
    ) -> Self {
        match (self.try_result, self.catch_result) {
            (Err(e), _) => Self {
                env: self.env,
                try_result: Err(e),
                catch_result: None,
            },
            (Ok(Ok(r)), _) => Self {
                env: self.env,
                try_result: Ok(Ok(r)),
                catch_result: None,
            },
            (Ok(Err(e)), Some(r)) => Self {
                env: self.env,
                try_result: Ok(Err(e)),
                catch_result: Some(r),
            },
            (Ok(Err(Error::JavaException)), None) => {
                let env = self.env;
                let catch_result = (|| {
                    if env.exception_check()? {
                        let ex = env.exception_occurred()?;
                        env.exception_clear()?;
                        if env.is_instance_of(ex, class)? {
                            return block(ex).map(|o| Some(o));
                        }
                        env.throw(ex)?;
                    }
                    Ok(None)
                })()
                .transpose();
                Self {
                    env,
                    try_result: Ok(Err(Error::JavaException)),
                    catch_result,
                }
            }
            (Ok(Err(e)), None) => Self {
                env: self.env,
                try_result: Ok(Err(e)),
                catch_result: None,
            },
        }
    }

    /// Get the result of the try/catch sequence. If [`try_block`] succeeded,
    /// or if one of the [`catch`](TryCatchResult::catch) calls succeeded, its
    /// result is returned.
    pub fn result(self) -> Result<T, Error> {
        match (self.try_result, self.catch_result) {
            (Err(e), _) => Err(e),
            (Ok(Ok(r)), _) => Ok(r),
            (Ok(Err(_)), Some(r)) => r,
            (Ok(Err(e)), None) => Err(e),
        }
    }
}

#[cfg(test)]
mod test {
    use jni::{errors::Error, objects::JThrowable, JNIEnv};

    use super::try_block;
    use crate::test_utils;

    fn test_catch<'a: 'b, 'b>(
        env: &'b JNIEnv<'a>,
        throw_class: Option<&str>,
        try_result: Result<i32, Error>,
        rethrow: bool,
    ) -> Result<i32, Error> {
        let old_ex = if env.exception_check().unwrap() {
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            Some(ex)
        } else {
            None
        };
        let illegal_argument_exception = env
            .find_class("java/lang/IllegalArgumentException")
            .unwrap();
        if let Some(ex) = old_ex {
            env.throw(ex).unwrap();
        }

        let ex = throw_class.map(|c| {
            let ex: JThrowable = env.new_object(c, "()V", &[]).unwrap().into();
            ex
        });

        try_block(env, || {
            if let Some(t) = ex {
                env.throw(t).unwrap();
            }
            try_result
        })
        .catch(illegal_argument_exception, |caught| {
            assert!(!env.exception_check().unwrap());
            assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
            Ok(1)
        })
        .catch("java/lang/ArrayIndexOutOfBoundsException", |caught| {
            assert!(!env.exception_check().unwrap());
            assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
            if rethrow {
                Err(Error::JavaException)
            } else {
                Ok(2)
            }
        })
        .catch("java/lang/IndexOutOfBoundsException", |caught| {
            assert!(!env.exception_check().unwrap());
            assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
            if rethrow {
                env.throw(caught).unwrap();
                Err(Error::JavaException)
            } else {
                Ok(3)
            }
        })
        .catch("java/lang/StringIndexOutOfBoundsException", |caught| {
            assert!(!env.exception_check().unwrap());
            assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
            Ok(4)
        })
        .result()
    }

    #[test]
    fn test_catch_first() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(
                &env,
                Some("java/lang/IllegalArgumentException"),
                Err(Error::JavaException),
                false,
            )
            .unwrap(),
            1
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_second() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(
                &env,
                Some("java/lang/ArrayIndexOutOfBoundsException"),
                Err(Error::JavaException),
                false,
            )
            .unwrap(),
            2
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_third() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(
                &env,
                Some("java/lang/StringIndexOutOfBoundsException"),
                Err(Error::JavaException),
                false,
            )
            .unwrap(),
            3
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_ok() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(test_catch(&env, None, Ok(0), false).unwrap(), 0);
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_none() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException = test_catch(
            &env,
            Some("java/lang/SecurityException"),
            Err(Error::JavaException),
            false,
        )
        .unwrap_err()
        {
            assert!(env.exception_check().unwrap());
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env
                .is_instance_of(ex, "java/lang/SecurityException")
                .unwrap());
        } else {
            panic!("No JavaException");
        }
    }

    #[test]
    fn test_catch_other() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::InvalidCtorReturn =
            test_catch(env, None, Err(Error::InvalidCtorReturn), false).unwrap_err()
        {
            assert!(!env.exception_check().unwrap());
        } else {
            panic!("InvalidCtorReturn not found");
        }
    }

    #[test]
    fn test_catch_bogus_exception() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException =
            test_catch(env, None, Err(Error::JavaException), false).unwrap_err()
        {
            assert!(!env.exception_check().unwrap());
        } else {
            panic!("JavaException not found");
        }
    }

    #[test]
    fn test_catch_prior_exception() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let ex: JThrowable = env
            .new_object("java/lang/IllegalArgumentException", "()V", &[])
            .unwrap()
            .into();
        env.throw(ex).unwrap();

        if let Error::JavaException = test_catch(&env, None, Ok(0), false).unwrap_err() {
            assert!(env.exception_check().unwrap());
            let actual_ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env.is_same_object(actual_ex, ex).unwrap());
        } else {
            panic!("JavaException not found");
        }
    }

    #[test]
    fn test_catch_rethrow() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException = test_catch(
            &env,
            Some("java/lang/StringIndexOutOfBoundsException"),
            Err(Error::JavaException),
            true,
        )
        .unwrap_err()
        {
            assert!(env.exception_check().unwrap());
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            assert!(env
                .is_instance_of(ex, "java/lang/StringIndexOutOfBoundsException")
                .unwrap());
        } else {
            panic!("JavaException not found");
        }
    }

    #[test]
    fn test_catch_bogus_rethrow() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException = test_catch(
            &env,
            Some("java/lang/ArrayIndexOutOfBoundsException"),
            Err(Error::JavaException),
            true,
        )
        .unwrap_err()
        {
            assert!(!env.exception_check().unwrap());
        } else {
            panic!("JavaException not found");
        }
    }
}
