package gedgygedgy.rust.stream;

import gedgygedgy.rust.task.Poll;
import gedgygedgy.rust.task.Waker;

public interface Stream<T> {
    Poll<StreamPoll<T>> pollNext(Waker waker);
}
