#![feature(step_trait)]

use std::thread;
use std::sync::mpsc;
use std::panic;
use std::mem;

// debug uses
use std::ops::SubAssign;
use std::fmt::Display;
use std::panic::UnwindSafe;
use std::panic::RefUnwindSafe;
use num::traits::ToPrimitive;
use num::traits::NumCast;
use num::traits::Saturating;
use std::iter::Step;

mod float;
mod int;

extern crate num_cpus;
extern crate num;

fn test<T, U>(f: extern fn (T, bool) -> U, i: T) -> bool
    where T: Display + UnwindSafe + RefUnwindSafe + ToPrimitive + Copy,
          U: PartialEq + Display + NumCast {
    let mut has_error = false;
    let result = panic::catch_unwind(|| {
        let expected = U::from(i).unwrap();

        let actual = f(i, false);
        println!("{}=={}", expected, actual);

        let err = expected != actual;
        if err {
            println!("{}: ex{}!=ax{}", i, expected, actual);
            f(i, true);
        }
        err
    });

    if let Err(_) = result {
        has_error = true;
        let _ = panic::catch_unwind(||{
            f(i, true);
        });
    } else {
        has_error |= result.unwrap();
    }
    has_error
}

fn test_range<T, U, V>(f: extern fn (U, bool) -> V, min: T, stp: T, max: T) -> bool
    where T: PartialOrd + Saturating + PartialEq + Step + NumCast + SubAssign + Send + Copy + 'static,
          for<'a> &'a T: std::ops::Add<Output=T>,
          U: Display + UnwindSafe + RefUnwindSafe + ToPrimitive + Copy + 'static,
          V: PartialEq + Display + NumCast + 'static {

    let cpu_count = num_cpus::get() as u32;

    let (tx, rx) = mpsc::channel();
    let mut current_offset = min;
    let mut active_thread_count = 0;
    let mut is_falty = false;
    loop {
        if (active_thread_count < cpu_count) && (current_offset < max) {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut has_error = false;
                let offset = current_offset;
                let mut loop_max = offset.saturating_add(stp);

                if loop_max < max {
                    loop_max -= T::from(1).unwrap();
                } else {
                    loop_max = max;
                }

                //println!("range={}..{}", offset, loop_max);
                for i in offset..loop_max {
                    //has_error |= test(__fixsfsi, i as f32);
                    //has_error |= test(si2f, i);
                    has_error |= test(f, unsafe { mem::transmute_copy(&i) });

                    if has_error {
                        break;
                    }
                }

                let _ = tx.send(has_error);
            });

            active_thread_count += 1;
            current_offset = current_offset.saturating_add(stp);
        } else {
            if active_thread_count == 0 {
                break;
            } else {
                if rx.recv().unwrap() {
                    is_falty = true;
                    break;
                }

                active_thread_count -= 1;
            }
        }
    }
    is_falty
}

fn main() {
    // T=u32 U=f32 V=i32
    //test_range(float::conv::__floatsisf, min, stp, max);

    let stp = 0x80000000u32;
    // [-inf ; 0 [
    test_range(float::conv::__fixsfsi, 0xFFF80000u32, stp, 0xFFF80000u32);
}
