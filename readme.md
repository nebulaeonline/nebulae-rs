# nebulae-rs

### Current Status

2023-12-18 Update:

Nothing building, huge amounts of code movement and churn. Cleaning up little things to make them ergonomic.

For frame allocation, I settled on a dual red-black tree implementation with with a page info cache. Still a lot to implement.

I went back and cleaned up the initial boot phase to be a bit more polished. I'm still new to rust, so sometimes I do things in a non-ideomatic way, and then I learn a more proper way, so then I go back and refactor.

There's not a lot of code here. There are a great deal of challenges ahead.

Stay tuned.

Initial Update:

As of right now, I've done some tidying up in anticipation of some upgrading.  I am solely focusing on the memory management system at the moment. I hope to have it tied together with some aarch64 updates to bring feature parity between the platforms. I just haven't had enough time to study the developer's manuals in depth. I'm working on it.

### Project Update

Well, it's that time again.

This time around nebulae has a new component: a rust kernel (unironically named iron).

This is still in the initial bring-up phase. 32/64-bit Intel chips are done, and aarch64 needs its paging module. I intend to port to risc-v as well, but need to write some support code first.

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

I'm glad you asked. Yes I would. I am a big fan of the ideas behind the [Strict Provenance](https://doc.rust-lang.org/std/ptr/index.html#strict-provenance) feature. I believe we should be able to prove a system from top to bottom, but we have to have the language / idioms / tools to make that happen, which unfortunately we don't in bare metal code.

Rust needs a "place" (verb; the action; "to place") command / function / macro / feature. I guess it could be called "sysprovides" or "hardware" or "ISAIDSO". Its purpose would be straightforward, and that would be to establish original provenance of a piece of memory. I want to tell the compiler that "there exists a struct laid out like so at this specific memory address." And I want to do it without transmute or from_raw_parts{_mut}, and I sure as hell don't want to drop to C to do it.

If my hardware spits back a table at memory location 0x5555 when I put 0xabba into register r77 and call the qwijybo instruction, I should be able to tell the compiler about it, and not have to do any super-illegal, out of bounds, instant-UB voodoo to make it work. Frankly, that defeats the whole purpose of a lot of the benefits Rust brings to the table.

PLEASE GIVE ME A METHOD TO ***ESTABLISH*** STRICT PROVENANCE

### Motivation
The idea here is straightforward, and the same as it's been for a couple of decades: create a microkernel that is ergonomic for personal use, and fast enough so it doesn't feel sluggish.

### Design Choices
There are a few that are set in stone:

1. The kernel will have a fixed binary interface for both applications and drivers.
2. The system will use PE as its executable format.
3. The kernel will be organized as a hybrid microkernel.
4. Graphical interfaces will be the primary focus of the system.
5. Compatibility with existing systems / software / standards is not a primary driver.

### Goals

My main goal is to write code and share it with the world. I want people to use & enjoy the things I create.

This is not a profit or status driven project. This code has never faced serious scrutiny nor had to withstand a rigorous code review, and it is overly verbose at times (I prefer this style, as I know the vast majority will be optimized away in a release build), but it's here and it works, and you know what they say ;)

Anyway, hopefully as time progresses this repo will be useful for others. Until that time, feel free to send anything my way from bug reports to PRs.

Happy coding.

n