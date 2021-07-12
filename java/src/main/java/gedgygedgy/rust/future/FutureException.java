package gedgygedgy.rust.future;

/**
 * Exception class for {@link Future} implementations to throw from
 * {@link gedgygedgy.rust.task.PollResult#get} if the result of the future is an exception.
 * Implementations should set the real exception as the cause of this
 * exception.
 */
public class FutureException extends RuntimeException {
    public FutureException(Throwable cause) {
        super(cause);
    }
}
