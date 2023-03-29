fn main() {
    println!("{:#}", std::env::args().collect::<Vec<String>>().join(" "));
}
