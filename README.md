# catty

![a picture of a slingshot](catty.svg)

Send a value synchronously and asynchronously wait for it. Catty is faster, simpler, and more lightweight than
[`futures::oneshot`](https://docs.rs/futures/0.3.5/futures/channel/oneshot/index.html), which is *slightly* more flexible.

## Benchmarks

To run the benchmarks with Criterion, simply do `cargo bench`. On my machine, the results are as follows:

```
create-futures          time:   [105.43 ns 105.52 ns 105.61 ns]
create-catty            time:   [47.774 ns 47.808 ns 47.843 ns]
oneshot-futures         time:   [195.77 ns 195.88 ns 196.01 ns]
oneshot-catty           time:   [134.70 ns 134.80 ns 134.91 ns]
```
