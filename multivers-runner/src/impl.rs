cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        pub use self::linux::*;
    } else {
        mod generic;
        pub use self::generic::*;
    }
}
