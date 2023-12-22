# nebulae-rs

### Dev Journal

[Go back to main readme](readme.md)

2023-12-22 Update:

I think I mostly have paging back on track. There's a few lingering bugs. As soon as that is done I'm switching over to arm.

2023-12-21 Update:

Fixing paging momentarily. The subsystem hasn't been touched through a few rounds of fairy dust sprinkling. It builds but segfaults, so there's some bad juju going on in there.

Physical allocator work is almost done. Virtual memory needs an overhaul, and aa64 hardware-specific support needs to be added.

I think this will be a nice plateau to bring feature parity across architectures.

Once parity is achieved, I will begin work on the executable subsystem.

Then the real fun begins.

2023-12-20 Update:

Both x86 & x64 build & run through their paces. aa64 still a WiP.

Boot has been re-tooled to be much cleaner. I wanted it to be tight.

Allocator work continues, but is hopefully nearing the point I can build some parts of std.

Need to verify all vmem routines.

Need to implement testing asap.

2023-12-18 Update:

Nothing building, huge amounts of code movement and churn. Cleaning up little things to make them ergonomic.

For frame allocation, I settled on a dual red-black tree implementation with with a page info cache. Still a lot to implement.

I went back and cleaned up the initial boot phase to be a bit more polished. I'm still new to rust, so sometimes I do things in a non-ideomatic way, and then I learn a more proper way, so then I go back and refactor.

There's not a lot of code here. There are a great deal of challenges ahead.

Stay tuned.

Initial Update:

As of right now, I've done some tidying up in anticipation of some upgrading.  I am solely focusing on the memory management system at the moment. I hope to have it tied together with some aarch64 updates to bring feature parity between the platforms. I just haven't had enough time to study the developer's manuals in depth. I'm working on it.