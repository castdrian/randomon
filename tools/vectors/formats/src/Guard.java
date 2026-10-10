import java.security.Permission;
import java.util.Map;
import java.util.concurrent.Callable;
import java.util.concurrent.atomic.AtomicReference;

public final class Guard {
    private static final ThreadLocal<Boolean> BLOCK_EXIT = ThreadLocal.withInitial(() -> false);

    private Guard() {
    }

    static final class ExitAttempt extends SecurityException {
        ExitAttempt() {
            super("System.exit blocked");
        }
    }

    @SuppressWarnings("removal")
    public static void install() {
        System.setSecurityManager(new SecurityManager() {
            @Override
            public void checkPermission(Permission permission) {
            }

            @Override
            public void checkPermission(Permission permission, Object context) {
            }

            @Override
            public void checkExit(int status) {
                if (BLOCK_EXIT.get()) {
                    throw new ExitAttempt();
                }
            }
        });
    }

    public static Map<String, Object> run(Callable<Object> body) {
        return run(body, 20000);
    }

    public static Map<String, Object> run(Callable<Object> body, long timeoutMillis) {
        AtomicReference<Map<String, Object>> outcome = new AtomicReference<>();
        Thread worker = new Thread(() -> {
            BLOCK_EXIT.set(true);
            outcome.set(evaluate(body));
        });
        worker.setDaemon(true);
        worker.start();
        try {
            worker.join(timeoutMillis);
        } catch (InterruptedException e) {
            throw new RuntimeException(e);
        }
        Map<String, Object> result = outcome.get();
        if (result == null) {
            result = Json.object();
            result.put("result", "hang");
        }
        return result;
    }

    private static Map<String, Object> evaluate(Callable<Object> body) {
        Map<String, Object> result = Json.object();
        java.io.PrintStream originalOut = System.out;
        java.io.PrintStream originalErr = System.err;
        try {
            Object value = body.call();
            if (value == null) {
                result.put("result", "null");
            } else {
                result.put("result", "ok");
                result.put("value", value);
            }
        } catch (ExitAttempt e) {
            result.put("result", "exit");
        } catch (Throwable e) {
            Throwable root = e;
            if (root instanceof ExitAttempt || (root.getCause() instanceof ExitAttempt)) {
                result.put("result", "exit");
            } else {
                result.put("result", "error");
                result.put("error", root.getClass().getName());
            }
        } finally {
            System.setOut(originalOut);
            System.setErr(originalErr);
        }
        return result;
    }
}
