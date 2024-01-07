# nebulae-rs

### Current Status

Only building on x64 at the moment. In rough shape.

I've created a separate [dev jounral](dev_journal.md) so I don't pollute this area. I'm trying to be detailed.

### Project Update

Well, it's that time again.

This time around nebulae has a new component: a rust kernel (unironically named iron).

This is still in the initial bring-up phase. Basically trying to get a working virtual memory system going before anything else. Of course, supporting multiple architectures necessarily means that I need to keep feature parity as I go along.

Still has a very, very long way to go.

Currently Iron only supports exceptions, not external interrupts (and even then, only on x64 & aarch64), and there's no usermode yet.

There's a million things left to do. :(

But there's only one way to eat an elephant :)

Next big endeavor: integrated testing using Cargo and qemu.

### What's Different?

Truth be told, this is a complete rewrite. And the rewrite has occurred in rust. Why? Because I'm a sadist. Seriously though, and truthfully, I'm a better developer than I was 5-10 years ago, and I've grown weary of C. Rust (or something else like it) is the future. No point in writing code for the past.

### crates.io

Even though crates.io offers a ton of libraries, I have deliberately chosen to re-implement most things from scratch. This is done primarily to reduce dependencies in the project. At some point, I will completely remove the x86, x86_64, and aarch64-cpu crates, leaving only the uefi crate and its dependencies. I will keep lazy_static and a few other crates, but the kernel itself will be mostly self-contained.

### [Rust?](https://rust-lang.org/) How Original

I know, right?

I loathe rust. It drives me up the wall. Some of its design choices leave me wanting. Seemingly routine code can take a turn and require superhuman effort to unravel.

But- I can't put it down. I can't stop thinking about it. Every time I think I could write a better language, I realize rust has thought of that paradigm or semantic and has done it as x, y, or z. Listen, there are rough edges. There are things you should be able to express that biff the compiler. Code that works on one architecture suddenly fails for one weird reason on another. But it does all mostly work, and once you get something running, it's usually (mostly) correct.

### Would you change anything about Rust?

Yes, and I will blog about that topic in the near future.

### Motivation
The idea here is straightforward, and the same as it's been for a couple of decades: create a microkernel that is ergonomic for personal use, and fast enough so it doesn't feel sluggish.

### Design Choices
There are a few that are set in stone:

1. The kernel will have a fixed binary interface for both applications and drivers.
2. The system will use PE as its executable format (secondary formats will be supported).
3. The kernel will be organized as a hybrid microkernel (involving some VM trickeration).
4. Graphical interfaces will be the primary focus of the system, although shells will be available.
5. Compatibility with existing systems / software / standards is not a primary driver (i.e. this is not a POSIX kernel).

### Goals

My main goal is to write code and share it with the world. I want people to use & enjoy the things I create.

Computers used to be filled with a very accessible sense of wonder, fun & discovery. You could do all sorts of weird things with your computers. They booted to a REPL.

I have a lot of crazy ideas for where this project will go, but I finally have some direction, and some motivation.

The motto here is "create, and help create"

This is not a profit or status driven project. This code has never faced serious scrutiny nor had to withstand a rigorous code review, and it is overly verbose at times (I prefer this style, as I know the vast majority will be optimized away in a release build), but it's here and it works, and you know what they say ;)

Anyway, hopefully as time progresses this repo will be useful for others. Until that time, feel free to send anything my way from bug reports to PRs.

Happy coding.

n