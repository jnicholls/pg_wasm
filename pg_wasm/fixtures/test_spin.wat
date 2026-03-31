(module
  (func (export "spin") (result i32)
    (loop $l
      (br $l)
    )
    (i32.const 0)))
