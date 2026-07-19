pub(crate) use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

use tiny_rv32ima_sim::simulator::{Initial, Simulator};

pub const TEST_DIR: &str = "tests/isa/flats";
pub const TEST_ELVES_DIR: &str = "tests/isa/elves";

pub struct RiscvTest<'a> {
    pub filename: &'a str,
    pub exit_address: u32,
}

pub fn run_test<P: AsRef<Path>>(
    simulator: &mut Simulator<Initial>,
    file_path: P,
    exit_address: u32,
) -> bool {
    let file_path = file_path.as_ref();
    let Ok(file) = File::open(file_path) else {
        println!("SKIP: {}", file_path.display());
        return false;
    };
    let mut reader = BufReader::new(file);

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();

    *simulator = Simulator::new();
    simulator.load_flat(&buf, 0x80000000);
    assert!(simulator.debug_run(exit_address));
    true
}

pub fn run_elf_test<P: AsRef<Path>>(
    simulator: &mut Simulator<Initial>,
    file_path: P,
    exit_address: u32,
) -> bool {
    let file_path = file_path.as_ref();
    let Ok(file) = File::open(file_path) else {
        println!("SKIP: {}", file_path.display());
        return false;
    };
    let mut reader = BufReader::new(file);

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();

    *simulator = Simulator::new();
    simulator.load_elf(&buf);
    assert!(simulator.debug_run(exit_address));
    true
}

pub fn run_tests<P: AsRef<Path>>(
    simulator: &mut Simulator<Initial>,
    dir_path: P,
    default_exit_address: u32,
    excludes: Vec<RiscvTest>,
) {
    let dir_path = dir_path.as_ref();
    let Ok(dir) = fs::read_dir(dir_path) else {
        println!("SKIP: {}", dir_path.display());
        return;
    };

    for file in dir.into_iter() {
        let file_path = file.unwrap().path();

        if file_path.extension().is_some_and(|ext| ext == "bin") {
            let filename = file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let mut exit_address = default_exit_address;
            for exclude in &excludes {
                if filename == exclude.filename {
                    exit_address = exclude.exit_address;
                }
            }

            println!("TRY: {}", filename);
            if run_test(simulator, file_path, exit_address) {
                println!("PASS: {}", filename);
            }
        }
    }
}

pub fn run_elf_tests<P: AsRef<Path>>(
    simulator: &mut Simulator<Initial>,
    dir_path: P,
    default_exit_address: u32,
    excludes: Vec<RiscvTest>,
) {
    let dir_path = dir_path.as_ref();
    let Ok(dir) = fs::read_dir(dir_path) else {
        println!("SKIP: {}", dir_path.display());
        return;
    };

    for file in dir.into_iter() {
        let file_path = file.unwrap().path();

        if file_path.extension().is_none() {
            let filename = file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            let mut exit_address = default_exit_address;
            for exclude in &excludes {
                if filename == exclude.filename {
                    exit_address = exclude.exit_address;
                }
            }

            println!("TRY: {}", filename);
            if run_elf_test(simulator, file_path, exit_address) {
                println!("PASS: {}", filename);
            }
        }
    }
}

pub fn run_flat_isa_tests(
    simulator: &mut Simulator<Initial>,
    suite: &str,
    default_exit_address: u32,
    exceptions: Vec<RiscvTest>,
) {
    let prefix = format!("{suite}-p-");
    let mut test_paths = fs::read_dir(TEST_DIR)
        .unwrap_or_else(|err| panic!("failed to read {TEST_DIR}: {err}"))
        .map(|entry| {
            entry
                .expect("failed to read ISA test directory entry")
                .path()
        })
        .filter(|path| {
            path.is_file()
                && path.extension().is_none()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .collect::<Vec<_>>();

    test_paths.sort();
    assert!(
        !test_paths.is_empty(),
        "no {suite} tests found in {TEST_DIR}"
    );

    for test_path in test_paths {
        let filename = test_path.file_name().unwrap().to_string_lossy();
        let exit_address = exceptions
            .iter()
            .find(|exception| filename == exception.filename)
            .map_or(default_exit_address, |exception| exception.exit_address);

        println!("TRY: {filename}");
        assert!(run_elf_test(simulator, &test_path, exit_address));
        println!("PASS: {filename}");
    }
}
