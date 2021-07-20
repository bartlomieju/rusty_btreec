fn main() {
    let src = [
        "src/btree.c",
    ];
    let mut builder = cc::Build::new();
    let build = builder
        .files(src.iter());
    build.compile("btreec");
}