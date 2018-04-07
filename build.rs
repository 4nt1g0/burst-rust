extern crate gcc;

fn main() {
    // compile libsph
    gcc::Build::new().file("ext/shabal.c").compile("libsph_shabal.a");
}