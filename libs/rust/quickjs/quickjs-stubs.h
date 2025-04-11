#define CLOCK_MONOTONIC 1

typedef int clockid_t;

struct timespec {
    long tv_sec;  // seconds
    long tv_nsec; // nanoseconds
};

int clock_gettime(clockid_t clk_id, struct timespec *tp) {
    return 0;
}
