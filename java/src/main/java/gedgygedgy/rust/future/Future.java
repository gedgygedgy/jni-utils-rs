package gedgygedgy.rust.future;

import gedgygedgy.rust.task.Poll;
import gedgygedgy.rust.task.Waker;

public interface Future<T> {
    Poll<T> poll(Waker waker);
}
