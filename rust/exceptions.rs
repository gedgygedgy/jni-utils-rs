use jni::{
    descriptors::Desc,
    errors::Error,
    objects::{JClass, JThrowable},
    JNIEnv,
};

pub trait Catch: Sized {
    type Output;

    fn result(self) -> Result<Self::Output, Error>;

    fn catch<'a: 'b, 'b>(
        self,
        env: &'b JNIEnv<'a>,
        class: impl Desc<'a, JClass<'a>>,
        block: impl FnOnce(JThrowable<'a>) -> Result<Self::Output, Error>,
    ) -> Result<Self::Output, Error> {
        match self.result() {
            Ok(value) => Ok(value),
            Err(err) => {
                if let Error::JavaException = err {
                    if env.exception_check()? {
                        let ex = env.exception_occurred()?;
                        env.exception_clear()?;
                        if env.is_instance_of(ex, class)? {
                            return block(ex);
                        }
                        env.throw(ex)?;
                    }
                }
                Err(err)
            }
        }
    }
}

impl<T> Catch for Result<T, Error> {
    type Output = T;

    fn result(self) -> Result<Self::Output, Error> {
        self
    }
}

#[cfg(test)]
mod test {
    use jni::{errors::Error, objects::JThrowable, JNIEnv};

    use super::Catch;
    use crate::test_utils;

    fn test_catch<'a: 'b, 'b>(env: &'b JNIEnv<'a>, class: Option<&str>) -> Result<i32, Error> {
        let illegal_argument_exception = env
            .find_class("java/lang/IllegalArgumentException")
            .unwrap();

        let (ex, result) = if let Some(class) = class {
            let ex: JThrowable = env.new_object(class, "()V", &[]).unwrap().into();
            env.throw(ex).unwrap();
            (Some(ex), Err(Error::JavaException))
        } else {
            (None, Ok(0))
        };

        result
            .catch(env, illegal_argument_exception, |caught| {
                assert!(!env.exception_check().unwrap());
                assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
                Ok(1)
            })
            .catch(env, "java/lang/ArrayIndexOutOfBoundsException", |caught| {
                assert!(!env.exception_check().unwrap());
                assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
                Ok(2)
            })
            .catch(env, "java/lang/IndexOutOfBoundsException", |caught| {
                assert!(!env.exception_check().unwrap());
                assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
                Ok(3)
            })
            .catch(env, "java/lang/StringIndexOutOfBoundsException", |caught| {
                assert!(!env.exception_check().unwrap());
                assert!(env.is_same_object(ex.unwrap(), caught).unwrap());
                Ok(4)
            })
    }

    #[test]
    fn test_catch_first() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(&env, Some("java/lang/IllegalArgumentException")).unwrap(),
            1
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_second() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(&env, Some("java/lang/ArrayIndexOutOfBoundsException")).unwrap(),
            2
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_third() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(
            test_catch(&env, Some("java/lang/StringIndexOutOfBoundsException")).unwrap(),
            3
        );
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_ok() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        assert_eq!(test_catch(&env, None).unwrap(), 0);
        assert!(!env.exception_check().unwrap());
    }

    #[test]
    fn test_catch_none() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException =
            test_catch(&env, Some("java/lang/SecurityException")).unwrap_err()
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

        if let Error::InvalidCtorReturn = Err(Error::InvalidCtorReturn)
            .catch(env, "java/lang/Throwable", |_caught| Ok(()))
            .unwrap_err()
        {
        } else {
            panic!("InvalidCtorReturn not found");
        }
    }

    #[test]
    fn test_catch_bad_exception() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        if let Error::JavaException = Err(Error::JavaException)
            .catch(env, "java/lang/Throwable", |_caught| Ok(()))
            .unwrap_err()
        {
        } else {
            panic!("JavaException not found");
        }
    }
}
