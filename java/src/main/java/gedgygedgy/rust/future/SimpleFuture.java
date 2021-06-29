package gedgygedgy.rust.future;

import gedgygedgy.rust.task.Poll;
import gedgygedgy.rust.task.Waker;

public class SimpleFuture<T> implements Future<T> {
    private Waker waker = null;
    private Poll<T> result;
    private final Object lock = new Object();

    public SimpleFuture() {}

    @Override
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

    private void wakeInternal(Poll result) {
        Waker waker = null;
        synchronized (this.lock) {
            assert this.result == null;
            this.result = result;
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    public void wake(T result) {
        this.wakeInternal(() -> {
            return result;
        });
    }

    public void wakeWithThrowable(Throwable result) {
        this.wakeInternal(() -> {
            throw new FutureException(result);
        });
    }
}
