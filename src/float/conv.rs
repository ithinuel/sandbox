use float::Float;
use int::Int;

macro_rules! fp_overflow {
    (infinity, $fty:ty, $sign: expr) => {
        return {
            <$fty as Float>::from_parts(
                $sign,
                <$fty as Float>::exponent_max() as <$fty as Float>::Int,
                0 as <$fty as Float>::Int)
        }
    }
}

macro_rules! fp_convert {
    ($intrinsic:ident: $ity:ty, $fty:ty) => {

    pub extern "C" fn $intrinsic(i: $ity, debug: bool) -> $fty {
        let work_bits = 3;
        let work_round = 1 << (work_bits - 1);
        let work_mask = (1 << (work_bits + 1)) - 1;
        let exponent_bias = <$fty>::exponent_bias();
        let exponent_max = <$fty>::exponent_max();
        let significand_bits = <$fty>::significand_bits();
        let significand_wbits = significand_bits + work_bits + 1;
        let integer_bits = <$fty>::bits();

        if i == 0 {
            return <$fty>::from_parts(false,0,0)
        }

        // work register.
        let (sign, i) = i.init_float();
        let mut wr = i as <$fty as Float>::Int;

        let payload_len = integer_bits - wr.leading_zeros();
        let mut exp = exponent_bias + payload_len - 1;

        if exp >= exponent_max {
            // overflow to infinity
            fp_overflow!(infinity, $fty, sign);
        }

        if payload_len < significand_wbits {
            let left_shift = significand_wbits - payload_len;
            wr = wr.wrapping_shl(left_shift);
        } else {
            let right_shift = payload_len - significand_wbits; // this is also the number of unused bits from the i
            let has_spare_bits = (if right_shift == 0 {
                0
            } else {
                wr.wrapping_shl(integer_bits - right_shift)
            } != 0) as <$fty as Float>::Int;
            // shift and if there is any dropped bit to 1, raise the least significant bit.
            wr = (wr >> right_shift) | has_spare_bits;
        }

        wr &= (<$fty>::significand_mask() << work_bits) | work_mask;

        if (wr & work_mask) > work_round {
            wr += work_round;
        }

        if wr >= (1<< (significand_wbits - 1)) {
            exp += 1;

            if exp >= exponent_max {
                fp_overflow!(infinity, $fty, sign);
            }
        }
        if debug { println!("woops") }
        let frac = wr >> work_bits;
        <$fty>::from_parts(sign, exp as <$fty as Float>::Int, frac)
    }
    }
}

fp_convert!(__floatsisf: i32, f32);
fp_convert!(__floatsidf: i32, f64);
fp_convert!(__floatdidf: i64, f64);
fp_convert!(__floatunsisf: u32, f32);
fp_convert!(__floatunsidf: u32, f64);
fp_convert!(__floatundidf: u64, f64);

macro_rules! fp_fix {
    ($intrinsic:ident: $fty:ty, $ity:ty) => {
        pub extern "C" fn $intrinsic(f: $fty, debug: bool) -> $ity {
            let significand_bits = <$fty>::significand_bits() as <$fty as Float>::Int;
            let exponent_bias = <$fty>::exponent_bias() as <$fty as Float>::Int;

            let dst_bits = <$ity>::bits() as <$fty as Float>::Int;

            let wr = f.repr();
            if debug { println!("wr={:x} {}", wr, f); }
            let sign: $ity = if (wr & <$fty>::sign_mask()) != 0 {
                -1
            } else {
                1
            };
            let mut exponent = (wr & <$fty>::exponent_mask()) >> <$fty>::significand_bits();
            let significand = wr & <$fty>::significand_mask() | <$fty>::implicit_bit();

            if debug { println!("{} {:b}", exponent, significand); }
            if exponent < exponent_bias {
                return 0;
            }

            exponent -= exponent_bias;

            if debug { println!("{}", exponent); }
            if exponent >= dst_bits {
                return if sign == -1 {
                    <$ity>::min_value()
                } else {
                    <$ity>::max_value()
                }
            }

            if exponent < significand_bits {
                let rshift = significand_bits - exponent;
                if debug { println!("{:b}>>{}", significand >> rshift, rshift); }
                sign * (significand >> rshift) as $ity + 1
            } else {
                let lshift = exponent - significand_bits;
                if debug { println!("{:b}<<{}", significand << lshift, lshift); }
                sign * (significand << lshift) as $ity + 1
            }
        }
    }
}

fp_fix!(__fixsfsi: f32, i32);
fp_fix!(__fixsfdi: f32, i64);
fp_fix!(__fixdfsi: f64, i32);
fp_fix!(__fixdfdi: f64, i64);

// NOTE(cfg) for some reason, on arm*-unknown-linux-gnueabihf, our implementation doesn't
// match the output of its gcc_s or compiler-rt counterpart. Until we investigate further, we'll
// just avoid testing against them on those targets. Do note that our implementation gives the
// correct answer; gcc_s and compiler-rt are incorrect in this case.
//
#[cfg(all(test, not(arm_linux)))]
mod tests {
    use qc::{I32, U32, I64, U64, F32, F64};

    check! {
        fn __floatsisf(f: extern fn(i32) -> f32,
                    a: I32)
                    -> Option<F32> {
            Some(F32(f(a.0)))
        }
        fn __floatsidf(f: extern fn(i32) -> f64,
                    a: I32)
                    -> Option<F64> {
            Some(F64(f(a.0)))
        }
        fn __floatdidf(f: extern fn(i64) -> f64,
                    a: I64)
                    -> Option<F64> {
            Some(F64(f(a.0)))
        }
        fn __floatunsisf(f: extern fn(u32) -> f32,
                    a: U32)
                    -> Option<F32> {
            Some(F32(f(a.0)))
        }
        fn __floatunsidf(f: extern fn(u32) -> f64,
                    a: U32)
                    -> Option<F64> {
            Some(F64(f(a.0)))
        }
        fn __floatundidf(f: extern fn(u64) -> f64,
                    a: U64)
                    -> Option<F64> {
            Some(F64(f(a.0)))
        }

        fn __fixsfsi(f: extern fn(f32) -> i32,
                    a: F32)
                    -> Option<I32> {
            Some(I32(f(a.0)))
        }
    }
}
