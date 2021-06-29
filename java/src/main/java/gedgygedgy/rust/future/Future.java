package gedgygedgy.rust.future;
import gedgygedgy.rust.task.Poll;

public final class Future<T> {
    private gedgygedgy.rust.task.Waker waker = null;
    private Poll<T> result;
    private final Object lock = new Object();

    private Future() {}

    public Poll<T> poll(gedgygedgy.rust.task.Waker waker) {
        synchronized (this.lock) {
            if (this.result != null) {
                return this.result;
            } else {
                this.waker = waker;
                return null;
            }
        }
    }

    public static <U> Waker<U> create() {
        return new Waker<U>(new Future<U>());
    }

    private void wake(Poll result) {
        gedgygedgy.rust.task.Waker waker = null;
        synchronized (this.lock) {
            this.result = result;
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    public static class Waker<T> {
        private final Future<T> future;

        private Waker(Future<T> future) {
            this.future = future;
        }

        public Future<T> getFuture() {
            return this.future;
        }

        public void wake(T result) {
            this.future.wake(() -> {
                return result;
            });
        }

        public void wakeWithThrowable(Throwable result) {
            this.future.wake(() -> {
                throw new FutureException(result);
            });
        }
    }
}
