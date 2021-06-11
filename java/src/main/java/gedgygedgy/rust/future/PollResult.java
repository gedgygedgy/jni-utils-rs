package gedgygedgy.rust.future;

public class PollResult<T> {
    private T result;

    PollResult(T result) {
        this.result = result;
    }

    public T get() {
        return this.result;
    }
}
