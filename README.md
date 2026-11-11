Maintained fork of polyverse/timeout-iterator (deprecated upstream) by Archis Gore.

![Build Status](https://github.com/archisgore/timeout-iterator/workflows/Build%20Status/badge.svg)

# timeout-iterator

`timeout_iterator::TimeoutIterator` is a wrapper over any iterator that adds two additional functions:
* peek_timeout()
* next_timeout()

The canonical use-case is parsing multi-line free-form records (such as tailing a log fime) where it is desirable to
consume the very last line, and peek whether the record continues on the next time, without blocking indefinitely on the peek().

This was built for parsing Kernel logs from `/dev/kmsg` for instance. A kernel log record may look like this:

```text
6,361,518496,-;ahci 0000:00:05.0: AHCI 0001.0300 32 slots 6 ports 6 Gbps 0x1 impl SATA mode
 SUBSYSTEM=pci
 DEVICE=+pci:0000:00:05.0
 ```
The end of such a record is only known when the next line begins a new record. However, if this were the last
record emitted, then it is possible that it never gets flushed/parsed because there is no next record to trigger it.

This is where an iterator with a timeout capability helps to break the deadlock.

## Synchronous Iteration

When feature `sync` is used.

The iterator is obvious and intuitive:

```rust
# #[cfg(feature = "sync")]
# fn main() {
use timeout_iterator::synchronous::TimeoutIterator;

let numbers: Vec<u32> = vec![1, 2, 3, 4, 5];
let mut ti = TimeoutIterator::with_iter(numbers.into_iter()).unwrap();

assert_eq!(ti.next(), Some(1));
// Reaching the end of the input yields `None`, silently.
assert_eq!(ti.by_ref().count(), 4);
assert_eq!(ti.next(), None);
# }
# #[cfg(not(feature = "sync"))]
# fn main() {}
```

This will iterate over all the integers in the Vector.

However, if the underlying iterator is bursty, you can break out of iteration after a certain timeout duration.
This has practical applications when tailing log files - you can wait for a timeout to publish or process
groups of lines of log.

```rust,no_run
# #[cfg(feature = "sync")]
# fn main() {
use timeout_iterator::synchronous::TimeoutIterator;
use timeout_iterator::error::Error;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::time::Duration;

let file = File::open("log.txt").unwrap();
let lines = BufReader::new(file).lines();
let mut ti = TimeoutIterator::with_iter(lines).unwrap();

loop {
   match ti.next_timeout(Duration::from_secs(1)) {
       Ok(Ok(next_line)) => {
           println!("{}", next_line);
       },
       Ok(Err(_)) => {
           // TimeoutIterator succeeded, underlying iterator provided an error
       },
       Err(Error::TimedOut) => {
           // timed out waiting for underlying iterator to provide something
       },
       Err(_) => {
           // other TimeoutIterator error (e.g. the underlying iterator is exhausted)
           break;
       }
   }
}
# }
# #[cfg(not(feature = "sync"))]
# fn main() {}
```

There is a similar `peek_timeout` function to peek the next entry (or timeout doing so), so you
can see if anything is coming down the line without consuming it.


## Asynchronous Iteration

When feature `async` is used.

`TimeoutStream` wraps any `Stream`. Its `peek`, `peek_timeout` and `next_timeout` methods take `Pin<&mut Self>`,
so pin the stream first (for example with `Box::pin`) and call them through `as_mut()`:

```rust
# #[cfg(feature = "async")]
# #[tokio::main(flavor = "current_thread")]
# async fn main() {
use futures::stream::iter;
use timeout_iterator::asynchronous::TimeoutStream;
use tokio_stream::StreamExt;

let numbers: Vec<u32> = vec![1, 2, 3, 4, 5];
let numbers_stream = iter(numbers.into_iter());

let mut ti = Box::pin(TimeoutStream::with_stream(numbers_stream).await.unwrap());
# }
# #[cfg(not(feature = "async"))]
# fn main() {}
```

You can use it like any other Stream, and peek the next value without consuming it:

```rust
# #[cfg(feature = "async")]
# #[tokio::main(flavor = "current_thread")]
# async fn main() {
# use futures::stream::iter;
# use timeout_iterator::asynchronous::TimeoutStream;
# use tokio_stream::StreamExt;
# let mut ti = Box::pin(TimeoutStream::with_stream(iter(vec![1u32, 2, 3, 4, 5])).await.unwrap());
// Can peek many times
assert_eq!(*ti.as_mut().peek().await.unwrap(), 1);
assert_eq!(*ti.as_mut().peek().await.unwrap(), 1);

// And then consume with `next`
assert_eq!(ti.next().await.unwrap(), 1);
assert_eq!(ti.next().await.unwrap(), 2);
assert_eq!(ti.next().await.unwrap(), 3);
assert_eq!(ti.next().await.unwrap(), 4);
assert_eq!(ti.next().await.unwrap(), 5);
assert!(ti.next().await.is_none());
# }
# #[cfg(not(feature = "async"))]
# fn main() {}
```

You can peek or consume with a timeout and catch the `Error::TimedOut` error:

```rust
# #[cfg(feature = "async")]
# #[tokio::main(flavor = "current_thread")]
# async fn main() {
use futures::stream::iter;
use std::time::Duration;
use timeout_iterator::asynchronous::TimeoutStream;
use timeout_iterator::error::Error;
use tokio_stream::StreamExt;

let numbers: Vec<u32> = vec![1, 2, 3, 4, 5];

// Slow down the numbers to stream
let throttled_numbers = iter(numbers.into_iter()).throttle(Duration::from_millis(400));
let mut ti = Box::pin(TimeoutStream::with_stream(Box::pin(throttled_numbers)).await.unwrap());

// First number is always available
assert_eq!(ti.next().await.unwrap(), 1);

// 2nd number will time out well before it is ready
assert!(matches!(
    ti.as_mut().next_timeout(Duration::from_millis(10)).await.unwrap_err(),
    Error::TimedOut
));

// Will consume it if called blocking: the timed out call did not lose the item
assert_eq!(ti.next().await.unwrap(), 2);

// Peek with timeout will... timeout
assert!(matches!(
    ti.as_mut().peek_timeout(Duration::from_millis(10)).await.unwrap_err(),
    Error::TimedOut
));

// a blocking peek will eventually succeed
// we dereference the peek because it's a reference (not move)
assert_eq!(*ti.as_mut().peek().await.unwrap(), 3);

// The number will be consumed
assert_eq!(ti.next().await.unwrap(), 3);

// As proven by the next number
assert_eq!(ti.next().await.unwrap(), 4);
# }
# #[cfg(not(feature = "async"))]
# fn main() {}
```
