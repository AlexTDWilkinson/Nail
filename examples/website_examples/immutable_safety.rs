use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone, PartialEq)]
    struct User {
        name: String,
        score: i64,
        active: bool,
    }
    let users: Vec<User> = vec! [User { name: "Alice".to_string(),  score: 100,  active: true }, User { name: "Bob".to_string(),  score: 85,  active: false }, User { name: "Charlie".to_string(),  score: 92,  active: true }];
    let active_users: Vec<User> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = users.clone().into_par_iter().enumerate().map(|(_idx, user)| async move {
            let condition_result = {
user.active.clone()            };
            if condition_result {
                Some(user.clone())
            } else {
                None
            }
        }).collect();
        let __results = future::join_all(__futures).await;
        let __result: Vec<_> = __results.into_iter().filter_map(|x| x).collect();
        __result
    };
    let high_scorers: Vec<String> = {
        use rayon::prelude::*;
        use rayon::iter::IntoParallelIterator;
        use futures::future;
        let __futures: Vec<_> = active_users.clone().into_par_iter().enumerate().map(|(_idx, user)| {
            async move {
                let result: String = if user.score.clone() > 90 { std_lib::array::join(vec! [user.name.clone(), " (Elite!)".to_string()], "".to_string()).await } else { user.name.clone() };

result.clone()
            }
        }).collect();
        let __result = future::join_all(__futures).await;
        __result
    };
    let total_score: i64 = {
        let mut sum = 0;
        for (_idx, user) in users.clone().into_iter().enumerate() {
            sum = sum.clone() + user.score.clone();
        }
        sum
    };
    print_macro!("Active users with titles:".to_string());
    {
        for (_idx, name) in high_scorers.clone().into_iter().enumerate() {
            print_macro!(name.clone());
        }
        ()
    }    print_macro!("Total score: ".to_string());
    print_macro!(total_score.clone());
    print_macro!("Original user count: ".to_string());
    print_macro!(std_lib::array::len(users.clone()).await);
}
