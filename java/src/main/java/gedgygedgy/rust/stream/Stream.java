package gedgygedgy.rust.stream;
import gedgygedgy.rust.task.Poll;

import java.util.LinkedList;
import java.util.Queue;

public class Stream<T> {
    private gedgygedgy.rust.task.Waker waker = null;
    private final Queue<T> result = new LinkedList<>();
    private boolean finished = false;
    private final Object lock = new Object();

    private Stream() {}

    public Poll<StreamPoll<T>> pollNext(gedgygedgy.rust.task.Waker waker) {
        synchronized (this.lock) {
            if (!this.result.isEmpty()) {
                return () -> () -> this.result.remove();
            }
            if (this.finished) {
                return () -> null;
            }
            this.waker = waker;
            return null;
        }
    }

    public static <U> Waker<U> create() {
        return new Waker<U>(new Stream<U>());
    }

    private void add(T item) {
        gedgygedgy.rust.task.Waker waker = null;
        synchronized (this.lock) {
            assert !this.finished;
            this.result.add(item);
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    private void finish() {
        gedgygedgy.rust.task.Waker waker = null;
        synchronized (this.lock) {
            this.finished = true;
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    public static class Waker<T> {
        private final Stream<T> stream;

        private Waker(Stream<T> stream) {
            this.stream = stream;
        }

        public Stream<T> getStream() {
            return this.stream;
        }

        public void add(T result) {
            this.stream.add(result);
        }

        public void finish() {
            this.stream.finish();
        }
    }
}
