(module
  (memory 1)
  (func (export "add") (result i32)
    i32.const 42
  )
  (func (export "spin") (result i32)
    (loop (br 0))
    i32.const 0
  )
  (func $spin-rec (param i32) (result i32)
    (local.get 0)
    (if (result i32)
      (then
        (call $spin-rec (i32.sub (local.get 0) (i32.const 1)))
      )
      (else (i32.const 0))
    )
  )
  (func (export "spin-param") (param i32) (result i32)
    (call $spin-rec (local.get 0))
  )
  (func (export "grow") (result i32)
    (local $g i32)
    (local.set $g (memory.grow (i32.const 10)))
    ;; `memory.grow` returns -1 on failure (e.g. store page cap); trap so SQL surfaces an error.
    (if (i32.eq (local.get $g) (i32.const -1))
      (then (unreachable))
    )
    i32.const 0
  )
)
