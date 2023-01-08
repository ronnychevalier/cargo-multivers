use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// From https://github.com/calebzulawski/target-features/
//
// Later we should get rid of this, and call rustc at runtime to get the CPUs and features

fn main() {
    let target_cpus = std::fs::read_to_string("target-cpus.txt").unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    let mut lines = target_cpus.lines().peekable();
    let mut archs: HashMap<&str, HashMap<&str, HashSet<&str>>> = HashMap::new();
    while lines.peek().is_some() {
        let cpu = lines.next().unwrap().strip_prefix("cpu =").unwrap().trim();
        let arch = lines.next().unwrap().strip_prefix("arch =").unwrap().trim();
        let features = lines
            .next()
            .unwrap()
            .strip_prefix("features =")
            .unwrap()
            .trim()
            .split(' ')
            .filter(|s| !s.is_empty())
            .collect::<HashSet<_>>();
        let _ = lines.next();
        archs.entry(arch).or_default().insert(cpu, features);
    }

    let mut module = BufWriter::new(File::create(Path::new(&out_dir).join("codegen.rs")).unwrap());

    write!(
        &mut module,
        "static CPUS: phf::Map<&'static str, phf::Map<&'static str, phf::Set<&'static str>>> = "
    )
    .unwrap();
    let archs = archs
        .into_iter()
        .fold(phf_codegen::Map::new(), |mut phf, (arch, cpus)| {
            let cpus = cpus
                .into_iter()
                .fold(phf_codegen::Map::new(), |mut phf, (cpu, features)| {
                    let features = features
                        .into_iter()
                        .fold(phf_codegen::Set::new(), |mut phf, feature| {
                            phf.entry(feature);
                            phf
                        })
                        .build()
                        .to_string();

                    phf.entry(cpu, &features);
                    phf
                })
                .build()
                .to_string();

            phf.entry(arch, &cpus);
            phf
        });

    write!(&mut module, "{}", archs.build()).unwrap();

    writeln!(&mut module, ";").unwrap();
}
