package io.github.gedgygedgy.rust.task;

/**
 * Wraps a {@code std::task::Waker} in a Java object.
 * <p>
 * Instances of this class cannot be obtained directly from Java. Instead, call
 * {@code jni_utils::task::waker()} from Rust code to obtain an instance of
 * this class. (This generally shouldn't be necessary, since
 * {@code jni_utils::future::JFuture} and {@code jni_utils::stream::JStream}
 * take care of this for you.)
 */
public final class Waker {
    private long data;

    private Waker() {}

    /**
     * Wakes the {@code std::task::Waker} associated with this object.
     */
    public native void wake();

    /**
     * Frees the {@code std::task::Waker} associated with this object.
     */
    @Override
    @SuppressWarnings("deprecation") // We want finalize() to clean up the memory.
    protected native void finalize();
}
