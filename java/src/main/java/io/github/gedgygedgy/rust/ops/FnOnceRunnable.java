package io.github.gedgygedgy.rust.ops;

import java.io.Closeable;

/**
 * Wraps a {@code std::ops::FnOnce} in a Java object.
 * <p>
 * Instances of this class cannot be obtained directly from Java. Instead, call
 * {@code jni_utils::ops::fn_once_runnable()} from Rust code to obtain an
 * instance of this class.
 */
public final class FnOnceRunnable implements Runnable, Closeable {
    private long data;

    private FnOnceRunnable() {}

    /**
     * Runs the {@code std::ops::FnOnce} associated with this object.
     * <p>
     * This method is idempotent - if it's called twice, the second call is a
     * no-op. In addition, if {@link close} has already been called, this
     * method is a no-op.
     */
    @Override
    public native void run();

    /**
     * Disposes of the {@code std::ops::FnOnce} associated with this object.
     * <p>
     * This method is idempotent - if it's called twice, the second call is a
     * no-op. In addition, if {@link run} has already been called, this method
     * is a no-op.
     */
    @Override
    public native void close();
}
