Lunk is an event graph processing library.

The expected use case is user interfaces, where user events (clicks, typing) can set off cascades of other changes and computation. Specifically, WASM/web user interfaces (since it's single threaded) but this could also reasonably be used with other single-threaded UI libraries like Gtk.

Most web UI frameworks include their own event graph processing tools: Sycamore has signals, Dioxus and others have similar. This is a standalone tool, for use without a framework (i.e. with Gloo and other a la carte tools).

Compared to other tools, this library focuses on ease of use, flexibility, and composition with common language structures rather than on performance.

- You can represent a full graph, including cycles
- Data has simple, lifetime-less types (`Prim<i32>`, `vec::Vec<i32>`, etc) which makes it simple to pass around and store in structures
- Handles graph modifications during graph processing - new nodes will be processed after their dependencies
- A macro for generating links, but implementing it by hand requires minimal boilerplate
- Animation

This only handles synchronous graph processing at the moment. Asynchronous events are not handled.

Status: MVP

# Usage

## Basic usage

1. Create an `EventGraph` `eg`
2. Call `eg.event(|pc| { })`. All events and program initialization should be done within `eg.event`. `eg.event` waits until the callback ends and then processes the dirty graph. Creating new links triggers event processing, which is why even initialization should be done in this context.
3. Within the event context, create values with `lunk::Prim::new()` and `lunk::Vec::new()`. Create links with `lunk::link!()`.
4. Modify data values using the associated methods to trigger cascading updates. Note that any newly created links will also always be run during the event processing in the event they were created.

An example helps:

```rust
fn graph_stuff() {
    let ec = EventGraph::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = lunk::Prim::new(ctx, 0i32);
        let b = lunk::Vec::new(ctx, 0i32);
        let _link = lunk::link!((
            ctx = ctx,
            output: lunk::Prim<i32> = b;
            a = a.weak();
        ) {
            let a = a.upgrade()?;
            output.set(ctx, a.borrow().get() + 5);
        });
        a.set(ctx, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
}
```

## Linking things

The basic way to link things is to implement `LinkCb` on a struct and then instantiate the link with `new_link`.

There's a macro to automate this: `link!`, which I'd recommend. I'm not a fan of macros, but I think this macro is fairly un-surprising and saves a bit of boilerplate.

The `link!` macro looks kind of like a function signature, but where the arguments are separated into three segments with `;`.

```rust
let _link = link!((ctx1 = ctx2, output: DTYPE = output1; a=a1, ...; b=b1, ...) {
    let a = a.upgrade()?;
    let b = b.upgrade()?;
    output.set(a.get() + b.get());
})
```

The first argument segment has fixed elements:

- `ctx1` is the name of the context binding in the context of the link callback,
  and `ctx2` is the `ProcessingContext` in the current scope used for setting
  up the link. This is provided within an invocation of `EventGraph`'s `event()`.
  The `=` here is inappropriate, `ctx1` and `ctx2` are separate values, I just used
  it to be consistent with the rest of the macro.
- Capture `output1` from the current scope and use it as the output of the link activation function. It will be available with the name `output` in the link callback, where the code can modify it. It must have a type of `DTYPE` -- the macro can't infer this so you need to specify it manually.

  Example: `output: Prim<i32> = counter`.

  You can omit `output: ...` if you don't need a listenable output from the computation. The link will be invoked as usual when its inputs change, but no further graph calculations will happen after this node.

The second segment takes any number of values:

- Capture a graph data value `a1` in the current value and use it as an input to the link callback, available with the name `a`. The callback will be called whenever any of the inputs change.

The third section again takes any number of values:

- Capture any non-input values `b1` which will also be available in the link callback with the name `b`.

The body is a function implementation. It returns an `Option<()>` if you want to abort processing here, for example if a weak reference to an input is invalid. The macro adds `return None` to the end.

## Ownership

Links and data have bidirectional references but these are weak. If a link is dropped it will stop being updated during events.

## Animation

To animate primitive values just create an `Animator` and call `set_ease` on the primitive instead of `set`. `set_ease` requires an easing function - the crate `ezing` looks complete and should be easy to use I think.

```
my_prim.set_ease(animator.borrow_mut(), 44.3, 0.3, ezing::linear_inout);
```

Then regularly call

```
animator.borrow_mut().update(eg, delta_s);
```

to step the animation (`delta_s` is seconds since the last update).

Any value that implements `Mult` `Add` and `Sub` can be eased like this.

You can create your own custom animations by implementing `PrimAnimation` and calling `animator.start(MyPrimAnimation{...})`.

## Troubleshooting

### My callback isn't firing

Possible causes

- The callback link was dropped, or an intermediate link on the way to the final link was dropped.

  Forward references are weak, so each dependent link object needs to be kept around as long as the callback is relevant.

- You set the value but `PartialEq` determined it was equal to the current value

  If the value is the same, no updates will occur. This is to prevent unnecessary work when lots of changes are triggered by eliminating unmodified paths, but if you implemented `PartialEq` imprecisely it can prevent legitimate events from being handled.

# Why flexibility over performance

The main gains from these libraries come from helping you avoid costly work. The more flexible, the more work it'll help you avoid.

For work it doesn't avoid, it's still fast: it's written in Rust, the time spent doing graph processing is miniscule compared to FFI calls, styling, layout, rendering in a web environment.

Despite performance not being a focus, it's actually very fast! I tried integrating it into <https://github.com/krausest/js-framework-benchmark> and got good performance: 746ms vs 796ms for Sycamore (!)

(I don't quite believe this is faster than Sycamore:)

- There may be some randomness in the benchmark, it's a browser after all
- Some of the difference may be due to non-event-graph things, like element creation (direct `rooting` manipulation vs Sycamore's JSX-like system)

# Design decisions

## Separate links and data values

You typically pass around data so other systems can attach their own listeners. If a value is an output of a graph computation, it needs to include references to all the inputs the computation needs, which in general means you need a new type for each computation. By keeping the data separate from the link, the data types can be simple while the complexity is kept in the links.

This also supports many configurations with only a few functions/macros: value that's manually triggered not computed, a value that's computed from other values, and a computation that doesn't output a value.

## Requiring the user to specify the output type in the macro

The macro lacks the ability to detect the output type where it's needed.

I could have used template magic to infer the type, but this would have made manual (macro-less) link implementations need more boilerplate, so I decided against it. The types are fairly simple so I don't think it's a huge downside.

## Animations as a separate structure

In true a la carte philosophy, I figured some people might not want animations and it wasn't hard to make entirely separate.

I think bundling the animator with the processing context shouldn't be too hard.

In case there are other similar extensions, having a solution that allows external extension is important (maybe this won't happen though, then I may go ahead and integrate it).
