use std::time::Duration;

use futures::Future;

pub enum Timeout<T>{
    Result(T),
    Timedout,
}


pub async fn timeout<T>(fut: impl Future<Output = T>, delay: Duration ) -> Timeout<T> {
    // delay
    todo!()
}