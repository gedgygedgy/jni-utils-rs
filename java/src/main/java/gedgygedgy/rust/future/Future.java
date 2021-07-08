package gedgygedgy.rust.future;

import gedgygedgy.rust.task.PollResult;
import gedgygedgy.rust.task.Waker;

public interface Future<T> {
    PollResult<T> poll(Waker waker);
}
