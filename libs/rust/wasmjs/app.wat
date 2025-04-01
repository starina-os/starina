(module
  (import "builtins" "print" (func $print (param i32 i32)))
  (memory 1)
  (data (i32.const 0) "Hello World from WebAssembly!")
  (export "memory" (memory 0))
  (export "main" (func $main))
  (func $main
    (call $print
      (i32.const 0)
      (i32.const 28)
    )
  )
)