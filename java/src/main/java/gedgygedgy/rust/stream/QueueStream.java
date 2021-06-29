package gedgygedgy.rust.stream;

import gedgygedgy.rust.task.Poll;
import gedgygedgy.rust.task.Waker;

import java.util.LinkedList;
import java.util.Queue;

public class QueueStream<T> implements Stream<T> {
    private Waker waker = null;
    private final Queue<T> result = new LinkedList<>();
    private boolean finished = false;
    private final Object lock = new Object();

    public QueueStream() {}

    @Override
    public Poll<StreamPoll<T>> pollNext(Waker waker) {
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

    public void add(T item) {
        Waker waker = null;
        synchronized (this.lock) {
            assert !this.finished;
            this.result.add(item);
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    public void finish() {
        Waker waker = null;
        synchronized (this.lock) {
            assert !this.finished;
            this.finished = true;
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }
}
