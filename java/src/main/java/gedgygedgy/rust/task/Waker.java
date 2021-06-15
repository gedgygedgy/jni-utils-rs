package gedgygedgy.rust.task;

public final class Waker {
    private long data;

    private Waker() {}

    public native void wake();

    @Override
    @SuppressWarnings("deprecation") // We want finalize() to clean up the memory.
    protected native void finalize();
}
