package gedgygedgy.rust.task;

public class Waker {
    private long data;

    private Waker() {}

    public native void wake();
}
