(component
  (core module $M
    (func (export "add") (param i32 i32) (result i32)
      (i32.add (local.get 0) (local.get 1)))
  )
  (core instance $i (instantiate $M))
  (func (export "add") (param "a" s32) (param "b" s32) (result s32)
    (canon lift (core func $i "add")))
)
