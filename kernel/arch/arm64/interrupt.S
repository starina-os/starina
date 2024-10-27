.balign 2048
.global arm64_exception_vector
arm64_exception_vector:
    b arm64_handle_exception
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_exception
    .balign 128

    b arm64_handle_exception
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_exception
    .balign 128

    b arm64_handle_exception
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_exception
    .balign 128

    b arm64_handle_exception
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_interrupt
    .balign 128
    b arm64_handle_exception
    .balign 128

.balign 2048
.global arm64_idle_exception_vector
arm64_idle_exception_vector:
    b arm64_handle_idle_exception
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_exception
    .balign 128

    b arm64_handle_idle_exception
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_exception
    .balign 128

    b arm64_handle_idle_exception
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_exception
    .balign 128

    b arm64_handle_idle_exception
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_interrupt
    .balign 128
    b arm64_handle_idle_exception
    .balign 128
