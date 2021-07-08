package gedgygedgy.rust.stream;

import gedgygedgy.rust.task.PollResult;
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
    public PollResult<StreamPoll<T>> pollNext(Waker waker) {
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

    private void doEvent(Runnable r) {
        Waker waker = null;
        synchronized (this.lock) {
            assert !this.finished;
            r.run();
            waker = this.waker;
        }
        if (waker != null) {
            waker.wake();
        }
    }

    public void add(T item) {
        this.doEvent(() -> this.result.add(item));
    }

    public void finish() {
        this.doEvent(() -> this.finished = true);
    }
}
