use jni::{errors::Result, JNIEnv};

pub mod future;

pub fn init(env: &JNIEnv) -> Result<()> {
    future::jni::init(env)?;
    Ok(())
}
