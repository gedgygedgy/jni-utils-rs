package gedgygedgy.rust.future;

public class JavaObjectFuture<T> {
    private long waker;
    private T result;
    private boolean haveResult = false;
    private final Object resultLock = new Object();

    public JavaObjectFuture() {
        this.initWaker();
    }

    public PollResult<T> poll() {
        synchronized (this.resultLock) {
            if (this.haveResult) {
                return new PollResult<T>(this.result);
            } else {
                return null;
            }
        }
    }

    public void wake(T result) {
        synchronized (this.resultLock) {
            this.haveResult = true;
            this.result = result;
        }
        this.wakeInternal();
    }

    private native void initWaker();

    private native void wakeInternal();
}
