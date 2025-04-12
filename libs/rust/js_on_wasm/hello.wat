(module
    (import "host" "hello" (func $host_hello (param i32)))
    (func (export "hello")
        (call $host_hello (i32.const 3))
    )
)

