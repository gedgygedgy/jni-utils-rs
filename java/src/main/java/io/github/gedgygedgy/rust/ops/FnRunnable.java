package io.github.gedgygedgy.rust.ops;

import io.github.gedgygedgy.rust.thread.LocalThreadChecker;

import java.io.Closeable;

/**
 * Wraps a {@code std::ops::Fn} in a Java object.
 * <p>
 * Instances of this class cannot be obtained directly from Java. Instead, call
 * {@code jni_utils::ops::fn_runnable()} from Rust code to obtain an instance
 * of this class.
 */
public final class FnRunnable implements Runnable, Closeable {
    private final LocalThreadChecker threadChecker;
    private long data;

    private FnRunnable(boolean local) {
        this.threadChecker = new LocalThreadChecker(local);
    }

    /**
     * Runs the {@code std::ops::Fn} associated with this object.
     * <p>
     * Unlike {@link FnOnceRunnable#run}, this method is not idempotent -
     * calling it twice will call the associated {@code std::ops::Fn} twice.
     * If {@link close} has already been called, this method is a no-op.
     */
    @Override
    public void run() {
        this.threadChecker.check();
        this.runInternal();
    }

    private native void runInternal();

    /**
     * Disposes of the {@code std::ops::Fn} associated with this object.
     * <p>
     * This method is idempotent - if it's called twice, the second call is a
     * no-op.
     */
    @Override
    public void close() {
        this.threadChecker.check();
        this.closeInternal();
    }

    private native void closeInternal();
}
