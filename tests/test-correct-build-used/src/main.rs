macro_rules! compiled_features_string {
    ($($target_feature_name: literal)*) => {{
        let compiled_features_array: &[&str] =
        &[
            $(
                #[cfg(target_feature = $target_feature_name)]
                $target_feature_name
            ),*
        ];
        compiled_features_array.join(",")
    }};
}

fn main() {
    let compile_time_features = compiled_features_string!(
        "adx" "aes" "avx" "avx2" "bmi1" "bmi2" "cmpxchg16b" "f16c" "fma"
        "fxsr" "lzcnt" "movbe" "pclmulqdq" "popcnt" "rdrand" "rdseed" "sha"
        "sse" "sse2" "sse3" "sse4.1" "sse4.2" "ssse3"
        "xsave" "xsavec" "xsaveopt" "xsaves"
    );
    print!("{compile_time_features}");
}
