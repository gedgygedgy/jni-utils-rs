package io.github.gedgygedgy.rust.ops;

import io.github.gedgygedgy.rust.thread.LocalThreadChecker;

import java.io.Closeable;

/**
 * Wraps a {@code std::ops::FnOnce} in a Java object.
 * <p>
 * Instances of this class cannot be obtained directly from Java. Instead, call
 * {@code jni_utils::ops::fn_once_runnable()} from Rust code to obtain an
 * instance of this class.
 */
public final class FnOnceRunnable implements Runnable, Closeable {
    private final LocalThreadChecker threadChecker;
    private long data;

    private FnOnceRunnable(boolean local) {
        this.threadChecker = new LocalThreadChecker(local);
    }

    /**
     * Runs the {@code std::ops::FnOnce} associated with this object.
     * <p>
     * This method is idempotent - if it's called twice, the second call is a
     * no-op. In addition, if {@link close} has already been called, this
     * method is a no-op.
     */
    @Override
    public void run() {
        this.threadChecker.check();
        this.runInternal();
    }

    private native void runInternal();

    /**
     * Disposes of the {@code std::ops::FnOnce} associated with this object.
     * <p>
     * This method is idempotent - if it's called twice, the second call is a
     * no-op. In addition, if {@link run} has already been called, this method
     * is a no-op.
     */
    @Override
    public void close() {
        this.threadChecker.check();
        this.closeInternal();
    }

    private native void closeInternal();
}
