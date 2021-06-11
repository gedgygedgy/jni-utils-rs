package gedgygedgy.rust.future;

public class JavaObjectFuture<T> {
    private final Waker waker = new Waker();
    private T result;
    private boolean haveResult = false;
    private final Object resultLock = new Object();

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
        this.waker.wake();
    }

    // This is its own object so that JNIEnv::{get,set}_rust_field() can lock it
    // without interfering with anyone else.
    private static class Waker {
        @SuppressWarnings("unused") // This is used by native code.
        public long waker;

        public Waker() {
            this.init();
        }

        private native void init();

        public native void wake();

        @Override
        protected native void finalize();
    }
}
