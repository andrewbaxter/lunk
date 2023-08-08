This is an event graph processing library.

The expected use case is user interfaces, where user events can set of cascades of changes to the view. Specifically, WASM/web user interfaces (since it's primarily single threaded) but this could also reasonably be used with other single-threaded UI libraries like Gtk.

Most web frameworks include their own event graph processing tools: Sycamore has signals, Dioxus and others have similar tools. This is a standalone tool, for use without a framework (i.e. with Gloo and other a la carte tools).

Compared to other tools, this library focuses on ease of use, flexibility, and composition with common language structures rather than on performance.

- You can represent a full graph, including cycles
- Data has simple, lifetime-less types (`Prim<i32>`, `vec::Vec<i32>`, etc) which makes it simple to pass around and store in structures
- Handles graph modifications during graph processing - new nodes will be processed after their dependencies
- A macro for generating links, but implementing it by hand requires minimal boilerplate

This only handles synchronous graph processing at the moment. Asynchronous events are not handled.

State: MVP

# Usage

## Basic usage

1. Create an `EventContext` `ec`
2. Call `ec.event(|ctx| { })`. All events and program initialization should be done within `ec.event`. `ec.event` waits until the callback ends and then processes the dirty graph. Creating new links triggers event processing, which is why even initialization should be done in this context.
3. Within the event context, create values with `lunk::new_prim()` and `lunk::new_vec()`. Create links with `link!()`.
4. Modify data values using the associated methods to trigger cascading updates. Note that any newly created links will also always be run during the event processing in the event they were created.

## Linking things

The basic way to link things is to implement `LinkCb` on a struct and then instantiate the link with `new_link`.

I'd recommend using the macro though. I'm not a fan of macros, but I think this macro is fairly un-surprising and saves a bit of boilerplate.

The `link!` macro looks kind of like a function signature, but where the arguments are separated into three segments with `;`.

```
let _link = link!((ctx1 = ctx2, output: DTYPE = output1; a=a1, ...; b=b1, ...) {
    let a = a.upgrade()?;
    let b = b.upgrade()?;
    output.set(a.get() + b.get());
})
```

The first argument section has fixed elements:

- `ctx1` is the name of the context binding in the context of the link callback,
  and `ctx2` is the `EventProcessingContext` in the current scope used for setting
  up the link. This is provided within an invocation of `EventContext`'s `event()`.
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

The graph is heterogenous, with data referring to dependent links, and links referring to both input and output data.

Links store a strong reference to their output data. Depending on how you invoke the `link!` macro, link inputs can be kept with either strong or weak references. Data keeps weak references to dependent links.

This means that in general cycles won't lead to memory leaks, but if a link gets dropped accidentally may unexpectedly stop.

I recommend storing links and data scoped to their associated view components, so that when those components are removed the corresponding links and data values also get dropped.

# Design decisions

## Usability, not performance

### It's fast

It's not optimized, but it's not slow. It's written in Rust, and it's computationally simple. I hooked it up to <https://github.com/krausest/js-framework-benchmark> and on my computer creating 10k rows took 850ms (in Rust + FFI) vs Sycamore's 750ms (in Rust + FFI). Most of the time in both was spent in FFI calls (creating elements, inserting elements, modifying element attributes, etc), not in graph processing.

Edit: I ran it again and got 795ms for both. There was a GC pause I saw last time gone this time. I don't actually think this is as fast as Sycamore, it might have been GC luck, but the point stands that it's decently fast as it is.

### Rendering is slow

In the above benchmark, creating 10k rows took 4.5s overall, of which 800ms was rust code, the rest was rendering. The rest was browser rendering and layout.

### Macro optimization not micro optimization

The goal of event graph processing is to avoid expensive updates or recomputations. Graph processing should always be a small part of the total computational time - a few ms more or less doesn't really matter if the large calculations can be avoided.

### Optimizations impede usability

Things like region allocation often require very specific idioms. If your use case doesn't map well to these idioms, you could end up with harder to implement, maintain, or even less performant code.

## Separate links and data values

You typically pass around data so other systems can attach their own listeners. If a value is an output of a graph computation, it needs to include references to all the inputs the computation needs, which in general means you need a new type for each computation. By keeping the data separate from the link, the data types can be simple while the complexity is kept in the links.

This also supports many configurations with only a few functions/macros: value that's manually triggered not computed, a value that's computed from other values, and a computation that doesn't output a value.

## Requiring the user to specify the output type in the macro

The macro lacks the ability to detect the output type where it's needed.

I could have used template magic to infer the type, but this would have made manual (macro-less) link implementations need more boilerplate, so I decided against it. The types are fairly simple so I don't think it's a huge downside.
