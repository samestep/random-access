// Summation.

use std::ops::{AddAssign, Index};

use num::{Float, Num, traits::ToBytes};

trait Number: Sized + Copy + Into<f64> + AddAssign + Num + Float + ToBytes {}

impl Number for f32 {}

impl Number for f64 {}

fn sum<T: Number, I: Copy>(floats: &[T], indices: &[I]) -> T
where
    [T]: Index<I, Output = T>,
{
    let mut total = T::zero();
    for &i in indices {
        total += floats[i];
    }
    total
}

#[test]
fn test_sum_f64_usize_not_associative() {
    let floats: &[f64] = &[0.1, 0.2, 0.3];
    let forward: &[usize] = &[0, 1, 2];
    let backward: &[usize] = &[2, 1, 0];
    assert_eq!(sum(floats, forward), 0.6000000000000001);
    assert_eq!(sum(floats, backward), 0.6);
}

// Core data generation.

use std::{
    fmt, fs,
    io::{self, Read, Seek, Write},
};

use rand::Rng;
use rand_distr::{Distribution, Normal, StandardNormal};
use tempfile::tempfile;

trait Int: TryFrom<usize, Error: fmt::Debug> + ToBytes {}

impl Int for u32 {}

impl Int for u64 {}

trait Progress {
    fn new(len: usize) -> Self;

    fn step(&mut self);
}

fn random_floats<T: Number, P: Progress>(rng: &mut impl Rng, mut writer: impl Write, n: usize)
where
    StandardNormal: Distribution<T>,
{
    let mut progress = P::new(n);
    let normal = Normal::<T>::new(T::zero(), T::one()).unwrap();
    for _ in 0..n {
        let x = normal.sample(rng);
        writer.write_all(x.to_ne_bytes().as_ref()).unwrap();
        progress.step();
    }
}

fn first_to_last<I: Int, P: Progress>(mut writer: impl Write, n: usize) {
    let mut progress = P::new(n);
    for i in 0..n {
        writer
            .write_all(I::try_from(i).unwrap().to_ne_bytes().as_ref())
            .unwrap();
        progress.step();
    }
}

fn permutation<I: Int, P: Progress>(rng: &mut impl Rng, mut writer: impl Write, n: usize) {
    let m = (n * size_of::<I>()).div_ceil(1 << 30);
    let mut progress = P::new(2 * n);
    let mut files: Vec<fs::File> = (0..m).map(|_| tempfile().unwrap()).collect();
    {
        let mut writers: Vec<io::BufWriter<_>> = files.iter_mut().map(io::BufWriter::new).collect();
        for i in 0..n {
            let j = rng.random_range(0..m);
            writers[j]
                .write_all(I::try_from(i).unwrap().to_ne_bytes().as_ref())
                .unwrap();
            progress.step();
        }
    }
    for mut file in files {
        let mut bytes = Vec::new();
        file.seek(io::SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut bytes).unwrap();
        let (prefix, values, suffix) = unsafe { bytes.align_to_mut::<I>() };
        assert!(prefix.is_empty());
        assert!(suffix.is_empty());
        for i in 0..values.len() {
            let j = rng.random_range(..=i);
            values.swap(i, j);
            progress.step();
        }
        writer.write_all(&bytes).unwrap();
    }
}

// Bulk data generation.

use std::{ops::RangeInclusive, path::Path};

use indicatif::ProgressBar;
use rand::SeedableRng;

impl Progress for ProgressBar {
    fn new(len: usize) -> Self {
        let bar = ProgressBar::new(len as u64);
        bar.set_position(0);
        bar
    }

    fn step(&mut self) {
        self.inc(1);
    }
}

fn make_rng(seed: u64) -> impl Rng {
    rand_pcg::Pcg64Mcg::seed_from_u64(seed)
}

fn generate_file(dir_name: &str, file_name: &str, f: impl FnOnce(io::BufWriter<fs::File>)) {
    let dir = Path::new(dir_name);
    let path = dir.join(file_name);
    println!("generating {}", path.display());
    fs::create_dir_all(dir).unwrap();
    f(io::BufWriter::new(fs::File::create(path).unwrap()));
}

struct Options {
    f32: bool,
    f64: bool,
    u32: bool,
    u64: bool,
}

const FLOAT32: &str = "float32";
const FLOAT64: &str = "float64";
const UNSHUFFLED32: &str = "unshuffled32";
const SHUFFLED32: &str = "shuffled32";
const UNSHUFFLED64: &str = "unshuffled64";
const SHUFFLED64: &str = "shuffled64";

fn generate(exponents: RangeInclusive<usize>, options: Options) {
    for exponent in exponents {
        let n = 1 << exponent;
        let name = format!("{exponent}.dat");
        if options.f32 {
            generate_file(FLOAT32, &name, |writer| {
                random_floats::<f32, ProgressBar>(&mut make_rng(0), writer, n);
            });
        }
        if options.f64 {
            generate_file(FLOAT64, &name, |writer| {
                random_floats::<f64, ProgressBar>(&mut make_rng(1), writer, n);
            });
        }
        if options.u32 {
            generate_file(UNSHUFFLED32, &name, |writer| {
                first_to_last::<u32, ProgressBar>(writer, n);
            });
            generate_file(SHUFFLED32, &name, |writer| {
                permutation::<u32, ProgressBar>(&mut make_rng(2), writer, n);
            });
        }
        if options.u64 {
            generate_file(UNSHUFFLED64, &name, |writer| {
                first_to_last::<u64, ProgressBar>(writer, n);
            });
            generate_file(SHUFFLED64, &name, |writer| {
                permutation::<u64, ProgressBar>(&mut make_rng(3), writer, n);
            });
        }
    }
}

// Measurement.

use std::time::Instant;

use serde::Serialize;

#[derive(Serialize)]
struct Measurement<'a> {
    floats: &'a str,
    indices: &'a str,
    exponent: usize,
    iteration: usize,
    output: f64,
    seconds: f64,
}

#[derive(Clone, Copy)]
struct Index32(u32);

#[derive(Clone, Copy)]
struct Index64(u64);

impl<T> Index<Index32> for [T] {
    type Output = T;

    fn index(&self, index: Index32) -> &Self::Output {
        unsafe { self.get_unchecked(index.0 as usize) }
    }
}

impl<T> Index<Index64> for [T] {
    type Output = T;

    fn index(&self, index: Index64) -> &Self::Output {
        unsafe { self.get_unchecked(index.0 as usize) }
    }
}

unsafe fn reinterpret<T>(bytes: &[u8]) -> &[T] {
    let (prefix, values, suffix) = unsafe { bytes.align_to::<T>() };
    assert!(prefix.is_empty());
    assert!(suffix.is_empty());
    values
}

fn measure_files<T: Number, I: Copy>(
    dir_floats: &str,
    dir_indices: &str,
    exponent: usize,
    repeat: usize,
) where
    [T]: Index<I, Output = T>,
{
    let name = format!("{exponent}.dat");
    let bytes_floats = fs::read(Path::new(dir_floats).join(&name)).unwrap();
    let bytes_indices = fs::read(Path::new(dir_indices).join(&name)).unwrap();
    let floats = unsafe { reinterpret::<T>(&bytes_floats) };
    let indices = unsafe { reinterpret::<I>(&bytes_indices) };
    for iteration in 0..repeat {
        let start = Instant::now();
        let total = sum(floats, indices);
        let duration = start.elapsed();
        let measurement = Measurement {
            floats: dir_floats,
            indices: dir_indices,
            exponent,
            iteration,
            output: total.into(),
            seconds: duration.as_secs_f64(),
        };
        println!("{}", serde_json::to_string(&measurement).unwrap());
    }
}

fn measure(exponents: RangeInclusive<usize>, options: Options, repeat: usize) {
    for exponent in exponents {
        if options.f32 {
            if options.u32 {
                measure_files::<f32, Index32>(FLOAT32, UNSHUFFLED32, exponent, repeat);
                measure_files::<f32, Index32>(FLOAT32, SHUFFLED32, exponent, repeat);
            }
            if options.u64 {
                measure_files::<f32, Index64>(FLOAT32, UNSHUFFLED64, exponent, repeat);
                measure_files::<f32, Index64>(FLOAT32, SHUFFLED64, exponent, repeat);
            }
        }
        if options.f64 {
            if options.u32 {
                measure_files::<f64, Index32>(FLOAT64, UNSHUFFLED32, exponent, repeat);
                measure_files::<f64, Index32>(FLOAT64, SHUFFLED32, exponent, repeat);
            }
            if options.u64 {
                measure_files::<f64, Index64>(FLOAT64, UNSHUFFLED64, exponent, repeat);
                measure_files::<f64, Index64>(FLOAT64, SHUFFLED64, exponent, repeat);
            }
        }
    }
}

// Interface.

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Action {
    /// Generate data files
    Generate {
        /// Array lengths are powers of two with at least this exponent
        #[arg(long, default_value_t = 0)]
        min: usize,

        /// Array lengths are powers of two with at most this exponent
        #[arg(long, default_value_t = 24)]
        max: usize,

        /// Don't generate single-precision floating-point data
        #[arg(long)]
        no_f32: bool,

        /// Don't generate double-precision floating-point data
        #[arg(long)]
        no_f64: bool,

        /// Don't generate 32-bit index data
        #[arg(long)]
        no_u32: bool,

        /// Don't generate 64-bit index data
        #[arg(long)]
        no_u64: bool,
    },

    /// Run on generated data and print timings as JSON
    Measure {
        /// Array lengths are powers of two with at least this exponent
        #[arg(long, default_value_t = 0)]
        min: usize,

        /// Array lengths are powers of two with at most this exponent
        #[arg(long, default_value_t = 24)]
        max: usize,

        /// Number of measurements to collect for each case
        #[arg(long, default_value_t = 100)]
        repeat: usize,

        /// Don't use single-precision floating-point data
        #[arg(long)]
        no_f32: bool,

        /// Don't use double-precision floating-point data
        #[arg(long)]
        no_f64: bool,

        /// Don't use 32-bit index data
        #[arg(long)]
        no_u32: bool,

        /// Don't use 64-bit index data
        #[arg(long)]
        no_u64: bool,
    },
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

fn main() {
    match Cli::parse().command {
        Action::Generate {
            min,
            max,
            no_f32,
            no_f64,
            no_u32,
            no_u64,
        } => {
            let options = Options {
                f32: !no_f32,
                f64: !no_f64,
                u32: !no_u32,
                u64: !no_u64,
            };
            generate(min..=max, options);
        }
        Action::Measure {
            min,
            max,
            repeat,
            no_f32,
            no_f64,
            no_u32,
            no_u64,
        } => {
            let options = Options {
                f32: !no_f32,
                f64: !no_f64,
                u32: !no_u32,
                u64: !no_u64,
            };
            measure(min..=max, options, repeat);
        }
    }
}
