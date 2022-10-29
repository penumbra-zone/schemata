use schemata::schema;

schema! {
    foo: u64;
    bar: String;
    baz(x: f32) {
        qux: u8;
    }
}
