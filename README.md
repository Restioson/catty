# catty

![a picture of a slingshot](catty.svg)

Send a value synchronously and asynchronously wait for it. Catty is faster, simpler, and more lightweight than
[`futures::oneshot`](https://docs.rs/futures/0.3.5/futures/channel/oneshot/index.html), which is *slightly* more flexible.

## Example

```rust
let (tx, rx) = catty::oneshot();
tx.send("Hello!");
assert_eq!(rx.await, Ok("Hello!"));
```

## Benchmarks

To run the benchmarks with Criterion, simply do `cargo bench`. On my machine, the results are as follows:

```
create-futures          time:   [70.934 ns 70.979 ns 71.045 ns]
create-catty            time:   [32.549 ns 32.594 ns 32.650 ns]
oneshot-futures         time:   [146.45 ns 146.76 ns 147.09 ns]
oneshot-catty           time:   [98.497 ns 99.065 ns 99.686 ns]
send-futures            time:   [80.163 ns 80.384 ns 80.680 ns]
send-catty              time:   [39.064 ns 39.206 ns 39.354 ns]
```
