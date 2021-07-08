package gedgygedgy.rust.stream;

import gedgygedgy.rust.task.PollResult;
import gedgygedgy.rust.task.Waker;

public interface Stream<T> {
    PollResult<StreamPoll<T>> pollNext(Waker waker);
}
