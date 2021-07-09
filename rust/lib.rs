//! # Extra Utilities for JNI in Rust
//!
//! This crate builds on top of the [`jni`](::jni) crate and provides
//! higher-level concepts to more easily deal with JNI. While the
//! [`jni`](::jni) crate implements low-level bindings to JNI,
//! [`jni-utils`](crate) is more focused on higher-level constructs that get
//! used frequently. Some of the features provided by [`jni-utils`](crate)
//! include:
//!
//! * Asynchronous calls to Java code using the [`JFuture`](future::JFuture)
//!   and [`JStream`](stream::JStream) types
//! * Conversion between various commonly-used Rust types and their
//!   corresponding Java types
//! * Emulation of `try`/`catch` blocks with the
//!   [`try_block`](exceptions::try_block) function
//!
//! The overriding principle of [`jni-utils`](crate) is that switches between
//! Rust and Java code should be minimized, and that it is easier to call Java
//! code from Rust than it is to call Rust code from Java. Calling Rust from
//! Java requires creating a class with a `native` method and exporting it from
//! Rust, either by a combination of `#[nomangle]` and `extern "C"` to export
//! the function as a symbol in a shared library, or by calling
//! [`JNIEnv::register_native_methods()`](::jni::JNIEnv::register_native_methods).
//! In contrast, calling Java from Rust only requires calling
//! [`JNIEnv::call_method()`](::jni::JNIEnv::call_method) (though you can cache
//! the method ID and use
//! [`JNIEnv::call_method_unchecked()`](::jni::JNIEnv::call_method_unchecked)
//! for a performance improvement.)
//!
//! To that end, [`jni-utils`](crate) seeks to minimize the number of holes
//! that must be poked through the Rust-Java boundary, and the number of
//! `native` exported-to-Java Rust functions that must be written. In
//! particular, the async API has been developed to minimize such exports by
//! allowing Java code to wake an `await` without creating a new `native`
//! function.
//!
//! Some features of [`jni-utils`](crate) require the accompanying Java support
//! library, which includes some native methods. Therefore,
//! [`jni_utils::init()`](crate::init) should be called before using
//! [`jni-utils`](crate).

use ::jni::{errors::Result, JNIEnv};

pub mod arrays;
pub mod exceptions;
pub mod future;
pub mod stream;
pub mod task;
pub mod uuid;

/// Initialize [`jni-utils`](crate) by registering required native methods.
/// This should be called before using [`jni-utils`](crate).
///
/// # Arguments
///
/// * `env` - Java environment with which to register native methods.
pub fn init(env: &JNIEnv) -> Result<()> {
    task::jni::init(env)?;
    Ok(())
}

pub(crate) mod jni {
    use ::jni::NativeMethod;
    use std::ffi::c_void;

    pub fn native(name: &str, sig: &str, fn_ptr: *mut c_void) -> NativeMethod {
        NativeMethod {
            name: name.into(),
            sig: sig.into(),
            fn_ptr,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use jni::JavaVM;
    use lazy_static::lazy_static;
    use std::{
        sync::{Arc, Mutex},
        task::{RawWaker, RawWakerVTable, Waker},
    };

    pub type TestWakerData = Mutex<bool>;

    unsafe fn test_waker_new(data: &Arc<TestWakerData>) -> RawWaker {
        let data_ptr = Arc::as_ptr(data);
        Arc::increment_strong_count(data_ptr);
        RawWaker::new(data_ptr as *const (), &VTABLE)
    }

    unsafe fn test_waker_clone(ptr: *const ()) -> RawWaker {
        let data_ptr = ptr as *const TestWakerData;
        Arc::increment_strong_count(data_ptr);
        RawWaker::new(data_ptr as *const (), &VTABLE)
    }

    unsafe fn test_waker_wake(ptr: *const ()) {
        test_waker_wake_by_ref(ptr);
        let data_ptr = ptr as *const TestWakerData;
        Arc::decrement_strong_count(data_ptr);
    }

    unsafe fn test_waker_wake_by_ref(ptr: *const ()) {
        let data_ptr = ptr as *const TestWakerData;
        let data = &*data_ptr;
        let mut lock = data.lock().unwrap();
        *lock = true;
    }

    unsafe fn test_waker_drop(ptr: *const ()) {
        let data_ptr = ptr as *const TestWakerData;
        Arc::decrement_strong_count(data_ptr);
    }

    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        test_waker_clone,
        test_waker_wake,
        test_waker_wake_by_ref,
        test_waker_drop,
    );

    pub fn test_waker(data: &Arc<TestWakerData>) -> Waker {
        unsafe { Waker::from_raw(test_waker_new(data)) }
    }

    lazy_static! {
        pub static ref JVM: JavaVM = {
            use jni::InitArgsBuilder;
            use std::{env, path::PathBuf};

            let mut jni_utils_jar = PathBuf::from(env::current_exe().unwrap());
            jni_utils_jar.pop();
            jni_utils_jar.pop();
            jni_utils_jar.push("java");
            jni_utils_jar.push("libs");
            jni_utils_jar.push("jni-utils.jar");

            let jvm_args = InitArgsBuilder::new()
                .option(&format!(
                    "-Djava.class.path={}",
                    jni_utils_jar.to_str().unwrap()
                ))
                .build()
                .unwrap();
            let jvm = JavaVM::new(jvm_args).unwrap();

            let env = jvm.attach_current_thread_permanently().unwrap();
            crate::init(&env).unwrap();

            jvm
        };
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, task::Waker};

    #[test]
    fn test_raw_waker_refcount() {
        let data = Arc::new(crate::test_utils::TestWakerData::new(false));
        assert_eq!(Arc::strong_count(&data), 1);

        let waker: Waker = crate::test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        let waker2 = waker.clone();
        assert_eq!(Arc::strong_count(&data), 3);
        assert_eq!(*data.lock().unwrap(), false);

        std::mem::drop(waker2);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        std::mem::drop(waker);
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), false);
    }

    #[test]
    pub fn test_raw_waker_wake() {
        let data = Arc::new(crate::test_utils::TestWakerData::new(false));
        assert_eq!(Arc::strong_count(&data), 1);

        let waker: Waker = crate::test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        waker.wake();
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(*data.lock().unwrap(), true);
    }

    #[test]
    pub fn test_raw_waker_wake_by_ref() {
        let data = Arc::new(crate::test_utils::TestWakerData::new(false));
        assert_eq!(Arc::strong_count(&data), 1);

        let waker: Waker = crate::test_utils::test_waker(&data);
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), false);

        waker.wake_by_ref();
        assert_eq!(Arc::strong_count(&data), 2);
        assert_eq!(*data.lock().unwrap(), true);
    }
}
