/* Linux-only isolated syscall boundary probe. No application/test secrets. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

static int injected;
static void before_lock(const char *path, int flags) {
    const char *lock = getenv("MANYHANDS_POLL_RACE_LOCK");
    if (injected || !lock || strcmp(path, lock) || !(flags & O_CREAT)) return;
    injected = 1;
    pid_t child = fork();
    if (!child) {
        unsetenv("MANYHANDS_POLL_RACE_LOCK");
        execlp("git", "git", "-C", getenv("MANYHANDS_POLL_RACE_ROOT"),
               "update-ref", getenv("MANYHANDS_POLL_RACE_REF"),
               getenv("MANYHANDS_POLL_RACE_COMPETING"), (char *)0);
        _exit(97);
    }
    int status;
    if (child < 0 || waitpid(child, &status, 0) != child ||
        !WIFEXITED(status) || WEXITSTATUS(status)) _exit(98);
    const char message[] = "external Git committed before reference lock\n";
    if (write(2, message, sizeof(message) - 1) != (ssize_t)(sizeof(message) - 1)) _exit(99);
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, int);
        va_end(args);
    }
    before_lock(path, flags);
    int (*real_open)(const char *, int, ...) = dlsym(RTLD_NEXT, "open");
    return real_open(path, flags, mode);
}

int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, int);
        va_end(args);
    }
    before_lock(path, flags);
    int (*real_open)(const char *, int, ...) = dlsym(RTLD_NEXT, "open64");
    return real_open(path, flags, mode);
}
