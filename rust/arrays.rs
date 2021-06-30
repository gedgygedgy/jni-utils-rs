use jni::{
    errors::Result,
    objects::{AutoArray, ReleaseMode},
    sys::{jbyte, jbyteArray, jint},
    JNIEnv,
};
use std::{iter::FromIterator, slice};

pub fn slice_to_byte_array<'a, 'b>(env: &'a JNIEnv<'a>, slice: &'b [u8]) -> Result<jbyteArray> {
    let obj = env.new_byte_array(slice.len() as jint)?;
    let array: AutoArray<'a, 'a, jbyte> =
        env.get_byte_array_elements(obj, ReleaseMode::CopyBack)?;
    let array_slice: &'a mut [jbyte] =
        unsafe { slice::from_raw_parts_mut(array.as_ptr(), slice.len()) };
    slice.into_iter().zip(array_slice).for_each(|(src, dest)| {
        *dest = *src as jbyte;
    });
    Ok(obj)
}

pub fn byte_array_to_vec<'a>(env: &'a JNIEnv<'a>, obj: jbyteArray) -> Result<Vec<u8>> {
    let array: AutoArray<'a, 'a, jbyte> =
        env.get_byte_array_elements(obj, ReleaseMode::NoCopyBack)?;
    let array_slice: &'a [jbyte] =
        unsafe { slice::from_raw_parts(array.as_ptr(), array.size()? as usize) };
    Ok(Vec::from_iter(array_slice.iter().map(|item| *item as u8)))
}

#[cfg(test)]
mod test {
    use crate::test_utils;

    #[test]
    fn test_slice_to_byte_array() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let obj = super::slice_to_byte_array(env, &[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(env.get_array_length(obj).unwrap(), 5);

        let mut bytes = [0i8; 5];
        env.get_byte_array_region(obj, 0, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_byte_array_to_vec() {
        let attach_guard = test_utils::JVM.attach_current_thread().unwrap();
        let env = &*attach_guard;

        let obj = env.new_byte_array(5).unwrap();
        env.set_byte_array_region(obj, 0, &[1, 2, 3, 4, 5]).unwrap();

        let vec = super::byte_array_to_vec(env, obj).unwrap();
        assert_eq!(vec, vec![1, 2, 3, 4, 5]);
    }
}
