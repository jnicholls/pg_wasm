//! Component fixture for pg_wasm marshal / composite integration tests.

mod bindings;

use bindings::{Guest, Point, export};

struct Component;

impl Guest for Component {
    fn echo_point(p: Point) -> Point {
        p
    }

    fn echo_tuple(t: (i32, i32)) -> (i32, i32) {
        t
    }
}

export!(Component with_types_in bindings);
