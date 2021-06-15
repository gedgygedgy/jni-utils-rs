package gedgygedgy.rust.task;

public final class Waker {
    private long data;

    private Waker() {}

    public native void wake();

    @Override
    protected native void finalize();
}
