package gedgygedgy.rust.future;
import gedgygedgy.rust.task.Poll;
import gedgygedgy.rust.task.Waker;

public final class Future<T> {
    private Waker waker = null;
    private Poll<T> result;
    private final Object lock = new Object();

    public Poll<T> poll(Waker waker) {
        synchronized (this.lock) {
            if (this.result != null) {
                return this.result;
            } else {
                this.waker = waker;
                return null;
            }
        }
    }

    public void wake(T result) {
        Waker waker = null;
        synchronized (this.lock) {
            this.result = () -> {
                return result;
            };
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }
}
