package io.github.gedgygedgy.rust.ops;

import io.github.gedgygedgy.rust.thread.LocalThreadChecker;

import java.io.Closeable;

final class FnRunnableImpl implements FnRunnable {
    private final LocalThreadChecker threadChecker;
    private long data;

    private FnRunnableImpl(boolean local) {
        this.threadChecker = new LocalThreadChecker(local);
    }

    @Override
    public void run() {
        this.threadChecker.check();
        this.runInternal();
    }

    private native void runInternal();

    @Override
    public void close() {
        this.threadChecker.check();
        this.closeInternal();
    }

    private native void closeInternal();
}
