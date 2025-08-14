use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    let data: Vec<i64> = vec! [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let processed: Vec<i64> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = data.clone().into_par_iter().enumerate().map(|(_idx, num)| {
            async move {
num.clone() * num.clone() * num.clone()
            }
        }).collect();
        let __result = future::join_all(__futures).await;
        __result
    };
    let sum: i64 = {
        let mut acc = 0;
        for (_idx, num) in processed.clone().into_iter().enumerate() {
            acc = acc.clone() + num.clone();
        }
        acc
    };
}
