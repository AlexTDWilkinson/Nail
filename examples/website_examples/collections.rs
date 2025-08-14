use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = vec! [1, 2, 3, 4, 5];
    let doubled: Vec<i64> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = numbers.clone().into_par_iter().enumerate().map(|(_idx, num)| {
            async move {
num.clone() * 2
            }
        }).collect();
        let __result = future::join_all(__futures).await;
        __result
    };
    let evens: Vec<i64> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = numbers.clone().into_par_iter().enumerate().map(|(_idx, num)| async move {
            let condition_result = {
num.clone() % 2 == 0            };
            if condition_result {
                Some(num.clone())
            } else {
                None
            }
        }).collect();
        let __results = future::join_all(__futures).await;
        let __result: Vec<_> = __results.into_iter().filter_map(|x| x).collect();
        __result
    };
    let sum: i64 = {
        let mut acc = 0;
        for (_idx, num) in numbers.clone().into_iter().enumerate() {
            acc = acc.clone() + num.clone();
        }
        acc
    };
}
