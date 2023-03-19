fn main() {
    println!("Hello multivers!");
    println!(
        "args: {:#}",
        std::env::args().collect::<Vec<String>>().join(" ")
    );
}
